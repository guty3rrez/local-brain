//! Runner BDD Cucumber para especificaciones vivas del Servidor MCP (SRS §21, §31, §32).

use cucumber::{given, then, when, World};
use std::sync::Arc;

use brain_application::{
    ForgetUseCase, RecallUseCase, RelateUseCase, RememberUseCase, TraverseGraphUseCase,
};
use brain_domain::ports::InMemoryMemoryRepository;
use brain_graph::InMemoryGraphRepository;
use brain_mcp::protocol::{JsonRpcRequest, JsonRpcResponse};
use brain_mcp::security::McpSecurityPolicy;
use brain_mcp::server::McpServer;

#[derive(Default, World)]
pub struct McpWorld {
    server: Option<McpServer>,
    last_response: Option<JsonRpcResponse>,
    last_recall_text: Option<String>,
}

impl std::fmt::Debug for McpWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpWorld")
            .field("has_server", &self.server.is_some())
            .field("last_response", &self.last_response)
            .field("last_recall_text", &self.last_recall_text)
            .finish()
    }
}

#[given(expr = "un servidor MCP activo con política de seguridad completa")]
async fn given_mcp_server(world: &mut McpWorld) {
    let repo = Arc::new(InMemoryMemoryRepository::new());
    let remember_uc = Arc::new(RememberUseCase::new(repo.clone()));
    let recall_uc = Arc::new(RecallUseCase::new(repo.clone()));
    let forget_uc = Arc::new(ForgetUseCase::new(repo.clone()));

    let graph_repo = Arc::new(InMemoryGraphRepository::new());
    let relate_uc =
        Arc::new(RelateUseCase::new(graph_repo.clone()).with_memory_repository(repo.clone()));
    let traverse_uc = Arc::new(TraverseGraphUseCase::new(graph_repo));

    let server = McpServer::new(
        remember_uc,
        recall_uc,
        forget_uc,
        McpSecurityPolicy::new_full(),
    )
    .with_graph(relate_uc, traverse_uc);
    world.server = Some(server);
}

#[when(expr = "el agente envía un mensaje {string}")]
async fn when_send_message(world: &mut McpWorld, method: String) {
    let server = world.server.as_ref().expect("Servidor MCP no inicializado");
    let req = JsonRpcRequest::new(method, Some(serde_json::json!(1)));
    let resp = server.handle_request(req).await;
    world.last_response = resp;
}

#[then(expr = "el servidor responde con la versión de protocolo {string} y nombre {string}")]
async fn then_init_response_valid(
    world: &mut McpWorld,
    expected_version: String,
    expected_name: String,
) {
    let resp = world.last_response.as_ref().expect("Sin respuesta previa");
    let result = resp
        .result
        .as_ref()
        .expect("Resultado esperado en initialize");
    assert_eq!(result["protocolVersion"], expected_version);
    assert_eq!(result["serverInfo"]["name"], expected_name);
}

#[when(expr = "el agente solicita la lista de herramientas con {string}")]
async fn when_request_tools_list(world: &mut McpWorld, method: String) {
    let server = world.server.as_ref().expect("Servidor MCP no inicializado");
    let req = JsonRpcRequest::new(method, Some(serde_json::json!(2)));
    world.last_response = server.handle_request(req).await;
}

#[then(expr = "la lista incluye las herramientas {string}, {string}, {string} y {string}")]
async fn then_tools_list_contains(
    world: &mut McpWorld,
    t1: String,
    t2: String,
    t3: String,
    t4: String,
) {
    let resp = world.last_response.as_ref().expect("Sin respuesta previa");
    let result = resp.result.as_ref().expect("Resultado esperado");
    let tools = result["tools"].as_array().expect("Array de tools esperado");
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();

    assert!(names.contains(&t1.as_str()));
    assert!(names.contains(&t2.as_str()));
    assert!(names.contains(&t3.as_str()));
    assert!(names.contains(&t4.as_str()));
}

#[when(expr = "el agente registra el recuerdo {string} para el proyecto {string}")]
async fn when_agent_remembers(world: &mut McpWorld, content: String, project: String) {
    let server = world.server.as_ref().expect("Servidor MCP no inicializado");
    let params = serde_json::json!({
        "name": "brain_remember",
        "arguments": {
            "content": content,
            "project": project,
            "importance": 0.8
        }
    });
    let req = JsonRpcRequest::new("tools/call", Some(serde_json::json!(3))).with_params(params);
    world.last_response = server.handle_request(req).await;
}

#[then(expr = "la respuesta confirma que el recuerdo fue almacenado")]
async fn then_remember_confirmed(world: &mut McpWorld) {
    let resp = world.last_response.as_ref().expect("Sin respuesta previa");
    let result = resp.result.as_ref().expect("Resultado esperado");
    assert_eq!(result["isError"], false);
    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("almacenado con éxito"));
}

#[when(expr = "el agente consulta recuerdos del proyecto {string}")]
async fn when_agent_recalls_project(world: &mut McpWorld, project: String) {
    let server = world.server.as_ref().expect("Servidor MCP no inicializado");
    let params = serde_json::json!({
        "name": "brain_recall",
        "arguments": {
            "project": project
        }
    });
    let req = JsonRpcRequest::new("tools/call", Some(serde_json::json!(4))).with_params(params);
    let resp = server
        .handle_request(req)
        .await
        .expect("Respuesta de recall esperada");
    let result = resp.result.expect("Resultado de tool esperado");
    let text = result["content"][0]["text"].as_str().unwrap().to_string();
    world.last_recall_text = Some(text);
}

