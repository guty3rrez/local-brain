//! Puertos secundarios (Traits) para el motor de consolidación (Arquitectura Hexagonal, SRS §8, §16, §17, §18).

use async_trait::async_trait;

use brain_domain::model::MemoryId;

use crate::errors::ConsolidationError;
use crate::model::{
    ClusterableMemory, ConflictId, Contradiction, Hypothesis, Pattern, ReflectionReport,
};

/// Puerto secundario para síntesis y razonamiento mediante modelos de lenguaje (SRS §16, §17).
#[async_trait]
pub trait ConsolidationLlmPort: Send + Sync {
    /// Genera una hipótesis candidata a partir de un patrón y su conjunto de memorias empíricas.
    async fn synthesize_hypothesis(
        &self,
        pattern: &Pattern,
        memories: &[ClusterableMemory],
    ) -> Result<Hypothesis, ConsolidationError>;

    /// Evalúa semánticamente si dos memorias presentan una contradicción o conflicto cognitivo.
    async fn evaluate_contradiction(
        &self,
        mem_a: &ClusterableMemory,
        mem_b: &ClusterableMemory,
    ) -> Result<Option<Contradiction>, ConsolidationError>;
}

/// Puerto secundario para persistencia y consulta de contradicciones y conflictos de conocimiento (SRS §18).
#[async_trait]
pub trait ConflictRepository: Send + Sync {
    /// Guarda un nuevo conflicto detectado.
    async fn save_conflict(&self, conflict: &Contradiction) -> Result<(), ConsolidationError>;

    /// Recupera un conflicto por su identificador único.
    async fn find_conflict_by_id(
        &self,
        id: &ConflictId,
    ) -> Result<Option<Contradiction>, ConsolidationError>;

    /// Busca si ya existe un conflicto registrado entre dos memorias específicas.
    async fn find_by_memory_pair(
        &self,
        mem_a: &MemoryId,
        mem_b: &MemoryId,
    ) -> Result<Option<Contradiction>, ConsolidationError>;

    /// Lista los conflictos activos en estado 'pending', opcionalmente filtrados por proyecto.
    async fn list_pending_conflicts(
        &self,
        project: Option<&str>,
    ) -> Result<Vec<Contradiction>, ConsolidationError>;

    /// Actualiza el estado o resolución de un conflicto existente.
    async fn update_conflict(&self, conflict: &Contradiction) -> Result<(), ConsolidationError>;

    /// Registra en auditoría una corrida de consolidación o reflexión.
    async fn record_consolidation_run(
        &self,
        report: &ReflectionReport,
    ) -> Result<(), ConsolidationError>;
}
