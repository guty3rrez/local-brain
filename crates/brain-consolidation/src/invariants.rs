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

#[cfg(test)]
mod tests {
    use brain_domain::model::MemoryId;

    use super::*;
    use crate::model::ConflictType;

    #[test]
    fn test_validate_hypothesis_accepts_tentative_confidence_with_sources() {
        let hypothesis = Hypothesis {
            statement: "Podría existir una relación causal".to_string(),
            domain: None,
            suggested_confidence: TENTATIVE_MAX_CONFIDENCE,
            source_memory_ids: vec![MemoryId::new()],
        };
        assert!(ConsolidationInvariants::validate_hypothesis(&hypothesis).is_ok());
    }

    #[test]
    fn test_validate_hypothesis_rejects_empty_source_memories() {
        let hypothesis = Hypothesis {
            statement: "Hipótesis sin evidencia de origen".to_string(),
            domain: None,
            suggested_confidence: 0.1,
            source_memory_ids: vec![],
        };
        assert!(matches!(
            ConsolidationInvariants::validate_hypothesis(&hypothesis),
            Err(ConsolidationError::EmptySourceMemories)
        ));
    }

    #[test]
    fn test_validate_hypothesis_rejects_confidence_above_tentative_ceiling() {
        let hypothesis = Hypothesis {
            statement: "Hipótesis demasiado segura de sí misma".to_string(),
            domain: None,
            suggested_confidence: TENTATIVE_MAX_CONFIDENCE + 0.01,
            source_memory_ids: vec![MemoryId::new()],
        };
        assert!(matches!(
            ConsolidationInvariants::validate_hypothesis(&hypothesis),
            Err(ConsolidationError::HypothesisConfidenceTooHigh { .. })
        ));
    }

    #[test]
    fn test_validate_cluster_accepts_minimum_size() {
        let cluster = MemoryCluster::new(None, vec![MemoryId::new(), MemoryId::new()]);
        assert!(ConsolidationInvariants::validate_cluster(&cluster).is_ok());
    }

    #[test]
    fn test_validate_cluster_rejects_below_minimum_size() {
        let cluster = MemoryCluster::new(None, vec![MemoryId::new()]);
        assert!(matches!(
            ConsolidationInvariants::validate_cluster(&cluster),
            Err(ConsolidationError::ClusterTooSmall {
                actual: 1,
                minimum: 2
            })
        ));
    }

    #[test]
    fn test_validate_conflict_accepts_distinct_memories_with_reason() {
        let conflict = Contradiction::new(
            MemoryId::new(),
            MemoryId::new(),
            ConflictType::DirectOpposite,
            "Afirman resultados opuestos para el mismo contexto",
        )
        .unwrap();
        assert!(ConsolidationInvariants::validate_conflict(&conflict).is_ok());
    }

    #[test]
    fn test_validate_conflict_rejects_self_contradiction() {
        // `Contradiction::new` ya bloquea el auto-conflicto por su cuenta; este test
        // construye el struct directamente para verificar el invariante de
        // `validate_conflict` en aislamiento, como defensa en profundidad.
        let id = MemoryId::new();
        let conflict = Contradiction {
            id: crate::model::ConflictId::new(),
            source_memory_id: id,
            conflicting_memory_id: id,
            conflict_type: ConflictType::DirectOpposite,
            reason: "razón válida".to_string(),
            suggested_resolution: None,
            status: crate::model::ConflictStatus::Pending,
            resolution_context: None,
            detected_at: chrono::Utc::now(),
            resolved_at: None,
            metadata: serde_json::json!({}),
        };
        assert!(matches!(
            ConsolidationInvariants::validate_conflict(&conflict),
            Err(ConsolidationError::SelfContradiction(_))
        ));
    }

    #[test]
    fn test_validate_conflict_rejects_empty_reason() {
        let conflict = Contradiction::new(
            MemoryId::new(),
            MemoryId::new(),
            ConflictType::ConditionalContradiction,
            "   ",
        )
        .unwrap();
        assert!(matches!(
            ConsolidationInvariants::validate_conflict(&conflict),
            Err(ConsolidationError::RepositoryError(_))
        ));
    }
}
