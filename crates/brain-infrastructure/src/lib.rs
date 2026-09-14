//! # Brain Infrastructure
//!
//! Adaptadores secundarios de infraestructura para Local Brain (SRS §8).
//!
//! Implementa los puertos definidos en `brain-domain::ports`:
//! - Persistencia relacional en PostgreSQL y pgvector (SQLx)
//! - Generación local de embeddings con llama.cpp
//! - Almacenamiento y cachés en disco local

pub const INFRASTRUCTURE_LAYER: &str = "brain-infrastructure";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infrastructure_layer_identifies_correctly() {
        assert_eq!(INFRASTRUCTURE_LAYER, "brain-infrastructure");
    }
}
