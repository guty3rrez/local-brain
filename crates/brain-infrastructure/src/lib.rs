//! # Brain Infrastructure
//!
//! Adaptadores secundarios de infraestructura para Local Brain (SRS §8).
//!
//! Implementa los puertos definidos en `brain-domain::ports`, `brain-graph::ports`,
//! `brain-learning::ports` y `brain-consolidation::ports`:
//! - Persistencia relacional en PostgreSQL y pgvector (SQLx)
//! - Generación local de embeddings con llama.cpp
//! - Motor de reflexión y contradicciones con llama.cpp
//! - Almacenamiento y cachés en disco local

pub mod embeddings;
pub mod llm;
pub mod persistence;

pub use embeddings::{LlamaCppConfig, LlamaCppEmbeddingProvider};
pub use llm::{LlamaCppReflectionClient, LlamaCppReflectionConfig};
pub use persistence::{
    PostgresBackupRepository, PostgresConflictRepository, PostgresDoctorDiagnostician,
    PostgresGraphRepository, PostgresLearningRepository, PostgresMemoryRepository,
};

pub const INFRASTRUCTURE_LAYER: &str = "brain-infrastructure";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infrastructure_layer_identifies_correctly() {
        assert_eq!(INFRASTRUCTURE_LAYER, "brain-infrastructure");
    }
}
