//! Casos de uso de la Capa de Aplicación para Consolidación, Reflexión y Contradicciones (SRS §16, §17, §18, Fase 7).

use std::collections::HashSet;
use std::sync::Arc;

use chrono::Utc;
use tracing::{info, instrument};
use uuid::Uuid;

use brain_consolidation::clustering::cluster_memories;
use brain_consolidation::invariants::{
    ConsolidationInvariants, DEFAULT_SIMILARITY_THRESHOLD, MIN_CLUSTER_SIZE,
};
use brain_consolidation::model::{
    ClusterableMemory, ConflictId, Contradiction, Pattern, PatternType, ReflectionReport,
};
use brain_consolidation::ports::{ConflictRepository, ConsolidationLlmPort};
use brain_domain::model::{Confidence, MemoryId, MemoryStatus, MemoryType};
use brain_domain::ports::MemoryRepository;
use brain_graph::model::{GraphEdge, RelationType};
use brain_graph::ports::GraphRepository;
use brain_learning::model::{CandidateKnowledge, Evidence, EvidenceSourceType};
use brain_learning::ports::LearningRepository;

use crate::use_cases::ApplicationError;

/// Comando para ejecutar el proceso de reflexión asíncrona sobre experiencias recientes (SRS §17).
#[derive(Debug, Clone)]
pub struct ReflectCommand {
    pub project: Option<String>,
    pub limit: usize,
    pub similarity_threshold: f32,
    pub dry_run: bool,
}

impl ReflectCommand {
    pub fn new() -> Self {
        Self {
            project: None,
            limit: 50,
            similarity_threshold: DEFAULT_SIMILARITY_THRESHOLD,
            dry_run: false,
        }
    }

    pub fn with_project(mut self, project: impl Into<String>) -> Self {
        self.project = Some(project.into());
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.similarity_threshold = threshold;
        self
    }

    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }
}

impl Default for ReflectCommand {
    fn default() -> Self {
        Self::new()
    }
}

/// Caso de uso: Motor de Reflexión Asíncrono (SRS §16, §17, §18).
pub struct ReflectUseCase {
    memory_repo: Arc<dyn MemoryRepository>,
    conflict_repo: Arc<dyn ConflictRepository>,
    llm_client: Arc<dyn ConsolidationLlmPort>,
    learning_repo: Option<Arc<dyn LearningRepository>>,
    graph_repo: Option<Arc<dyn GraphRepository>>,
}

impl ReflectUseCase {
    pub fn new(
        memory_repo: Arc<dyn MemoryRepository>,
        conflict_repo: Arc<dyn ConflictRepository>,
        llm_client: Arc<dyn ConsolidationLlmPort>,
    ) -> Self {
        Self {
            memory_repo,
            conflict_repo,
            llm_client,
            learning_repo: None,
            graph_repo: None,
        }
    }

    pub fn with_learning_repository(
        mut self,
        learning_repo: Option<Arc<dyn LearningRepository>>,
    ) -> Self {
        self.learning_repo = learning_repo;
        self
    }

    pub fn with_graph_repository(mut self, graph_repo: Option<Arc<dyn GraphRepository>>) -> Self {
        self.graph_repo = graph_repo;
        self
    }

