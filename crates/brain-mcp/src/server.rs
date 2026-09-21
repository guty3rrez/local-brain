//! Servidor MCP asíncrono sobre stdio y transporte genérico por streams (SRS §21, §31, §32).

use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tracing::{debug, error, info};

use brain_application::{
    ConsolidateUseCase, ExpireSessionUseCase, ExplainUseCase, ForgetUseCase, HybridRetrieveUseCase,
    LearnUseCase, RecallUseCase, ReflectUseCase, RelateUseCase, RememberUseCase,
    TraverseGraphUseCase,
};
use brain_core::CORE_VERSION;

use crate::protocol::{
    InitializeResult, JsonRpcError, JsonRpcRequest, JsonRpcResponse, ServerCapabilities,
    ServerInfo, ToolCallResult, ToolsCapability, ToolsListResult, MCP_PROTOCOL_VERSION,
};
use crate::security::McpSecurityPolicy;
use crate::tools::{
    execute_consolidate, execute_explain, execute_forget, execute_graph, execute_learn,
    execute_recall, execute_relate, execute_remember, execute_retrieve, execute_search,
    execute_session_end, list_tools,
};

/// Servidor MCP de Local Brain.
#[derive(Clone)]
pub struct McpServer {
    remember_uc: Arc<RememberUseCase>,
    recall_uc: Arc<RecallUseCase>,
    forget_uc: Arc<ForgetUseCase>,
    expire_session_uc: Option<Arc<ExpireSessionUseCase>>,
    relate_uc: Option<Arc<RelateUseCase>>,
    traverse_uc: Option<Arc<TraverseGraphUseCase>>,
    learn_uc: Option<Arc<LearnUseCase>>,
    explain_uc: Option<Arc<ExplainUseCase>>,
    reflect_uc: Option<Arc<ReflectUseCase>>,
    consolidate_uc: Option<Arc<ConsolidateUseCase>>,
    retrieve_uc: Option<Arc<HybridRetrieveUseCase>>,
    security: McpSecurityPolicy,
}

impl McpServer {
    /// Crea una nueva instancia del servidor MCP.
    pub fn new(
        remember_uc: Arc<RememberUseCase>,
        recall_uc: Arc<RecallUseCase>,
        forget_uc: Arc<ForgetUseCase>,
        security: McpSecurityPolicy,
    ) -> Self {
        Self {
            remember_uc,
            recall_uc,
            forget_uc,
            expire_session_uc: None,
            relate_uc: None,
            traverse_uc: None,
            learn_uc: None,
            explain_uc: None,
            reflect_uc: None,
            consolidate_uc: None,
            retrieve_uc: None,
            security,
        }
    }

    /// Adjunta el caso de uso para expiración de sesiones (SRS §10.1, §21.4).
    pub fn with_expire_session_uc(mut self, expire_session_uc: Arc<ExpireSessionUseCase>) -> Self {
        self.expire_session_uc = Some(expire_session_uc);
        self
    }

    /// Adjunta los casos de uso para operaciones en el Grafo de Conocimiento (SRS §11, §21.1, F5-03).
    pub fn with_graph(
        mut self,
        relate_uc: Arc<RelateUseCase>,
        traverse_uc: Arc<TraverseGraphUseCase>,
    ) -> Self {
        self.relate_uc = Some(relate_uc);
        self.traverse_uc = Some(traverse_uc);
        self
    }

    /// Adjunta los casos de uso para operaciones de aprendizaje y explicabilidad (SRS §15, §57, §59, §60, F6-03).
    pub fn with_learning(
        mut self,
        learn_uc: Arc<LearnUseCase>,
        explain_uc: Arc<ExplainUseCase>,
    ) -> Self {
        self.learn_uc = Some(learn_uc);
        self.explain_uc = Some(explain_uc);
        self
    }

    /// Adjunta los casos de uso para consolidación, reflexión y resolución de conflictos (SRS §16, §17, §18, F7-03).
    pub fn with_consolidation(
        mut self,
        reflect_uc: Arc<ReflectUseCase>,
        consolidate_uc: Arc<ConsolidateUseCase>,
    ) -> Self {
        self.reflect_uc = Some(reflect_uc);
        self.consolidate_uc = Some(consolidate_uc);
        self
    }

