//! Servidor MCP asíncrono sobre stdio y transporte genérico por streams (SRS §21, §31, §32).

use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tracing::{debug, info};

use brain_application::{
    ExpireSessionUseCase, ForgetUseCase, RecallUseCase, RelateUseCase, RememberUseCase,
    TraverseGraphUseCase,
};
use brain_core::CORE_VERSION;

use crate::protocol::{
    InitializeResult, JsonRpcError, JsonRpcRequest, JsonRpcResponse, ServerCapabilities,
    ServerInfo, ToolsCapability, ToolsListResult, MCP_PROTOCOL_VERSION,
};
use crate::security::McpSecurityPolicy;
use crate::tools::{
    execute_forget, execute_graph, execute_recall, execute_relate, execute_remember,
    execute_search, execute_session_end, list_tools,
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
                    Some(name) => name,
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

                let tool_result = match tool_name {
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
                        execute_session_end(
                            arguments,
                            self.expire_session_uc.clone(),
                            &self.security,
                        )
                        .await
                    }
                    "brain_relate" => {
                        execute_relate(arguments, self.relate_uc.clone(), &self.security).await
                    }
                    "brain_graph" => {
                        execute_graph(arguments, self.traverse_uc.clone(), &self.security).await
                    }
                    _ => {
                        return Some(JsonRpcResponse::error(
                            req.id,
                            JsonRpcError::method_not_found(tool_name),
                        ));
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
}