#[then(
    expr = "los recuerdos retornados contienen delimitadores de seguridad anti-prompt-injection"
)]
async fn then_delimiters_present(world: &mut McpWorld) {
    let text = world
        .last_recall_text
        .as_ref()
        .expect("Sin texto de recall");
    assert!(text.contains("<untrusted_memory"));
    assert!(text.contains("</untrusted_memory>"));
    assert!(text.contains("AVISO DE SEGURIDAD LOCAL BRAIN"));
}

#[when(expr = "el agente registra un recuerdo malicioso con contenido {string}")]
async fn when_agent_records_malicious(world: &mut McpWorld, content: String) {
    let server = world.server.as_ref().expect("Servidor MCP no inicializado");
    let params = serde_json::json!({
        "name": "brain_remember",
        "arguments": {
            "content": content,
            "project": "security-audit"
        }
    });
    let req = JsonRpcRequest::new("tools/call", Some(serde_json::json!(5))).with_params(params);
    world.last_response = server.handle_request(req).await;
}

#[when(expr = "el agente recupera el recuerdo")]
async fn when_agent_recalls_security(world: &mut McpWorld) {
    let server = world.server.as_ref().expect("Servidor MCP no inicializado");
    let params = serde_json::json!({
        "name": "brain_recall",
        "arguments": {
            "project": "security-audit"
        }
    });
    let req = JsonRpcRequest::new("tools/call", Some(serde_json::json!(6))).with_params(params);
    let resp = server
        .handle_request(req)
        .await
        .expect("Respuesta esperada");
    let result = resp.result.expect("Resultado de tool esperado");
    let text = result["content"][0]["text"].as_str().unwrap().to_string();
    world.last_recall_text = Some(text);
}

#[then(
    expr = "el contenido malicioso se encuentra debidamente escapado dentro del tag delimitador"
)]
async fn then_malicious_escaped(world: &mut McpWorld) {
    let text = world
        .last_recall_text
        .as_ref()
        .expect("Sin texto de recall");
    assert!(!text.contains("</untrusted_memory><system>"));
    assert!(text.contains("&lt;/untrusted_memory&gt;<system>"));
}

#[then(expr = "la respuesta incluye el aviso de seguridad de Local Brain")]
async fn then_security_notice(world: &mut McpWorld) {
    let text = world
        .last_recall_text
        .as_ref()
        .expect("Sin texto de recall");
    assert!(text.contains("AVISO DE SEGURIDAD LOCAL BRAIN"));
}

#[when(expr = "el agente intenta eliminar un recuerdo sin el flag confirm")]
async fn when_agent_deletes_without_confirm(world: &mut McpWorld) {
    let server = world.server.as_ref().expect("Servidor MCP no inicializado");
    let random_id = brain_domain::model::MemoryId::new();
    let params = serde_json::json!({
        "name": "brain_forget",
        "arguments": {
            "id": random_id.to_string(),
            "confirm": false
        }
    });
    let req = JsonRpcRequest::new("tools/call", Some(serde_json::json!(7))).with_params(params);
    world.last_response = server.handle_request(req).await;
}

#[then(expr = "la operación es rechazada requiriendo confirmación explícita")]
async fn then_forget_rejected(world: &mut McpWorld) {
    let resp = world.last_response.as_ref().expect("Sin respuesta previa");
    let result = resp.result.as_ref().expect("Resultado de tool esperado");
    assert_eq!(result["isError"], true);
    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("confirm"));
}

#[when(
    expr = "el agente ejecuta la herramienta {string} conectando {string} con {string} mediante {string}"
)]
async fn when_agent_calls_relate(
    world: &mut McpWorld,
    tool_name: String,
    source: String,
    target: String,
    relation: String,
) {
    let server = world.server.as_ref().expect("Servidor MCP no inicializado");
    let req = JsonRpcRequest::new("tools/call", Some(serde_json::json!(10))).with_params(
        serde_json::json!({
            "name": tool_name,
            "arguments": {
                "source": source,
                "target": target,
                "relation": relation,
                "weight": 0.95
            }
        }),
    );
    world.last_response = server.handle_request(req).await;
}

#[then(expr = "la respuesta confirma que la relación fue establecida")]
async fn then_relate_confirmed(world: &mut McpWorld) {
    let resp = world.last_response.as_ref().expect("Sin respuesta previa");
    let result = resp.result.as_ref().expect("Resultado esperado");
    let is_error = result
        .get("isError")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    assert!(!is_error, "Se esperaba resultado exitoso");
    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Relación establecida"));
}

#[when(expr = "el agente explora el grafo para {string} mediante la herramienta {string}")]
async fn when_agent_calls_graph(world: &mut McpWorld, node: String, tool_name: String) {
    let server = world.server.as_ref().expect("Servidor MCP no inicializado");
    let req = JsonRpcRequest::new("tools/call", Some(serde_json::json!(11))).with_params(
        serde_json::json!({
            "name": tool_name,
            "arguments": {
                "node": node,
                "depth": 1
            }
        }),
    );
    world.last_response = server.handle_request(req).await;
}

#[then(expr = "la respuesta contiene el nodo {string} con protección de contexto")]
async fn then_graph_response_protected(world: &mut McpWorld, node_name: String) {
    let resp = world.last_response.as_ref().expect("Sin respuesta previa");
    let result = resp.result.as_ref().expect("Resultado esperado");
    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.contains(&node_name));
    assert!(text.contains("<untrusted_graph_context>"));
    assert!(text.contains("</untrusted_graph_context>"));
}

#[tokio::main]
async fn main() {
    McpWorld::run("tests/features").await;
}
