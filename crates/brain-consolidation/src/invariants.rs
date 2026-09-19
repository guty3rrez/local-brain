//! Invariantes y reglas de negocio cognitivas para consolidación (SRS §16, §17, §18, §59).

use crate::errors::ConsolidationError;
use crate::model::{Contradiction, Hypothesis, MemoryCluster};

/// Confianza máxima permitida para una hipótesis producida por reflexión automática (0.40).
/// Conforme a SRS §17: "La reflexión deberá producir hipótesis, no verdades absolutas."
pub const TENTATIVE_MAX_CONFIDENCE: f32 = 0.40;

/// Cantidad mínima de experiencias requeridas para conformar un cluster válido de consolidación.
pub const MIN_CLUSTER_SIZE: usize = 2;

/// Umbral por defecto de similitud coseno para agrupamiento de experiencias afines (768 dimensiones).
pub const DEFAULT_SIMILARITY_THRESHOLD: f32 = 0.82;

pub struct ConsolidationInvariants;

impl ConsolidationInvariants {
    /// Valida que la hipótesis respete los límites de certidumbre tentativa y procedencia.
    pub fn validate_hypothesis(hypothesis: &Hypothesis) -> Result<(), ConsolidationError> {
        if hypothesis.source_memory_ids.is_empty() {
            return Err(ConsolidationError::EmptySourceMemories);
        }

        if hypothesis.suggested_confidence > TENTATIVE_MAX_CONFIDENCE {
            return Err(ConsolidationError::HypothesisConfidenceTooHigh {
                confidence: hypothesis.suggested_confidence,
                max: TENTATIVE_MAX_CONFIDENCE,
            });
        }

        Ok(())
    }

    /// Valida que un cluster de memorias cumpla con el tamaño mínimo de representatividad empírica.
    pub fn validate_cluster(cluster: &MemoryCluster) -> Result<(), ConsolidationError> {
        if cluster.memory_ids.len() < MIN_CLUSTER_SIZE {
            return Err(ConsolidationError::ClusterTooSmall {
                actual: cluster.memory_ids.len(),
                minimum: MIN_CLUSTER_SIZE,
            });
        }

        Ok(())
    }

    /// Valida las invariantes de un conflicto o contradicción detectada.
    pub fn validate_conflict(conflict: &Contradiction) -> Result<(), ConsolidationError> {
        if conflict.source_memory_id == conflict.conflicting_memory_id {
            return Err(ConsolidationError::SelfContradiction(
                conflict.source_memory_id.to_string(),
            ));
        }

        if conflict.reason.trim().is_empty() {
            return Err(ConsolidationError::RepositoryError(
                "La razón de la contradicción no puede estar vacía".to_string(),
            ));
        }

        Ok(())
    }
}