    #[instrument(skip(self), fields(project = ?cmd.project, limit = cmd.limit))]
    pub async fn execute(&self, cmd: ReflectCommand) -> Result<ReflectionReport, ApplicationError> {
        info!("Iniciando ciclo de reflexión cognitiva en Local Brain");

        // 1. Recuperar experiencias recientes
        let memories = if let Some(ref proj) = cmd.project {
            self.memory_repo.find_by_project(proj, cmd.limit).await?
        } else {
            self.memory_repo
                .find_by_type(MemoryType::Episodic, cmd.limit)
                .await?
        };

        // Filtrar memorias activas aptas para reflexión (excluir soft-deleted y ya en conflicto)
        let active_memories: Vec<_> = memories
            .into_iter()
            .filter(|m| m.status == MemoryStatus::Active)
            .collect();

        let memories_analyzed = active_memories.len();

        let clusterable: Vec<ClusterableMemory> = active_memories
            .iter()
            .map(|m| ClusterableMemory {
                id: m.id,
                content: m.content.text().to_string(),
                project: m.project.clone(),
                memory_type: format!("{:?}", m.memory_type),
                embedding: None,
                created_at: m.created_at,
            })
            .collect();

        // 2. Detección de contradicciones pairwise (SRS §18)
        let mut conflicts_detected = Vec::new();
        let mut conflicting_ids: HashSet<MemoryId> = HashSet::new();

        for i in 0..clusterable.len() {
            for j in (i + 1)..clusterable.len() {
                let mem_a = &clusterable[i];
                let mem_b = &clusterable[j];

                if let Some(conflict) = self
                    .llm_client
                    .evaluate_contradiction(mem_a, mem_b)
                    .await
                    .map_err(|e| {
                    ApplicationError::Domain(brain_domain::model::DomainError::RepositoryError(
                        e.to_string(),
                    ))
                })? {
                    info!(
                        conflict_id = %conflict.id,
                        reason = %conflict.reason,
                        "Contradicción detectada durante reflexión"
                    );

                    conflicting_ids.insert(mem_a.id);
                    conflicting_ids.insert(mem_b.id);

                    if !cmd.dry_run {
                        // Persistir conflicto
                        self.conflict_repo
                            .save_conflict(&conflict)
                            .await
                            .map_err(|e| {
                                ApplicationError::Domain(
                                    brain_domain::model::DomainError::RepositoryError(
                                        e.to_string(),
                                    ),
                                )
                            })?;

                        // Actualizar estado de recuerdos a CONFLICT en persistencia
                        if let Some(mut a) = self.memory_repo.find_by_id(&mem_a.id).await? {
                            a.status = MemoryStatus::Conflict;
                            let _ = self.memory_repo.update(&a).await;
                        }
                        if let Some(mut b) = self.memory_repo.find_by_id(&mem_b.id).await? {
                            b.status = MemoryStatus::Conflict;
                            let _ = self.memory_repo.update(&b).await;
                        }

                        // Vincular arista CONTRADICTS en el grafo si está disponible
                        if let Some(ref graph) = self.graph_repo {
                            if let (Ok(node_a), Ok(node_b)) = (
                                graph.ensure_memory_node(&mem_a.id, &mem_a.content).await,
                                graph.ensure_memory_node(&mem_b.id, &mem_b.content).await,
                            ) {
                                let _ = graph
                                    .save_edge(
                                        &GraphEdge::new(
                                            node_a.id,
                                            node_b.id,
                                            RelationType::Contradicts,
                                            1.0,
                                        )
                                        .map_err(|e| {
                                            ApplicationError::Domain(
                                                brain_domain::model::DomainError::RepositoryError(
                                                    e.to_string(),
                                                ),
                                            )
                                        })?,
                                    )
                                    .await;
                            }
                        }
                    }

                    conflicts_detected.push(conflict);
                }
            }
        }

        // 3. Filtrar memorias no conflictivas para clustering (SRS §16, §17)
        let non_conflicting: Vec<ClusterableMemory> = clusterable
            .into_iter()
            .filter(|m| !conflicting_ids.contains(&m.id))
            .collect();

        let clusters =
            cluster_memories(&non_conflicting, cmd.similarity_threshold, MIN_CLUSTER_SIZE)
                .map_err(|e| {
                    ApplicationError::Domain(brain_domain::model::DomainError::RepositoryError(
                        e.to_string(),
                    ))
                })?;

        let clusters_formed = clusters.len();
        let mut patterns_detected = Vec::new();
        let mut hypotheses = Vec::new();
        let mut created_candidates = Vec::new();

        // 4. Identificar patrones y sintetizar hipótesis para cada cluster (SRS §17)
        for cluster in &clusters {
            let cluster_memories: Vec<ClusterableMemory> = non_conflicting
                .iter()
                .filter(|m| cluster.memory_ids.contains(&m.id))
                .cloned()
                .collect();

            let sample_desc = cluster_memories
                .first()
                .map(|m| m.content.clone())
                .unwrap_or_else(|| "Experiencias agrupadas".to_string());

            let pattern = Pattern {
                pattern_type: PatternType::CorrelatedObservation,
                description: format!("Regularidad en cluster: {sample_desc}"),
                frequency: cluster.memory_ids.len(),
                supporting_memory_ids: cluster.memory_ids.clone(),
            };

            let hyp = self
                .llm_client
                .synthesize_hypothesis(&pattern, &cluster_memories)
                .await
                .map_err(|e| {
                    ApplicationError::Domain(brain_domain::model::DomainError::RepositoryError(
                        e.to_string(),
                    ))
                })?;

            ConsolidationInvariants::validate_hypothesis(&hyp).map_err(|e| {
                ApplicationError::Domain(brain_domain::model::DomainError::RepositoryError(
                    e.to_string(),
                ))
            })?;

            if !cmd.dry_run {
                if let Some(ref learning) = self.learning_repo {
                    let mut candidate =
                        CandidateKnowledge::new_candidate(&hyp.statement, hyp.domain.clone(), None)
                            .map_err(|e| {
                                ApplicationError::Domain(
                                    brain_domain::model::DomainError::RepositoryError(
                                        e.to_string(),
                                    ),
                                )
                            })?;

                    candidate.confidence = Confidence::new(hyp.suggested_confidence)?;

                    for src_id in &hyp.source_memory_ids {
                        let ev = Evidence::new(
                            candidate.id,
                            EvidenceSourceType::DirectObservation,
                            format!("Experiencia origen #{}", src_id),
                            true,
                        )
                        .map_err(|e| {
                            ApplicationError::Domain(
                                brain_domain::model::DomainError::RepositoryError(e.to_string()),
                            )
                        })?
                        .with_memory_id(*src_id);

                        candidate.add_evidence(ev);
                    }

                    learning.save_candidate(&candidate).await.map_err(|e| {
                        ApplicationError::Domain(brain_domain::model::DomainError::RepositoryError(
                            e.to_string(),
                        ))
                    })?;

                    created_candidates.push(candidate.id);
                }
            }

            hypotheses.push(hyp);
            patterns_detected.push(pattern);
        }

        let summary = format!(
            "Reflexión completada: {} memorias analizadas, {} clusters formados, {} hipótesis generadas, {} contradicciones detectadas.",
            memories_analyzed, clusters_formed, hypotheses.len(), conflicts_detected.len()
        );

        let report = ReflectionReport {
            run_id: Uuid::now_v7(),
            project: cmd.project,
            memories_analyzed,
            clusters_formed,
            patterns_detected,
            hypotheses,
            conflicts_detected,
            created_candidates,
            summary,
            executed_at: Utc::now(),
        };

        if !cmd.dry_run {
            let _ = self.conflict_repo.record_consolidation_run(&report).await;
        }

        info!(
            clusters = report.clusters_formed,
            hypotheses = report.hypotheses.len(),
            conflicts = report.conflicts_detected.len(),
            "Reflexión cognitiva finalizada exitosamente"
        );

        Ok(report)
    }
}