    /// Adjunta el caso de uso de recuperación híbrida avanzada (SRS §13, §14, §56, §58, F8-01).
    pub fn with_retrieve_uc(mut self, retrieve_uc: Arc<HybridRetrieveUseCase>) -> Self {
        self.retrieve_uc = Some(retrieve_uc);
        self
    }

    /// Despacha una tool MCP conocida por nombre. Retorna `None` si `tool_name` no
    /// corresponde a ninguna tool registrada (equivalente a "method not found").
    /// Se ejecuta dentro de una tarea Tokio spawneada por `handle_request` para que
    /// un panic durante la ejecución quede aislado a esa sola llamada.
    async fn dispatch_tool(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Option<ToolCallResult> {
        Some(match tool_name {
            "brain_remember" => {
                execute_remember(arguments, self.remember_uc.clone(), &self.security).await
            }
            "brain_recall" => {
                execute_recall(arguments, self.recall_uc.clone(), &self.security).await
            }
            "brain_search" => {
                execute_search(arguments, self.recall_uc.clone(), &self.security).await
            }
            "brain_forget" => {
                execute_forget(arguments, self.forget_uc.clone(), &self.security).await
            }
            "brain_session_end" => {
                execute_session_end(arguments, self.expire_session_uc.clone(), &self.security).await
            }
            "brain_relate" => {
                execute_relate(arguments, self.relate_uc.clone(), &self.security).await
            }
            "brain_graph" => {
                execute_graph(arguments, self.traverse_uc.clone(), &self.security).await
            }
            "brain_learn" => execute_learn(arguments, self.learn_uc.clone(), &self.security).await,
            "brain_explain" => {
                execute_explain(arguments, self.explain_uc.clone(), &self.security).await
            }
            "brain_consolidate" => {
                execute_consolidate(
                    arguments,
                    self.reflect_uc.clone(),
                    self.consolidate_uc.clone(),
                    &self.security,
                )
                .await
            }
            "brain_retrieve" => {
                execute_retrieve(arguments, self.retrieve_uc.clone(), &self.security).await
            }
            // Solo compilada en tests: permite probar el mecanismo de aislamiento de
            // panics de `handle_request` sin depender de un bug real conocido.
            #[cfg(test)]
            "__test_panic__" => panic!("panic de prueba: aislamiento de panics en tools/call"),
            _ => return None,
        })
    }

    /// Procesa una solicitud JSON-RPC y retorna la respuesta si corresponde.
    /// Para notificaciones (como notifications/initialized), retorna `None`.
    pub async fn handle_request(&self, req: JsonRpcRequest) -> Option<JsonRpcResponse> {
        debug!(method = %req.method, id = ?req.id, "Manejando solicitud MCP");

        match req.method.as_str() {
            "initialize" => {
                let init_result = InitializeResult {
                    protocol_version: MCP_PROTOCOL_VERSION.to_string(),
                    capabilities: ServerCapabilities {
                        tools: ToolsCapability {
                            list_changed: false,
                        },
                    },
                    server_info: ServerInfo {
                        name: "local-brain".to_string(),
                        version: CORE_VERSION.to_string(),
                    },
                };

                let val = serde_json::to_value(init_result).unwrap_or_default();
                Some(JsonRpcResponse::success(req.id, val))
            }
            "notifications/initialized" => {
                info!("Cliente MCP handshake completado (initialized)");
                None
            }
            "ping" => Some(JsonRpcResponse::success(req.id, serde_json::json!({}))),
            "tools/list" => {
                let tools = list_tools();
                let result = ToolsListResult { tools };
                let val = serde_json::to_value(result).unwrap_or_default();
                Some(JsonRpcResponse::success(req.id, val))
            }
            "tools/call" => {
                let params = match req.params {
                    Some(p) => p,
                    None => {
                        return Some(JsonRpcResponse::error(
                            req.id,
                            JsonRpcError::invalid_params("Parámetros ausentes para tools/call"),
                        ));
                    }
                };

                let tool_name = match params.get("name").and_then(|v| v.as_str()) {
                    Some(name) => name.to_string(),
                    None => {
                        return Some(JsonRpcResponse::error(
                            req.id,
                            JsonRpcError::invalid_params(
                                "El campo 'name' es requerido en tools/call",
                            ),
                        ));
                    }
                };

                let arguments = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({}));

                // Cada tool se despacha en su propia tarea Tokio para que un panic aislado
                // dentro de un handler (p. ej. un bug futuro similar al de brain_relate,
                // SRS-tracker 80805145) no se propague hasta `run_stream` y tumbe todo el
                // proceso MCP: `JoinHandle::await` convierte ese panic en un `Err(JoinError)`
                // recuperable en vez de un unwind que atraviesa `main()`.
                let server = self.clone();
                let dispatch_name = tool_name.clone();
                let tool_result = match tokio::spawn(async move {
                    server.dispatch_tool(&dispatch_name, arguments).await
                })
                .await
                {
                    Ok(Some(result)) => result,
                    Ok(None) => {
                        return Some(JsonRpcResponse::error(
                            req.id,
                            JsonRpcError::method_not_found(&tool_name),
                        ));
                    }
                    Err(join_error) => {
                        error!(
                            tool = %tool_name,
                            error = %join_error,
                            "Panic aislado durante la ejecución de una tool MCP; el proceso continúa operativo"
                        );
                        ToolCallResult::error(format!(
                            "Error interno inesperado al ejecutar la herramienta '{tool_name}'. El servidor sigue operativo; puede reintentar la llamada."
                        ))
                    }
                };

                let val = serde_json::to_value(tool_result).unwrap_or_default();
                Some(JsonRpcResponse::success(req.id, val))
            }
            unknown => Some(JsonRpcResponse::error(
                req.id,
                JsonRpcError::method_not_found(unknown),
            )),
        }
    }

