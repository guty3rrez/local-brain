//! Casos de uso de la Capa de Aplicación para Aprendizaje y Explicabilidad (SRS §15, §57, §59, §60, Fase 6).

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, instrument, warn};

use brain_domain::model::{Memory, MemoryContent, MemoryOrigin, Provenance, SemanticMemoryData};
use brain_domain::ports::MemoryRepository;
use brain_learning::confidence::{ConfidenceBreakdown, ConfidenceCalculator};
use brain_learning::invariants::LearningInvariants;
use brain_learning::model::{
    CandidateId, CandidateKnowledge, Evidence, EvidenceSourceType, LearningStage,
};
use brain_learning::ports::LearningRepository;

use crate::use_cases::ApplicationError;

/// Comando para registrar una observación empírica o proponer un nuevo conocimiento candidato.
#[derive(Debug, Clone)]
pub struct LearnCommand {
    pub statement: String,
    pub initial_evidence: Option<String>,
    pub source_type: Option<EvidenceSourceType>,
    pub domain: Option<String>,
    pub agent: Option<String>,
    pub is_supporting: bool,
    pub human_validated: bool,
    pub is_observation: bool,
}

impl LearnCommand {
    /// Inicializa un nuevo comando para proponer conocimiento candidato o creencia.
    pub fn new_candidate(statement: impl Into<String>) -> Self {
        Self {
            statement: statement.into(),
            initial_evidence: None,
            source_type: None,
            domain: None,
            agent: None,
            is_supporting: true,
            human_validated: false,
            is_observation: false,
        }
    }

    /// Inicializa un nuevo comando para registrar una observación factual puntual.
    pub fn new_observation(statement: impl Into<String>) -> Self {
        Self {
            statement: statement.into(),
            initial_evidence: None,
            source_type: Some(EvidenceSourceType::DirectObservation),
            domain: None,
            agent: None,
            is_supporting: true,
            human_validated: false,
            is_observation: true,
        }
    }

    pub fn with_evidence(
        mut self,
        evidence: impl Into<String>,
        source_type: EvidenceSourceType,
    ) -> Self {
        self.initial_evidence = Some(evidence.into());
        self.source_type = Some(source_type);
        self
    }

    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }

    pub fn with_agent(mut self, agent: impl Into<String>) -> Self {
        self.agent = Some(agent.into());
        self
    }

    pub fn with_human_validated(mut self, validated: bool) -> Self {
        self.human_validated = validated;
        self
    }

    pub fn with_supporting(mut self, supporting: bool) -> Self {
        self.is_supporting = supporting;
        self
    }
}

/// Resultado de la ejecución de un aprendizaje.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnResult {
    pub candidate_id: CandidateId,
    pub statement: String,
    pub stage: LearningStage,
    pub confidence: f32,
    pub evidence_count: usize,
    pub human_validated: bool,
    pub domain: Option<String>,
}

/// Caso de uso para registrar observaciones y transformar experiencias en conocimiento candidato (SRS §15, §59).
pub struct LearnUseCase {
    learning_repo: Arc<dyn LearningRepository>,
    memory_repo: Option<Arc<dyn MemoryRepository>>,
}

impl LearnUseCase {
    pub fn new(learning_repo: Arc<dyn LearningRepository>) -> Self {
        Self {
            learning_repo,
            memory_repo: None,
        }
    }

    pub fn with_memory_repository(mut self, memory_repo: Arc<dyn MemoryRepository>) -> Self {
        self.memory_repo = Some(memory_repo);
        self
    }

