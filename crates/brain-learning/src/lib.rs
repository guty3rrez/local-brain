//! # Brain Learning
//!
//! Crate de Dominio Puro para el Motor de Aprendizaje y Conocimiento Candidato (SRS §15, §59, §60, Fase 6).
//!
//! Conforme al SRS (RNF-006), este módulo no posee dependencias de persistencia (SQLx/PostgreSQL),
//! red (HTTP/MCP) ni frameworks externos. Modela:
//! - Distinción entre Observación puntual y Creencia generalizada (SRS §59).
//! - Pipeline de maduración: Experience → Observation → Candidate Knowledge → Validation → Consolidated Knowledge (SRS §15).
//! - Cálculo probabilístico y heurístico de confianza con desglose explicativo (SRS §60).
//! - Mitigación estricta de alucinaciones de agentes (SRS §40, §62).

pub mod confidence;
pub mod errors;
pub mod in_memory;
pub mod invariants;
pub mod model;
pub mod ports;

pub use confidence::{ConfidenceBreakdown, ConfidenceCalculator};
pub use errors::LearningError;
pub use in_memory::InMemoryLearningRepository;
pub use invariants::{
    LearningInvariants, MIN_CONFIDENCE_FOR_VALIDATION, MIN_EVIDENCES_FOR_AUTO_VALIDATION,
};
pub use model::{
    CandidateId, CandidateKnowledge, Evidence, EvidenceId, EvidenceSourceType, LearningStage,
    MAX_STATEMENT_BYTES,
};
pub use ports::LearningRepository;

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use chrono::Utc;

    use super::*;

    #[test]
    fn test_observation_creation_and_defaults() {
        let obs = CandidateKnowledge::new_observation(
            "El deployment tardó 4 minutos.",
            Some("stack-a".to_string()),
            Some("agent-1".to_string()),
        )
        .expect("Debe crear observación válida");

        assert_eq!(obs.stage, LearningStage::Observation);
        assert!(obs.confidence.value() <= 0.40);
        assert_eq!(obs.evidences.len(), 1);
        assert_eq!(
            obs.evidences[0].source_type,
            EvidenceSourceType::DirectObservation
        );
        assert!(obs.evidences[0].is_supporting);
    }

    #[test]
    fn test_candidate_creation_with_empty_statement_fails() {
        let err = CandidateKnowledge::new_candidate("   ", None, None).unwrap_err();
        assert_eq!(err, LearningError::EmptyStatement);
    }

    #[test]
    fn test_candidate_creation_statement_too_large_fails() {
        let large = "a".repeat(MAX_STATEMENT_BYTES + 10);
        let err = CandidateKnowledge::new_candidate(large, None, None).unwrap_err();
        assert!(matches!(err, LearningError::StatementTooLarge { .. }));
    }

    #[test]
    fn test_learning_stage_parsing_and_transitions() {
        for stage in [
            LearningStage::Observation,
            LearningStage::Candidate,
            LearningStage::Validated,
            LearningStage::Consolidated,
            LearningStage::Rejected,
        ] {
            let s = stage.as_str();
            let parsed = LearningStage::from_str(s).expect("Debe parsear etapa");
            assert_eq!(stage, parsed);
        }

        // Observation no puede saltar directo a Consolidated
        assert!(!LearningStage::Observation.can_transition_to(LearningStage::Consolidated));
        // Observation puede pasar a Candidate
        assert!(LearningStage::Observation.can_transition_to(LearningStage::Candidate));
        // Candidate puede pasar a Validated
        assert!(LearningStage::Candidate.can_transition_to(LearningStage::Validated));
        // Validated puede pasar a Consolidated
        assert!(LearningStage::Validated.can_transition_to(LearningStage::Consolidated));
    }

    #[test]
    fn test_confidence_monotonicity_with_supporting_evidence() {
        let mut candidate = CandidateKnowledge::new_candidate(
            "PostgreSQL con pgvector ofrece búsqueda vectorial rápida",
            Some("db".to_string()),
            None,
        )
        .unwrap();

        let now = Utc::now();
        let (conf0, _) = ConfidenceCalculator::calculate(
            &candidate.evidences,
            candidate.human_validated,
            candidate.stage,
            now,
        );

        // Añadir 1 evidencia
        let ev1 = Evidence::new(
            candidate.id,
            EvidenceSourceType::ToolExecution,
            "Benchmark p95 < 10ms",
            true,
        )
        .unwrap();
        candidate.add_evidence(ev1);
        let (conf1, _) = ConfidenceCalculator::calculate(
            &candidate.evidences,
            candidate.human_validated,
            candidate.stage,
            now,
        );
        assert!(conf1.value() > conf0.value());

        // Añadir 2da evidencia
        let ev2 = Evidence::new(
            candidate.id,
            EvidenceSourceType::ToolExecution,
            "Test con 1000 vectores exitoso",
            true,
        )
        .unwrap();
        candidate.add_evidence(ev2);
        let (conf2, _) = ConfidenceCalculator::calculate(
            &candidate.evidences,
            candidate.human_validated,
            candidate.stage,
            now,
        );
        assert!(conf2.value() > conf1.value());

        // Añadir 3ra evidencia
        let ev3 = Evidence::new(
            candidate.id,
            EvidenceSourceType::DirectObservation,
            "Integración fluida en CI",
            true,
        )
        .unwrap();
        candidate.add_evidence(ev3);
        let (conf3, _) = ConfidenceCalculator::calculate(
            &candidate.evidences,
            candidate.human_validated,
            candidate.stage,
            now,
        );
        assert!(conf3.value() > conf2.value());
        assert!(conf3.value() >= MIN_CONFIDENCE_FOR_VALIDATION);
    }

    #[test]
    fn test_confidence_penalized_heavily_by_contradiction() {
        let mut candidate = CandidateKnowledge::new_candidate(
            "SQLite es suficiente para alta concurrencia en producción",
            Some("db".to_string()),
            None,
        )
        .unwrap();

        let ev_pos = Evidence::new(
            candidate.id,
            EvidenceSourceType::AgentHypothesis,
            "Fácil de desplegar sin servidor",
            true,
        )
        .unwrap();
        candidate.add_evidence(ev_pos);

        let now = Utc::now();
        let (conf_before, _) = ConfidenceCalculator::calculate(
            &candidate.evidences,
            candidate.human_validated,
            candidate.stage,
            now,
        );

        // Añadir contraevidencia empírica de herramienta
        let ev_neg = Evidence::new(
            candidate.id,
            EvidenceSourceType::ToolExecution,
            "DatabaseLocked error con 50 peticiones simultáneas",
            false, // refuta
        )
        .unwrap();
        candidate.add_evidence(ev_neg);

        let (conf_after, breakdown) = ConfidenceCalculator::calculate(
            &candidate.evidences,
            candidate.human_validated,
            candidate.stage,
            now,
        );

        assert!(
            conf_after.value() < conf_before.value(),
            "La contradicción debe reducir la confianza"
        );
        assert!(breakdown.contradicting_count == 1);
        assert!(breakdown.consistency_factor < 0.60);
    }

    #[test]
    fn test_human_validation_guarantees_high_confidence() {
        let mut candidate = CandidateKnowledge::new_candidate(
            "Usar arquitectura hexagonal en Local Brain",
            Some("architecture".to_string()),
            None,
        )
        .unwrap();

        candidate.human_validated = true;
        let now = Utc::now();
        let (conf, breakdown) = ConfidenceCalculator::calculate(
            &candidate.evidences,
            candidate.human_validated,
            candidate.stage,
            now,
        );

        assert!(conf.value() >= 0.85);
        assert!(breakdown.human_validated);
    }

    #[test]
    fn test_agent_hallucination_mitigation_isolated_claim_does_not_validate() {
        let mut candidate = CandidateKnowledge::new_candidate(
            "Afirmación alucinada no comprobada",
            Some("general".to_string()),
            None,
        )
        .unwrap();

        // Sin evidencias ni validación humana, no debe pasar a Validated
        let changed = LearningInvariants::evaluate_progression(&mut candidate).unwrap();
        assert_eq!(candidate.stage, LearningStage::Candidate);
        assert!(!changed);
        assert!(candidate.confidence.value() < 0.40);
    }

    #[test]
    fn test_progression_from_candidate_to_validated_with_sufficient_evidence() {
        let mut candidate = CandidateKnowledge::new_candidate(
            ".NET es preferido para backends empresariales complejos",
            Some("tech-stack".to_string()),
            None,
        )
        .unwrap();

        // Añadir 3 evidencias de soporte independientes
        candidate.add_evidence(
            Evidence::new(
                candidate.id,
                EvidenceSourceType::ToolExecution,
                "EF Core soporta migraciones robustas",
                true,
            )
            .unwrap(),
        );
        candidate.add_evidence(
            Evidence::new(
                candidate.id,
                EvidenceSourceType::DirectObservation,
                "Rendimiento tipado superior en endpoints",
                true,
            )
            .unwrap(),
        );
        candidate.add_evidence(
            Evidence::new(
                candidate.id,
                EvidenceSourceType::ToolExecution,
                "MediatR facilita Clean Architecture desacoplada",
                true,
            )
            .unwrap(),
        );

        let changed = LearningInvariants::evaluate_progression(&mut candidate).unwrap();
        assert!(changed);
        assert_eq!(candidate.stage, LearningStage::Validated);
        assert!(candidate.confidence.value() >= 0.70);
    }

    #[tokio::test]
    async fn test_in_memory_learning_repository_lifecycle() {
        let repo = InMemoryLearningRepository::new();
        let candidate = CandidateKnowledge::new_candidate(
            "Rust garantiza memory-safety sin garbage collector",
            Some("rust".to_string()),
            None,
        )
        .unwrap();

        repo.save_candidate(&candidate).await.unwrap();
        assert_eq!(repo.candidate_count(), 1);

        let found = repo
            .find_candidate_by_id(&candidate.id)
            .await
            .unwrap()
            .expect("Debe encontrar candidato");
        assert_eq!(found.statement, candidate.statement);

        // Añadir evidencia
        let ev = Evidence::new(
            candidate.id,
            EvidenceSourceType::ToolExecution,
            "Compilador borrow-checker previene data races",
            true,
        )
        .unwrap();
        repo.add_evidence(&candidate.id, &ev).await.unwrap();

        let evs = repo.get_evidences(&candidate.id).await.unwrap();
        assert_eq!(evs.len(), 1);

        // Búsqueda
        let search_results = repo.search_candidates("memory-safety", 10).await.unwrap();
        assert_eq!(search_results.len(), 1);

        let by_domain = repo.find_candidates_by_domain("rust", 10).await.unwrap();
        assert_eq!(by_domain.len(), 1);

        // Eliminación
        repo.delete_candidate(&candidate.id).await.unwrap();
        assert_eq!(repo.candidate_count(), 0);
    }

    #[test]
    fn test_confidence_bounds_and_zero_evidence() {
        let (conf, breakdown) =
            ConfidenceCalculator::calculate(&[], false, LearningStage::Candidate, Utc::now());

        assert!(conf.value() >= 0.0 && conf.value() <= 1.0);
        assert_eq!(breakdown.supporting_count, 0);
        assert_eq!(breakdown.contradicting_count, 0);
        assert_eq!(breakdown.consistency_factor, 1.0);
        assert_eq!(breakdown.volume_factor, 0.0);
    }

    #[test]
    fn test_overwhelming_contradictions_demotes_to_rejected() {
        let mut candidate = CandidateKnowledge::new_candidate(
            "Cualquier librería puede usarse sin considerar licencia",
            Some("legal".to_string()),
            None,
        )
        .unwrap();

        // 1 débil hipótesis a favor
        candidate.add_evidence(
            Evidence::new(
                candidate.id,
                EvidenceSourceType::AgentHypothesis,
                "Parece funcionar en local",
                true,
            )
            .unwrap(),
        );

        // 2 fuertes evidencias en contra
        candidate.add_evidence(
            Evidence::new(
                candidate.id,
                EvidenceSourceType::ToolExecution,
                "cargo-deny bloqueó licencia GPL no compatible",
                false,
            )
            .unwrap(),
        );
        candidate.add_evidence(
            Evidence::new(
                candidate.id,
                EvidenceSourceType::Human,
                "Auditoría legal exige compatibilidad estricta AGPLv3",
                false,
            )
            .unwrap(),
        );

        let changed = LearningInvariants::evaluate_progression(&mut candidate).unwrap();
        assert!(changed);
        assert_eq!(candidate.stage, LearningStage::Rejected);
        assert!(candidate.confidence.value() <= 0.20);
    }

    #[test]
    fn test_evidence_weight_custom_and_out_of_range() {
        let id = CandidateId::new();
        let ev = Evidence::new(id, EvidenceSourceType::DirectObservation, "Texto", true).unwrap();
        assert!(ev.clone().with_weight(0.75).is_ok());
        assert!(ev.clone().with_weight(-0.01).is_err());
        assert!(ev.with_weight(1.01).is_err());
    }

    #[test]
    fn test_source_type_parsing_and_defaults() {
        assert_eq!(
            EvidenceSourceType::from_str("human").unwrap(),
            EvidenceSourceType::Human
        );
        assert_eq!(
            EvidenceSourceType::from_str("tool").unwrap(),
            EvidenceSourceType::ToolExecution
        );
        assert_eq!(
            EvidenceSourceType::from_str("observation").unwrap(),
            EvidenceSourceType::DirectObservation
        );
        assert_eq!(
            EvidenceSourceType::from_str("hypothesis").unwrap(),
            EvidenceSourceType::AgentHypothesis
        );
        assert!(EvidenceSourceType::from_str("invalido").is_err());
    }
}