    /// Ejecuta el servidor sobre streams asíncronos arbitrarios (AsyncRead / AsyncWrite).
    /// Esta abstracción permite probar el servidor de forma unitaria/integrada y ejecutarlo sobre stdio.
    pub async fn run_stream<R, W>(&self, reader: R, mut writer: W) -> Result<(), std::io::Error>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        let mut lines = BufReader::new(reader).lines();

        while let Some(line) = lines.next_line().await? {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }

            let request: Result<JsonRpcRequest, _> = serde_json::from_str(line_trimmed);
            let response = match request {
                Ok(req) => self.handle_request(req).await,
                Err(e) => Some(JsonRpcResponse::error(
                    None,
                    JsonRpcError::parse_error(format!("Error al deserializar JSON-RPC: {e}")),
                )),
            };

            if let Some(resp) = response {
                let serialized = serde_json::to_string(&resp)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                writer.write_all(serialized.as_bytes()).await?;
                writer.write_all(b"\n").await?;
                writer.flush().await?;
            }
        }

        Ok(())
    }

    /// Ejecuta el servidor MCP sobre stdin / stdout (SRS §21).
    pub async fn run_stdio(&self) -> Result<(), std::io::Error> {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        self.run_stream(stdin, stdout).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brain_domain::ports::InMemoryMemoryRepository;
    use tokio::io::duplex;

    #[tokio::test]
    async fn server_handles_initialize_and_ping() {
        let repo = Arc::new(InMemoryMemoryRepository::new());
        let remember_uc = Arc::new(RememberUseCase::new(repo.clone()));
        let recall_uc = Arc::new(RecallUseCase::new(repo.clone()));
        let forget_uc = Arc::new(ForgetUseCase::new(repo.clone()));
        let server = McpServer::new(
            remember_uc,
            recall_uc,
            forget_uc,
            McpSecurityPolicy::new_default(),
        );

        // 1. Initialize
        let init_req = JsonRpcRequest::new("initialize", Some(serde_json::json!(1)));
        let init_resp = server.handle_request(init_req).await.unwrap();
        assert_eq!(init_resp.id, Some(serde_json::json!(1)));
        assert!(init_resp.result.is_some());
        let res_str = serde_json::to_string(&init_resp.result).unwrap();
        assert!(res_str.contains("local-brain"));

        // 2. Initialized notification (no response)
        let notif = JsonRpcRequest::new("notifications/initialized", None);
        let notif_resp = server.handle_request(notif).await;
        assert!(notif_resp.is_none());

        // 3. Ping
        let ping_req = JsonRpcRequest::new("ping", Some(serde_json::json!(2)));
        let ping_resp = server.handle_request(ping_req).await.unwrap();
        assert_eq!(ping_resp.id, Some(serde_json::json!(2)));
    }

    #[tokio::test]
    async fn server_duplex_stream_communication() {
        let repo = Arc::new(InMemoryMemoryRepository::new());
        let remember_uc = Arc::new(RememberUseCase::new(repo.clone()));
        let recall_uc = Arc::new(RecallUseCase::new(repo.clone()));
        let forget_uc = Arc::new(ForgetUseCase::new(repo.clone()));
        let server = McpServer::new(
            remember_uc,
            recall_uc,
            forget_uc,
            McpSecurityPolicy::new_full(),
        );

        let (client_read, server_write) = duplex(4096);
        let (server_read, mut client_write) = duplex(4096);

        // Lanzar server en background task
        let server_task = tokio::spawn(async move {
            server.run_stream(server_read, server_write).await.unwrap();
        });

        let mut client_lines = BufReader::new(client_read).lines();

        // Enviar tools/list
        let req = r#"{"jsonrpc":"2.0","id":100,"method":"tools/list"}"#;
        client_write.write_all(req.as_bytes()).await.unwrap();
        client_write.write_all(b"\n").await.unwrap();
        client_write.flush().await.unwrap();

        let resp_line = client_lines.next_line().await.unwrap().unwrap();
        let resp: JsonRpcResponse = serde_json::from_str(&resp_line).unwrap();
        assert_eq!(resp.id, Some(serde_json::json!(100)));
        assert!(resp.result.is_some());

        drop(client_write);
        server_task.await.unwrap();
    }

    /// Regresión: un panic dentro de un tool handler (simulado vía la tool de prueba
    /// `__test_panic__`, solo compilada en `cfg(test)`) debe quedar aislado por
    /// `tokio::spawn` + `JoinHandle` en vez de propagarse y tumbar todo el proceso MCP
    /// — mismo patrón de fondo que causó el bug del tracker en `brain_relate`
    /// (80805145), ahora cubierto de forma genérica para cualquier tool futura.
    #[tokio::test]
    async fn server_isolates_panic_inside_tool_handler_and_keeps_running() {
        let repo = Arc::new(InMemoryMemoryRepository::new());
        let remember_uc = Arc::new(RememberUseCase::new(repo.clone()));
        let recall_uc = Arc::new(RecallUseCase::new(repo.clone()));
        let forget_uc = Arc::new(ForgetUseCase::new(repo.clone()));
        let server = McpServer::new(
            remember_uc,
            recall_uc,
            forget_uc,
            McpSecurityPolicy::new_full(),
        );

        let (client_read, server_write) = duplex(4096);
        let (server_read, mut client_write) = duplex(4096);

        let server_task = tokio::spawn(async move {
            server.run_stream(server_read, server_write).await.unwrap();
        });

        let mut client_lines = BufReader::new(client_read).lines();

        // 1. Llamada que panica dentro del handler.
        let panic_req = r#"{"jsonrpc":"2.0","id":200,"method":"tools/call","params":{"name":"__test_panic__","arguments":{}}}"#;
        client_write.write_all(panic_req.as_bytes()).await.unwrap();
        client_write.write_all(b"\n").await.unwrap();
        client_write.flush().await.unwrap();

        let resp_line = client_lines.next_line().await.unwrap().unwrap();
        let resp: JsonRpcResponse = serde_json::from_str(&resp_line).unwrap();
        assert_eq!(resp.id, Some(serde_json::json!(200)));
        // El panic no se convierte en un error de protocolo JSON-RPC: sigue siendo un
        // resultado de tool normal (isError: true), igual que cualquier otra falla de
        // negocio — el stream nunca se cierra.
        let result = resp
            .result
            .expect("debe haber result pese al panic aislado");
        assert_eq!(result["isError"], serde_json::json!(true));

        // 2. El loop del servidor sigue vivo: una segunda llamada normal responde bien.
        let ping_req = r#"{"jsonrpc":"2.0","id":201,"method":"ping"}"#;
        client_write.write_all(ping_req.as_bytes()).await.unwrap();
        client_write.write_all(b"\n").await.unwrap();
        client_write.flush().await.unwrap();

        let resp_line2 = client_lines.next_line().await.unwrap().unwrap();
        let resp2: JsonRpcResponse = serde_json::from_str(&resp_line2).unwrap();
        assert_eq!(resp2.id, Some(serde_json::json!(201)));
        assert!(resp2.result.is_some());

        drop(client_write);
        server_task.await.unwrap();
    }
}