/// Comando para consolidar un conjunto explícito de memorias (SRS §16).
#[derive(Debug, Clone)]
pub struct ConsolidateCommand {
    pub memory_ids: Vec<MemoryId>,
    pub project: Option<String>,
    pub dry_run: bool,
}

/// Caso de uso: Consolidación de un grupo explícito de experiencias (SRS §16).
pub struct ConsolidateUseCase {
    memory_repo: Arc<dyn MemoryRepository>,
    conflict_repo: Arc<dyn ConflictRepository>,
    llm_client: Arc<dyn ConsolidationLlmPort>,
    learning_repo: Option<Arc<dyn LearningRepository>>,
}

impl ConsolidateUseCase {
    pub fn new(
        memory_repo: Arc<dyn MemoryRepository>,
        conflict_repo: Arc<dyn ConflictRepository>,
        llm_client: Arc<dyn ConsolidationLlmPort>,
    ) -> Self {
        Self {
            memory_repo,
            conflict_repo,
            llm_client,
            learning_repo: None,
        }
    }

    pub fn with_learning_repository(
        mut self,
        learning_repo: Option<Arc<dyn LearningRepository>>,
    ) -> Self {
        self.learning_repo = learning_repo;
        self
    }

    pub async fn execute(
        &self,
        cmd: ConsolidateCommand,
    ) -> Result<ReflectionReport, ApplicationError> {
        let mut memories = Vec::new();
        for id in &cmd.memory_ids {
            if let Some(mem) = self.memory_repo.find_by_id(id).await? {
                memories.push(mem);
            }
        }

        let clusterable: Vec<ClusterableMemory> = memories
            .into_iter()
            .map(|m| ClusterableMemory {
                id: m.id,
                content: m.content.text().to_string(),
                project: m.project.clone(),
                memory_type: format!("{:?}", m.memory_type),
                embedding: None,
                created_at: m.created_at,
            })
            .collect();

        let pattern = Pattern {
            pattern_type: PatternType::CorrelatedObservation,
            description: format!(
                "Consolidación explícita de {} experiencias",
                clusterable.len()
            ),
            frequency: clusterable.len(),
            supporting_memory_ids: cmd.memory_ids.clone(),
        };

        let hyp = self
            .llm_client
            .synthesize_hypothesis(&pattern, &clusterable)
            .await
            .map_err(|e| {
                ApplicationError::Domain(brain_domain::model::DomainError::RepositoryError(
                    e.to_string(),
                ))
            })?;

        let mut created_candidates = Vec::new();

        if !cmd.dry_run {
            if let Some(ref learning) = self.learning_repo {
                let mut candidate =
                    CandidateKnowledge::new_candidate(&hyp.statement, hyp.domain.clone(), None)
                        .map_err(|e| {
                            ApplicationError::Domain(
                                brain_domain::model::DomainError::RepositoryError(e.to_string()),
                            )
                        })?;

                candidate.confidence = Confidence::new(hyp.suggested_confidence)?;

                for src_id in &cmd.memory_ids {
                    let ev = Evidence::new(
                        candidate.id,
                        EvidenceSourceType::DirectObservation,
                        format!("Experiencia consolidada #{}", src_id),
                        true,
                    )
                    .map_err(|e| {
                        ApplicationError::Domain(brain_domain::model::DomainError::RepositoryError(
                            e.to_string(),
                        ))
                    })?
                    .with_memory_id(*src_id);

                    candidate.add_evidence(ev);
                }

                learning.save_candidate(&candidate).await.map_err(|e| {
                    ApplicationError::Domain(brain_domain::model::DomainError::RepositoryError(
                        e.to_string(),
                    ))
                })?;

                created_candidates.push(candidate.id);
            }
        }

        let report = ReflectionReport {
            run_id: Uuid::now_v7(),
            project: cmd.project,
            memories_analyzed: clusterable.len(),
            clusters_formed: 1,
            patterns_detected: vec![pattern],
            hypotheses: vec![hyp],
            conflicts_detected: vec![],
            created_candidates,
            summary: format!(
                "Consolidación explícita exitosa de {} recuerdos",
                clusterable.len()
            ),
            executed_at: Utc::now(),
        };

        if !cmd.dry_run {
            let _ = self.conflict_repo.record_consolidation_run(&report).await;
        }

        Ok(report)
    }
}