    #[instrument(skip(self), fields(statement = %cmd.statement))]
    pub async fn execute(&self, cmd: LearnCommand) -> Result<LearnResult, ApplicationError> {
        if !cmd.is_observation {
            let existing_matches = self
                .learning_repo
                .search_candidates(&cmd.statement, 5)
                .await?;
            if let Some(mut existing) = existing_matches.into_iter().find(|c| {
                c.statement
                    .trim()
                    .eq_ignore_ascii_case(cmd.statement.trim())
            }) {
                if let Some(ev_text) = cmd.initial_evidence {
                    let src = cmd
                        .source_type
                        .unwrap_or(EvidenceSourceType::DirectObservation);
                    let mut ev = Evidence::new(existing.id, src, ev_text, cmd.is_supporting)?;
                    if let Some(a) = cmd.agent {
                        ev = ev.with_agent(a);
                    }
                    existing.add_evidence(ev.clone());
                    self.learning_repo.add_evidence(&existing.id, &ev).await?;
                }
                if cmd.human_validated {
                    existing.human_validated = true;
                }
                LearningInvariants::evaluate_progression(&mut existing)?;
                self.learning_repo.update_candidate(&existing).await?;

                if (existing.stage == LearningStage::Validated
                    || existing.stage == LearningStage::Consolidated)
                    && self.memory_repo.is_some()
                {
                    if let Some(ref mem_repo) = self.memory_repo {
                        let prov = Provenance::new(MemoryOrigin::Observation)
                            .with_context(format!("Candidate ID: {}", existing.id));
                        let content = MemoryContent::new(&existing.statement)?;
                        let evidence_ids: Vec<brain_domain::model::MemoryId> = existing
                            .evidences
                            .iter()
                            .filter_map(|e| e.memory_id)
                            .collect();
                        let mut memory = Memory::new_semantic(content, prov, existing.confidence);
                        memory.project = existing.domain.clone();
                        memory.type_data =
                            brain_domain::model::MemoryTypeData::Semantic(SemanticMemoryData {
                                statement: existing.statement.clone(),
                                evidence_ids,
                                domain_area: existing.domain.clone(),
                                last_validated_at: if existing.human_validated {
                                    Some(Utc::now())
                                } else {
                                    None
                                },
                            });

                        if let Err(e) = mem_repo.save(&memory).await {
                            warn!(error = %e, "No se pudo sincronizar automáticamente la memoria semántica");
                        } else {
                            existing.memory_id = Some(memory.id);
                            let _ = self.learning_repo.update_candidate(&existing).await;
                        }
                    }
                }

                return Ok(LearnResult {
                    candidate_id: existing.id,
                    statement: existing.statement,
                    stage: existing.stage,
                    confidence: existing.confidence.value(),
                    evidence_count: existing.evidences.len(),
                    human_validated: existing.human_validated,
                    domain: existing.domain,
                });
            }
        }

        let mut candidate = if cmd.is_observation {
            CandidateKnowledge::new_observation(cmd.statement, cmd.domain, cmd.agent.clone())?
        } else {
            let initial_ev = if let Some(ev_text) = cmd.initial_evidence {
                let src = cmd
                    .source_type
                    .unwrap_or(EvidenceSourceType::DirectObservation);
                let dummy_id = CandidateId::new();
                let mut ev = Evidence::new(dummy_id, src, ev_text, cmd.is_supporting)?;
                if let Some(a) = cmd.agent {
                    ev = ev.with_agent(a);
                }
                Some(ev)
            } else {
                None
            };
            let mut cand =
                CandidateKnowledge::new_candidate(cmd.statement, cmd.domain, initial_ev)?;
            cand.human_validated = cmd.human_validated;
            LearningInvariants::evaluate_progression(&mut cand)?;
            cand
        };

        self.learning_repo.save_candidate(&candidate).await?;

        // Si el conocimiento ha sido validado o consolidado y tenemos repositorio de memoria,
        // sincronizamos un recuerdo semántico para que esté disponible en consultas semánticas y RAG.
        if (candidate.stage == LearningStage::Validated
            || candidate.stage == LearningStage::Consolidated)
            && self.memory_repo.is_some()
        {
            if let Some(ref mem_repo) = self.memory_repo {
                let prov = Provenance::new(MemoryOrigin::Observation)
                    .with_context(format!("Candidate ID: {}", candidate.id));
                let content = MemoryContent::new(&candidate.statement)?;
                let evidence_ids: Vec<brain_domain::model::MemoryId> = candidate
                    .evidences
                    .iter()
                    .filter_map(|e| e.memory_id)
                    .collect();
                let mut memory = Memory::new_semantic(content, prov, candidate.confidence);
                memory.project = candidate.domain.clone();
                memory.type_data =
                    brain_domain::model::MemoryTypeData::Semantic(SemanticMemoryData {
                        statement: candidate.statement.clone(),
                        evidence_ids,
                        domain_area: candidate.domain.clone(),
                        last_validated_at: if candidate.human_validated {
                            Some(Utc::now())
                        } else {
                            None
                        },
                    });

                if let Err(e) = mem_repo.save(&memory).await {
                    warn!(error = %e, "No se pudo sincronizar automáticamente la memoria semántica");
                } else {
                    candidate.memory_id = Some(memory.id);
                    let _ = self.learning_repo.update_candidate(&candidate).await;
                }
            }
        }

        info!(
            candidate_id = %candidate.id,
            stage = %candidate.stage,
            confidence = candidate.confidence.value(),
            "Aprendizaje registrado exitosamente"
        );

        Ok(LearnResult {
            candidate_id: candidate.id,
            statement: candidate.statement,
            stage: candidate.stage,
            confidence: candidate.confidence.value(),
            evidence_count: candidate.evidences.len(),
            human_validated: candidate.human_validated,
            domain: candidate.domain,
        })
    }
}

