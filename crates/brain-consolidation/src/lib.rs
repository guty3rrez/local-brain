//! # Brain Consolidation
//!
//! Crate de Dominio Puro para el Motor de Consolidación, Reflexión y Contradicciones (SRS §16, §17, §18, Fase 7).
//!
//! Conforme al SRS (RNF-006) y AGENTS.md, este módulo no posee dependencias de persistencia directa (SQLx/PostgreSQL),
//! red (HTTP/MCP) ni frameworks externos. Modela:
//! - Agrupamiento determinista (Clustering) de experiencias episódicas y operativas (SRS §16, §17).
//! - Reconocimiento de patrones recurrentes y síntesis de hipótesis tentativas (SRS §17).
//! - Detección determinista y asistida de contradicciones lógicas y preferencias en conflicto (`CONFLICT`, SRS §18).
//! - Ciclo de vida y resolución de contradicciones cognitivas.

pub mod clustering;
pub mod contradiction;
pub mod errors;
pub mod in_memory;
pub mod invariants;
pub mod model;
pub mod ports;

pub use clustering::{cluster_memories, cosine_similarity};
pub use contradiction::ContradictionDetector;
pub use errors::ConsolidationError;
pub use in_memory::{InMemoryConflictRepository, MockConsolidationLlm};
pub use invariants::{
    ConsolidationInvariants, DEFAULT_SIMILARITY_THRESHOLD, MIN_CLUSTER_SIZE,
    TENTATIVE_MAX_CONFIDENCE,
};
pub use model::{
    ClusterId, ClusterableMemory, ConflictId, ConflictStatus, ConflictType, Contradiction,
    Hypothesis, MemoryCluster, Pattern, PatternType, ReflectionReport,
};
pub use ports::{ConflictRepository, ConsolidationLlmPort};

#[cfg(test)]
mod tests {
    use super::*;
    use brain_domain::model::MemoryId;
    use chrono::Utc;
    use std::str::FromStr;