/// Comando para resolver un conflicto de conocimiento aportando contexto (SRS §18).
#[derive(Debug, Clone)]
pub struct ResolveConflictCommand {
    pub conflict_id: ConflictId,
    pub resolution_context: String,
}

/// Caso de uso: Resolución de Contradicciones y Conflictos Cognitivos (SRS §18).
pub struct ResolveConflictUseCase {
    conflict_repo: Arc<dyn ConflictRepository>,
    memory_repo: Arc<dyn MemoryRepository>,
}

impl ResolveConflictUseCase {
    pub fn new(
        conflict_repo: Arc<dyn ConflictRepository>,
        memory_repo: Arc<dyn MemoryRepository>,
    ) -> Self {
        Self {
            conflict_repo,
            memory_repo,
        }
    }

    pub async fn execute(
        &self,
        cmd: ResolveConflictCommand,
    ) -> Result<Contradiction, ApplicationError> {
        let mut conflict = self
            .conflict_repo
            .find_conflict_by_id(&cmd.conflict_id)
            .await
            .map_err(|e| {
                ApplicationError::Domain(brain_domain::model::DomainError::RepositoryError(
                    e.to_string(),
                ))
            })?
            .ok_or_else(|| {
                ApplicationError::Domain(brain_domain::model::DomainError::RepositoryError(
                    format!("Conflicto no encontrado: {}", cmd.conflict_id),
                ))
            })?;

        conflict
            .resolve(cmd.resolution_context.clone())
            .map_err(|e| {
                ApplicationError::Domain(brain_domain::model::DomainError::RepositoryError(
                    e.to_string(),
                ))
            })?;

        self.conflict_repo
            .update_conflict(&conflict)
            .await
            .map_err(|e| {
                ApplicationError::Domain(brain_domain::model::DomainError::RepositoryError(
                    e.to_string(),
                ))
            })?;

        // Restaurar estado activo en las memorias involucradas y enriquecer contexto
        if let Some(mut mem_a) = self
            .memory_repo
            .find_by_id(&conflict.source_memory_id)
            .await?
        {
            mem_a.status = MemoryStatus::Active;
            mem_a.source.context_reference =
                Some(format!("Conflicto resuelto: {}", cmd.resolution_context));
            let _ = self.memory_repo.update(&mem_a).await;
        }

        if let Some(mut mem_b) = self
            .memory_repo
            .find_by_id(&conflict.conflicting_memory_id)
            .await?
        {
            mem_b.status = MemoryStatus::Active;
            mem_b.source.context_reference =
                Some(format!("Conflicto resuelto: {}", cmd.resolution_context));
            let _ = self.memory_repo.update(&mem_b).await;
        }

        info!(
            conflict_id = %conflict.id,
            "Conflicto cognitivo resuelto con éxito"
        );

        Ok(conflict)
    }
}