/// Comando para agregar una evidencia a un conocimiento candidato existente.
#[derive(Debug, Clone)]
pub struct AddEvidenceCommand {
    pub candidate_id: CandidateId,
    pub content: String,
    pub source_type: EvidenceSourceType,
    pub is_supporting: bool,
    pub agent: Option<String>,
}

impl AddEvidenceCommand {
    pub fn new(
        candidate_id: CandidateId,
        content: impl Into<String>,
        source_type: EvidenceSourceType,
        is_supporting: bool,
    ) -> Self {
        Self {
            candidate_id,
            content: content.into(),
            source_type,
            is_supporting,
            agent: None,
        }
    }

    pub fn with_agent(mut self, agent: impl Into<String>) -> Self {
        self.agent = Some(agent.into());
        self
    }
}

/// Caso de uso para incorporar evidencia empírica a un conocimiento candidato (SRS §15, §60).
pub struct AddEvidenceUseCase {
    learning_repo: Arc<dyn LearningRepository>,
}

impl AddEvidenceUseCase {
    pub fn new(learning_repo: Arc<dyn LearningRepository>) -> Self {
        Self { learning_repo }
    }

    #[instrument(skip(self), fields(candidate_id = %cmd.candidate_id))]
    pub async fn execute(&self, cmd: AddEvidenceCommand) -> Result<LearnResult, ApplicationError> {
        let mut candidate = self
            .learning_repo
            .find_candidate_by_id(&cmd.candidate_id)
            .await?
            .ok_or_else(|| ApplicationError::NotFoundString(cmd.candidate_id.to_string()))?;

        let mut evidence = Evidence::new(
            candidate.id,
            cmd.source_type,
            cmd.content,
            cmd.is_supporting,
        )?;
        if let Some(a) = cmd.agent {
            evidence = evidence.with_agent(a);
        }

        candidate.add_evidence(evidence.clone());
        self.learning_repo
            .add_evidence(&candidate.id, &evidence)
            .await?;

        // Reevalúa progresión con las nuevas evidencias
        LearningInvariants::evaluate_progression(&mut candidate)?;
        self.learning_repo.update_candidate(&candidate).await?;

        Ok(LearnResult {
            candidate_id: candidate.id,
            statement: candidate.statement,
            stage: candidate.stage,
            confidence: candidate.confidence.value(),
            evidence_count: candidate.evidences.len(),
            human_validated: candidate.human_validated,
            domain: candidate.domain,
        })
    }
}

/// Consulta para explicar el fundamento de una creencia o conocimiento (SRS §57, §21.5).
#[derive(Debug, Clone)]
pub struct ExplainQuery {
    pub query: String,
    pub domain: Option<String>,
}

impl ExplainQuery {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            domain: None,
        }
    }

    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }
}

/// Informe de Explicabilidad conforme al SRS §57.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplanationReport {
    pub conclusion: String,
    pub stage: LearningStage,
    pub confidence: f32,
    pub confidence_breakdown: ConfidenceBreakdown,
    pub evidences: Vec<Evidence>,
    pub human_validated: bool,
    pub domain: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub formatted_explanation: String,
}

/// Caso de uso para responder "¿Por qué el sistema cree esto?" (SRS §57, §21.5).
pub struct ExplainUseCase {
    learning_repo: Arc<dyn LearningRepository>,
    memory_repo: Option<Arc<dyn MemoryRepository>>,
}

impl ExplainUseCase {
    pub fn new(learning_repo: Arc<dyn LearningRepository>) -> Self {
        Self {
            learning_repo,
            memory_repo: None,
        }
    }

    pub fn with_memory_repository(mut self, memory_repo: Arc<dyn MemoryRepository>) -> Self {
        self.memory_repo = Some(memory_repo);
        self
    }