    #[test]
    fn test_cosine_similarity_edge_cases() {
        let empty: Vec<f32> = vec![];
        assert_eq!(cosine_similarity(&empty, &empty), 0.0);

        let v1 = vec![1.0, 0.0, 0.0];
        let v2 = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&v1, &v2) - 1.0).abs() < 1e-5);

        let v3 = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&v1, &v3) - 0.0).abs() < 1e-5);

        let v4 = vec![-1.0, 0.0, 0.0];
        assert!((cosine_similarity(&v1, &v4) - (-1.0)).abs() < 1e-5);
    }

    #[test]
    fn test_clustering_groups_similar_vectors() {
        let m1 = ClusterableMemory {
            id: MemoryId::new(),
            content: "Experiencia A".to_string(),
            project: Some("project-x".to_string()),
            memory_type: "episodic".to_string(),
            embedding: Some(vec![1.0, 0.0, 0.0]),
            created_at: Utc::now(),
        };

        let m2 = ClusterableMemory {
            id: MemoryId::new(),
            content: "Experiencia B muy similar".to_string(),
            project: Some("project-x".to_string()),
            memory_type: "episodic".to_string(),
            embedding: Some(vec![0.98, 0.02, 0.0]),
            created_at: Utc::now(),
        };

        let m3 = ClusterableMemory {
            id: MemoryId::new(),
            content: "Experiencia C ortogonal".to_string(),
            project: Some("project-x".to_string()),
            memory_type: "episodic".to_string(),
            embedding: Some(vec![0.0, 1.0, 0.0]),
            created_at: Utc::now(),
        };

        let clusters = cluster_memories(&[m1.clone(), m2.clone(), m3], 0.85, 2)
            .expect("Clustering debe ejecutarse con éxito");

        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].memory_ids.len(), 2);
        assert!(clusters[0].memory_ids.contains(&m1.id));
        assert!(clusters[0].memory_ids.contains(&m2.id));
    }

    #[test]
    fn test_srs_18_supabase_vs_dotnet_contradiction_detection() {
        let mem_a = ClusterableMemory {
            id: MemoryId::new(),
            content: "Supabase es preferido para proyectos pequeños.".to_string(),
            project: Some("web".to_string()),
            memory_type: "semantic".to_string(),
            embedding: None,
            created_at: Utc::now(),
        };

        let mem_b = ClusterableMemory {
            id: MemoryId::new(),
            content: ".NET es preferido para proyectos pequeños.".to_string(),
            project: Some("web".to_string()),
            memory_type: "semantic".to_string(),
            embedding: None,
            created_at: Utc::now(),
        };

        let conflict = ContradictionDetector::check_contradiction(&mem_a, &mem_b)
            .expect("Debe evaluar sin error")
            .expect("Debe detectar contradicción entre Supabase y .NET para proyectos pequeños");

        assert_eq!(
            conflict.conflict_type,
            ConflictType::MutuallyExclusivePreference
        );
        assert_eq!(conflict.status, ConflictStatus::Pending);
        assert!(conflict.reason.contains("proyectos pequeños"));
        assert!(conflict.suggested_resolution.is_some());
    }

    #[test]
    fn test_polarity_inversion_contradiction_detection() {
        let mem_a = ClusterableMemory {
            id: MemoryId::new(),
            content: "El deployment con Docker siempre funciona correctamente en producción."
                .to_string(),
            project: Some("devops".to_string()),
            memory_type: "episodic".to_string(),
            embedding: None,
            created_at: Utc::now(),
        };

        let mem_b = ClusterableMemory {
            id: MemoryId::new(),
            content: "El deployment con Docker nunca funciona correctamente en producción."
                .to_string(),
            project: Some("devops".to_string()),
            memory_type: "episodic".to_string(),
            embedding: None,
            created_at: Utc::now(),
        };

        let conflict = ContradictionDetector::check_contradiction(&mem_a, &mem_b)
            .expect("Debe evaluar sin error")
            .expect("Debe detectar oposición lógica");

        assert_eq!(conflict.conflict_type, ConflictType::DirectOpposite);
    }

    #[test]
    fn test_self_contradiction_is_rejected() {
        let id = MemoryId::new();
        let err =
            Contradiction::new(id, id, ConflictType::DirectOpposite, "Misma memoria").unwrap_err();
        assert_eq!(err, ConsolidationError::SelfContradiction(id.to_string()));
    }

    #[test]
    fn test_hypothesis_confidence_invariant_enforced() {
        let valid = Hypothesis {
            statement: "Hipótesis tentativa".to_string(),
            domain: Some("test".to_string()),
            suggested_confidence: 0.35,
            source_memory_ids: vec![MemoryId::new()],
        };
        assert!(ConsolidationInvariants::validate_hypothesis(&valid).is_ok());

        let invalid = Hypothesis {
            statement: "Hipótesis con demasiada certeza".to_string(),
            domain: Some("test".to_string()),
            suggested_confidence: 0.75,
            source_memory_ids: vec![MemoryId::new()],
        };
        let err = ConsolidationInvariants::validate_hypothesis(&invalid).unwrap_err();
        assert!(matches!(
            err,
            ConsolidationError::HypothesisConfidenceTooHigh { .. }
        ));

        let empty_sources = Hypothesis {
            statement: "Sin fuentes".to_string(),
            domain: None,
            suggested_confidence: 0.30,
            source_memory_ids: vec![],
        };
        assert_eq!(
            ConsolidationInvariants::validate_hypothesis(&empty_sources).unwrap_err(),
            ConsolidationError::EmptySourceMemories
        );
    }

    #[tokio::test]
    async fn test_in_memory_conflict_repository_lifecycle() {
        let repo = InMemoryConflictRepository::new();
        let id1 = MemoryId::new();
        let id2 = MemoryId::new();

        let mut conflict = Contradiction::new(
            id1,
            id2,
            ConflictType::MutuallyExclusivePreference,
            "Conflicto de arquitectura",
        )
        .expect("Conflicto válido");

        repo.save_conflict(&conflict)
            .await
            .expect("Debe guardar conflicto");
        assert_eq!(repo.count(), 1);

        let pair_search = repo.find_by_memory_pair(&id2, &id1).await.unwrap();
        assert!(pair_search.is_some());
        assert_eq!(pair_search.unwrap().id, conflict.id);

        let pending = repo.list_pending_conflicts(None).await.unwrap();
        assert_eq!(pending.len(), 1);

        conflict
            .resolve("Se decidió usar microservicios para alta carga")
            .unwrap();
        repo.update_conflict(&conflict).await.unwrap();

        let resolved_pending = repo.list_pending_conflicts(None).await.unwrap();
        assert_eq!(resolved_pending.len(), 0);

        let fetched = repo
            .find_conflict_by_id(&conflict.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched.status, ConflictStatus::Resolved);
        assert_eq!(
            fetched.resolution_context.as_deref(),
            Some("Se decidió usar microservicios para alta carga")
        );
    }

    #[test]
    fn test_conflict_type_and_status_parsing() {
        for t in [
            ConflictType::DirectOpposite,
            ConflictType::MutuallyExclusivePreference,
            ConflictType::ConditionalContradiction,
        ] {
            let s = t.as_str();
            let parsed = ConflictType::from_str(s).unwrap();
            assert_eq!(t, parsed);
        }

        for st in [
            ConflictStatus::Pending,
            ConflictStatus::Resolved,
            ConflictStatus::Dismissed,
        ] {
            let s = st.as_str();
            let parsed = ConflictStatus::from_str(s).unwrap();
            assert_eq!(st, parsed);
        }
    }
}