/// Consulta para listar conflictos pendientes.
#[derive(Debug, Clone, Default)]
pub struct ListConflictsQuery {
    pub project: Option<String>,
}

/// Caso de uso: Listar conflictos de conocimiento pendientes.
pub struct ListConflictsUseCase {
    conflict_repo: Arc<dyn ConflictRepository>,
}

impl ListConflictsUseCase {
    pub fn new(conflict_repo: Arc<dyn ConflictRepository>) -> Self {
        Self { conflict_repo }
    }

    pub async fn execute(
        &self,
        query: ListConflictsQuery,
    ) -> Result<Vec<Contradiction>, ApplicationError> {
        self.conflict_repo
            .list_pending_conflicts(query.project.as_deref())
            .await
            .map_err(|e| {
                ApplicationError::Domain(brain_domain::model::DomainError::RepositoryError(
                    e.to_string(),
                ))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brain_consolidation::in_memory::{InMemoryConflictRepository, MockConsolidationLlm};
    use brain_domain::model::{Memory, MemoryContent, MemoryOrigin, Provenance};
    use brain_domain::ports::InMemoryMemoryRepository;
    use brain_graph::InMemoryGraphRepository;
    use brain_learning::InMemoryLearningRepository;

    #[tokio::test]
    async fn test_reflect_detects_contradiction_and_sets_conflict_status() {
        let mem_repo = Arc::new(InMemoryMemoryRepository::new());
        let conflict_repo = Arc::new(InMemoryConflictRepository::new());
        let llm = Arc::new(MockConsolidationLlm::new());
        let graph_repo = Arc::new(InMemoryGraphRepository::new());

        let prov = Provenance::new(MemoryOrigin::Observation);
        let mem1 = Memory::new_episodic(
            MemoryContent::new("Supabase es preferido para proyectos pequeños.").unwrap(),
            prov.clone(),
            Some("web".to_string()),
        );
        let mem2 = Memory::new_episodic(
            MemoryContent::new(".NET es preferido para proyectos pequeños.").unwrap(),
            prov,
            Some("web".to_string()),
        );

        mem_repo.save(&mem1).await.unwrap();
        mem_repo.save(&mem2).await.unwrap();

        let use_case = ReflectUseCase::new(mem_repo.clone(), conflict_repo.clone(), llm)
            .with_graph_repository(Some(graph_repo));

        let report = use_case
            .execute(ReflectCommand::new().with_project("web"))
            .await
            .expect("Reflexión debe ejecutarse");

        assert_eq!(report.conflicts_detected.len(), 1);
        assert_eq!(conflict_repo.count(), 1);

        // Verificar que las memorias cambiaron a status CONFLICT
        let fetched1 = mem_repo.find_by_id(&mem1.id).await.unwrap().unwrap();
        let fetched2 = mem_repo.find_by_id(&mem2.id).await.unwrap().unwrap();
        assert_eq!(fetched1.status, MemoryStatus::Conflict);
        assert_eq!(fetched2.status, MemoryStatus::Conflict);
    }

    #[tokio::test]
    async fn test_reflect_forms_clusters_and_creates_candidate_knowledge() {
        let mem_repo = Arc::new(InMemoryMemoryRepository::new());
        let conflict_repo = Arc::new(InMemoryConflictRepository::new());
        let learning_repo = Arc::new(InMemoryLearningRepository::new());
        let llm = Arc::new(MockConsolidationLlm::new());

        let prov = Provenance::new(MemoryOrigin::Observation);
        let mem1 = Memory::new_episodic(
            MemoryContent::new(
                "El despliegue con Docker tomó 10 minutos debido a capas no cacheadas",
            )
            .unwrap(),
            prov.clone(),
            Some("infra".to_string()),
        );
        let mem2 = Memory::new_episodic(
            MemoryContent::new(
                "El despliegue con Docker volvió a demorar por descarga de base image",
            )
            .unwrap(),
            prov,
            Some("infra".to_string()),
        );

        mem_repo.save(&mem1).await.unwrap();
        mem_repo.save(&mem2).await.unwrap();

        let use_case = ReflectUseCase::new(mem_repo.clone(), conflict_repo.clone(), llm)
            .with_learning_repository(Some(learning_repo.clone()));

        let report = use_case
            .execute(ReflectCommand::new().with_project("infra"))
            .await
            .expect("Reflexión debe ejecutarse");

        assert_eq!(report.clusters_formed, 1);
        assert_eq!(report.hypotheses.len(), 1);
        assert_eq!(report.created_candidates.len(), 1);

        let cand_id = report.created_candidates[0];
        let cand = learning_repo
            .find_candidate_by_id(&cand_id)
            .await
            .unwrap()
            .unwrap();
        assert!(cand.confidence.value() <= 0.40);
        assert_eq!(cand.evidences.len(), 2);
    }

    #[tokio::test]
    async fn test_resolve_conflict_restores_active_status() {
        let mem_repo = Arc::new(InMemoryMemoryRepository::new());
        let conflict_repo = Arc::new(InMemoryConflictRepository::new());
        let llm = Arc::new(MockConsolidationLlm::new());

        let prov = Provenance::new(MemoryOrigin::Observation);
        let mem1 = Memory::new_episodic(
            MemoryContent::new("Supabase es preferido para proyectos pequeños.").unwrap(),
            prov.clone(),
            Some("web".to_string()),
        );
        let mem2 = Memory::new_episodic(
            MemoryContent::new(".NET es preferido para proyectos pequeños.").unwrap(),
            prov,
            Some("web".to_string()),
        );

        mem_repo.save(&mem1).await.unwrap();
        mem_repo.save(&mem2).await.unwrap();

        let reflect_uc = ReflectUseCase::new(mem_repo.clone(), conflict_repo.clone(), llm);
        let report = reflect_uc
            .execute(ReflectCommand::new().with_project("web"))
            .await
            .unwrap();

        let conflict = &report.conflicts_detected[0];

        let resolve_uc = ResolveConflictUseCase::new(conflict_repo.clone(), mem_repo.clone());
        let resolved = resolve_uc
            .execute(ResolveConflictCommand {
                conflict_id: conflict.id,
                resolution_context: "Supabase para MVPs rápidos; .NET para lógica pesada"
                    .to_string(),
            })
            .await
            .expect("Debe resolver conflicto");

        assert_eq!(
            resolved.status,
            brain_consolidation::model::ConflictStatus::Resolved
        );

        // Memorias vuelven a status Active
        let f1 = mem_repo.find_by_id(&mem1.id).await.unwrap().unwrap();
        let f2 = mem_repo.find_by_id(&mem2.id).await.unwrap().unwrap();
        assert_eq!(f1.status, MemoryStatus::Active);
        assert_eq!(f2.status, MemoryStatus::Active);
        assert!(f1.source.context_reference.is_some());
    }
}