    #[instrument(skip(self), fields(query = %query.query))]
    pub async fn execute(
        &self,
        query: ExplainQuery,
    ) -> Result<ExplanationReport, ApplicationError> {
        // 1. Intentar resolver por ID exacto de candidato
        let target_candidate = if let Ok(cand_id) = query.query.parse::<CandidateId>() {
            self.learning_repo.find_candidate_by_id(&cand_id).await?
        } else {
            // 2. Buscar por texto de consulta en el repositorio de aprendizaje
            let search_results = self
                .learning_repo
                .search_candidates(&query.query, 1)
                .await?;
            search_results.into_iter().next()
        };

        if let Some(candidate) = target_candidate {
            let (_, breakdown) = ConfidenceCalculator::calculate(
                &candidate.evidences,
                candidate.human_validated,
                candidate.stage,
                Utc::now(),
            );

            let formatted = Self::format_explanation(&candidate, &breakdown);

            return Ok(ExplanationReport {
                conclusion: candidate.statement,
                stage: candidate.stage,
                confidence: candidate.confidence.value(),
                confidence_breakdown: breakdown,
                evidences: candidate.evidences,
                human_validated: candidate.human_validated,
                domain: candidate.domain,
                created_at: candidate.created_at,
                updated_at: candidate.updated_at,
                formatted_explanation: formatted,
            });
        }

        // 3. Si no se halló en candidatos, verificar en memoria semántica si hay repositorio
        if let Some(ref mem_repo) = self.memory_repo {
            if let Ok(mem_id) = query.query.parse::<brain_domain::model::MemoryId>() {
                if let Some(mem) = mem_repo.find_by_id(&mem_id).await? {
                    let pseudo_cand = CandidateKnowledge {
                        id: CandidateId::from_uuid(*mem.id.as_uuid()),
                        statement: mem.content.text().to_string(),
                        stage: LearningStage::Consolidated,
                        confidence: mem.confidence,
                        domain: mem.project.clone(),
                        human_validated: matches!(mem.source.origin, MemoryOrigin::Manual),
                        memory_id: Some(mem.id),
                        evidences: vec![Evidence::new(
                            CandidateId::from_uuid(*mem.id.as_uuid()),
                            EvidenceSourceType::DirectObservation,
                            format!(
                                "Memoria consolidada registrada con origen {:?}",
                                mem.source.origin
                            ),
                            true,
                        )?],
                        metadata: serde_json::json!({}),
                        created_at: mem.created_at,
                        updated_at: mem.updated_at,
                    };

                    let (_, breakdown) = ConfidenceCalculator::calculate(
                        &pseudo_cand.evidences,
                        pseudo_cand.human_validated,
                        pseudo_cand.stage,
                        Utc::now(),
                    );

                    let formatted = Self::format_explanation(&pseudo_cand, &breakdown);

                    return Ok(ExplanationReport {
                        conclusion: pseudo_cand.statement,
                        stage: pseudo_cand.stage,
                        confidence: pseudo_cand.confidence.value(),
                        confidence_breakdown: breakdown,
                        evidences: pseudo_cand.evidences,
                        human_validated: pseudo_cand.human_validated,
                        domain: pseudo_cand.domain,
                        created_at: pseudo_cand.created_at,
                        updated_at: pseudo_cand.updated_at,
                        formatted_explanation: formatted,
                    });
                }
            }
        }

        Err(ApplicationError::NotFoundString(format!(
            "No se encontró conocimiento candidato ni memoria que explique '{}'",
            query.query
        )))
    }

