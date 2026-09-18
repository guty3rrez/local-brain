//! # Brain MCP
//!
//! Servidor Model Context Protocol (MCP) para Local Brain (SRS §8, §21, §31, §32).
//!
//! Expone herramientas de memoria (`brain_remember`, `brain_recall`, `brain_search`)
//! para agentes de IA (Claude Code, Codex, Antigravity, Cursor) sobre stdio y SSE.

pub mod protocol;
pub mod security;
pub mod server;
pub mod tools;

pub use protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, MCP_PROTOCOL_VERSION};
pub use security::{McpPermission, McpSecurityPolicy};
pub use server::McpServer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_protocol_version_is_valid() {
        assert_eq!(MCP_PROTOCOL_VERSION, "2024-11-05");
    }
}
