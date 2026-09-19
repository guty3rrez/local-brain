//! Puertos de Arquitectura Hexagonal para búsqueda léxica y recuperación híbrida (SRS §8, §13).

use async_trait::async_trait;
use brain_domain::model::{DomainError, MemoryId};

/// Puerto secundario para indexación y búsqueda léxica full-text (FTS) (SRS §13.1, §25).
#[async_trait]
pub trait FullTextSearchRepository: Send + Sync {
    /// Ejecuta una búsqueda textual léxica sobre el contenido de los recuerdos,
    /// retornando una lista ordenada de pares `(MemoryId, score léxico normalizado)`.
    async fn search_fulltext(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<(MemoryId, f32)>, DomainError>;
}