    /// Formatea la explicación textual estrictamente según la especificación del SRS §57.
    fn format_explanation(
        candidate: &CandidateKnowledge,
        breakdown: &ConfidenceBreakdown,
    ) -> String {
        let mut out = String::new();
        out.push_str("Conclusión:\n");
        out.push_str(&format!("{}\n\n", candidate.statement));

        out.push_str("Estado de Aprendizaje:\n");
        out.push_str(&format!("{}\n\n", candidate.stage.as_str().to_uppercase()));

        out.push_str("Confianza:\n");
        out.push_str(&format!(
            "{:.2} (soporte: {}, contraevidencias: {}, consistencia: {:.0}%)\n\n",
            candidate.confidence.value(),
            breakdown.supporting_count,
            breakdown.contradicting_count,
            breakdown.consistency_factor * 100.0
        ));

        out.push_str("Evidencia:\n");
        if candidate.evidences.is_empty() {
            out.push_str("- Sin evidencias registradas aún.\n");
        } else {
            for ev in &candidate.evidences {
                let sign = if ev.is_supporting {
                    "[+] SOPORTE"
                } else {
                    "[-] REFUTA"
                };
                out.push_str(&format!(
                    "- {} [{}] \"{}\" ({})\n",
                    sign,
                    ev.source_type.as_str(),
                    ev.content,
                    ev.recorded_at.format("%Y-%m-%d")
                ));
            }
        }
        out.push('\n');

        out.push_str("Validación Humana:\n");
        out.push_str(if candidate.human_validated {
            "Sí (Aprobado)\n\n"
        } else {
            "No (Automático/Empírico)\n\n"
        });

        out.push_str("Última revisión:\n");
        out.push_str(&format!("{}\n", candidate.updated_at.format("%Y-%m-%d")));

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brain_learning::in_memory::InMemoryLearningRepository;

    #[tokio::test]
    async fn test_learn_use_case_observation_and_candidate() {
        let repo = Arc::new(InMemoryLearningRepository::new());
        let use_case = LearnUseCase::new(repo.clone());

        // 1. Registrar observación factual
        let obs_cmd = LearnCommand::new_observation("El deployment tardó 4 minutos.")
            .with_domain("devops")
            .with_agent("agent-ci");
        let obs_res = use_case.execute(obs_cmd).await.unwrap();

        assert_eq!(obs_res.stage, LearningStage::Observation);
        assert!(obs_res.confidence <= 0.40);

        // 2. Registrar creencia candidata con evidencia inicial
        let cand_cmd = LearnCommand::new_candidate("Este stack tiene deployments lentos.")
            .with_domain("devops")
            .with_evidence(
                "Medición promedio de 4m excede el SLA de 1m",
                EvidenceSourceType::ToolExecution,
            );
        let cand_res = use_case.execute(cand_cmd).await.unwrap();

        assert_eq!(cand_res.stage, LearningStage::Candidate);
        assert!(cand_res.evidence_count >= 1);
    }

    #[tokio::test]
    async fn test_add_evidence_and_progression_to_validated() {
        let repo = Arc::new(InMemoryLearningRepository::new());
        let learn_uc = LearnUseCase::new(repo.clone());
        let add_ev_uc = AddEvidenceUseCase::new(repo.clone());

        let res = learn_uc
            .execute(
                LearnCommand::new_candidate(".NET es recomendado para este tipo de proyecto")
                    .with_domain("backend"),
            )
            .await
            .unwrap();

        // Agregar 3 evidencias de soporte independientes
        add_ev_uc
            .execute(AddEvidenceCommand::new(
                res.candidate_id,
                "Experience #182: Auth y reglas complejas",
                EvidenceSourceType::DirectObservation,
                true,
            ))
            .await
            .unwrap();
        add_ev_uc
            .execute(AddEvidenceCommand::new(
                res.candidate_id,
                "Experience #201: Manejo robusto de concurrencia",
                EvidenceSourceType::ToolExecution,
                true,
            ))
            .await
            .unwrap();
        let final_res = add_ev_uc
            .execute(AddEvidenceCommand::new(
                res.candidate_id,
                "Decision #43: Compatibilidad empresarial demostrada",
                EvidenceSourceType::ToolExecution,
                true,
            ))
            .await
            .unwrap();

        assert_eq!(final_res.stage, LearningStage::Validated);
        assert!(final_res.confidence >= 0.70);
    }

    #[tokio::test]
    async fn test_explain_use_case_matches_srs_57_format() {
        let repo = Arc::new(InMemoryLearningRepository::new());
        let learn_uc = LearnUseCase::new(repo.clone());
        let explain_uc = ExplainUseCase::new(repo.clone());

        let cmd = LearnCommand::new_candidate(".NET es recomendado para este tipo de proyecto")
            .with_domain("tech-stack")
            .with_evidence(
                "Experience #182: Firebase Auth presentó limitaciones",
                EvidenceSourceType::DirectObservation,
            )
            .with_human_validated(true);
        let res = learn_uc.execute(cmd).await.unwrap();

        let report = explain_uc
            .execute(ExplainQuery::new(res.candidate_id.to_string()))
            .await
            .unwrap();

        assert_eq!(
            report.conclusion,
            ".NET es recomendado para este tipo de proyecto"
        );
        assert!(report.confidence >= 0.85);
        assert!(report.formatted_explanation.contains("Conclusión:"));
        assert!(report.formatted_explanation.contains("Confianza:"));
        assert!(report.formatted_explanation.contains("Evidencia:"));
        assert!(report
            .formatted_explanation
            .contains("Validación Humana:\nSí"));
    }
}
