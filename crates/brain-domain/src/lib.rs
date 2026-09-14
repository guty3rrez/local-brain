//! # Brain Domain
//!
//! Capa de dominio puro de Local Brain.
//!
//! Conforme al SRS (RNF-006), este módulo no posee dependencias de
//! persistencia (SQLx/PostgreSQL), red (HTTP/MCP), GPU ni frameworks externos.
//! Su lógica puede y debe compilarse y validarse en milisegundos.

pub mod model;
pub mod ports;

#[cfg(test)]
mod tests {
    use crate::model::MemoryType;

    #[test]
    fn domain_layer_is_pure_and_isolated() {
        assert_ne!(MemoryType::Working, MemoryType::Episodic);
    }
}
