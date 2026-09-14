//! # Brain MCP
//!
//! Servidor Model Context Protocol (MCP) para Local Brain (SRS §8, §21, §31, §32).
//!
//! Expone herramientas de memoria (`brain_remember`, `brain_recall`, `brain_search`)
//! para agentes de IA (Claude Code, Codex, Antigravity, Cursor) sobre stdio y SSE.

pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_protocol_version_is_valid() {
        assert_eq!(MCP_PROTOCOL_VERSION, "2024-11-05");
    }
}
