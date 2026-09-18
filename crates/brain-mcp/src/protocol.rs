//! Modelado del protocolo Model Context Protocol (MCP) y JSON-RPC 2.0 (SRS §8, §21).

use serde::{Deserialize, Serialize};

/// Versión oficial del protocolo MCP implementada (SRS §21).
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

// Códigos de error estándar JSON-RPC 2.0 y específicos de MCP (SRS §31)
pub const PARSE_ERROR: i32 = -32700;
pub const INVALID_REQUEST: i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS: i32 = -32602;
pub const INTERNAL_ERROR: i32 = -32603;
pub const PERMISSION_DENIED: i32 = -32001;
pub const CONFIRMATION_REQUIRED: i32 = -32002;

/// Solicitud o notificación JSON-RPC 2.0.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

impl JsonRpcRequest {
    pub fn new(method: impl Into<String>, id: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.into(),
            params: None,
        }
    }

    pub fn with_params(mut self, params: serde_json::Value) -> Self {
        self.params = Some(params);
        self
    }
}

/// Error retornado en una respuesta JSON-RPC 2.0.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    pub fn parse_error(msg: impl Into<String>) -> Self {
        Self::new(PARSE_ERROR, msg)
    }

    pub fn invalid_request(msg: impl Into<String>) -> Self {
        Self::new(INVALID_REQUEST, msg)
    }

    pub fn method_not_found(method: &str) -> Self {
        Self::new(METHOD_NOT_FOUND, format!("Método no encontrado: {method}"))
    }

    pub fn invalid_params(msg: impl Into<String>) -> Self {
        Self::new(INVALID_PARAMS, msg)
    }

    pub fn internal_error(msg: impl Into<String>) -> Self {
        Self::new(INTERNAL_ERROR, msg)
    }

    pub fn permission_denied(msg: impl Into<String>) -> Self {
        Self::new(PERMISSION_DENIED, msg)
    }

    pub fn confirmation_required(msg: impl Into<String>) -> Self {
        Self::new(CONFIRMATION_REQUIRED, msg)
    }
}

/// Respuesta JSON-RPC 2.0.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    pub fn success(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<serde_json::Value>, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

/// Información del servidor retornado durante el handshake inicial `initialize`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

/// Capacidades de herramientas del servidor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolsCapability {
    pub list_changed: bool,
}

/// Capacidades expuestas por el servidor en `initialize`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ServerCapabilities {
    pub tools: ToolsCapability,
}

/// Resultado de la negociación inicial `initialize`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub server_info: ServerInfo,
}

/// Definición de una herramienta expuesta mediante `tools/list`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Lista de herramientas retornada en `tools/list`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolsListResult {
    pub tools: Vec<ToolDefinition>,
}

/// Elemento de contenido de texto devuelto por una herramienta.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContentItem {
    #[serde(rename = "type")]
    pub type_: String,
    pub text: String,
}

impl ContentItem {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            type_: "text".to_string(),
            text: text.into(),
        }
    }
}

/// Resultado de la ejecución de una herramienta vía `tools/call`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallResult {
    pub content: Vec<ContentItem>,
    pub is_error: bool,
}

impl ToolCallResult {
    pub fn success(text: impl Into<String>) -> Self {
        Self {
            content: vec![ContentItem::text(text)],
            is_error: false,
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self {
            content: vec![ContentItem::text(text)],
            is_error: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_and_deserialize_json_rpc_request() {
        let req_json = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
        let req: JsonRpcRequest = serde_json::from_str(req_json).unwrap();
        assert_eq!(req.method, "ping");
        assert_eq!(req.id, Some(serde_json::json!(1)));

        let serialized = serde_json::to_string(&req).unwrap();
        assert!(serialized.contains(r#""method":"ping""#));
    }

    #[test]
    fn serialize_success_and_error_responses() {
        let ok_resp = JsonRpcResponse::success(
            Some(serde_json::json!(42)),
            serde_json::json!({"status": "ok"}),
        );
        let ok_str = serde_json::to_string(&ok_resp).unwrap();
        assert!(ok_str.contains(r#""status":"ok""#));
        assert!(!ok_str.contains("error"));

        let err_resp = JsonRpcResponse::error(
            Some(serde_json::json!(42)),
            JsonRpcError::permission_denied("No autorizado"),
        );
        let err_str = serde_json::to_string(&err_resp).unwrap();
        assert!(err_str.contains("-32001"));
        assert!(err_str.contains("No autorizado"));
    }
}
