//! Caso de Uso de Aplicación: Recuperación Híbrida Avanzada y Context Assembly (SRS §13, §14, §32, §56, §58).

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use brain_domain::model::{DomainError, Memory, MemoryId, MemoryType};
use brain_domain::ports::{EmbeddingProvider, MemoryRepository, VectorRepository};
use brain_graph::ports::GraphRepository;
use brain_retrieval::{
    FullTextSearchRepository, HybridRetrievalPipeline, RetrievalConfig, RetrievalMetrics,
    ScoredMemory,
};

use crate::use_cases::ApplicationError;

/// Consulta para el caso de uso de recuperación híbrida (SRS §13.2).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HybridRetrieveQuery {
    /// Texto de la consulta semántica / léxica.
    pub query: String,
    /// Filtro opcional por proyecto.
    pub project: Option<String>,
    /// Filtro opcional por tipo de memoria.
    pub memory_type: Option<MemoryType>,
    /// Límite máximo de recuerdos devueltos.
    pub limit: Option<usize>,
    /// Score mínimo admisible en el ranking final.
    pub min_score: Option<f32>,
    /// Si es true, incluye el desglose matemático explicativo en el contexto (SRS §57).
    pub explain: bool,
}

impl HybridRetrieveQuery {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            ..Default::default()
        }
    }

    pub fn with_project(mut self, project: impl Into<String>) -> Self {
        self.project = Some(project.into());
        self
    }

    pub fn with_type(mut self, memory_type: MemoryType) -> Self {
        self.memory_type = Some(memory_type);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_min_score(mut self, min_score: f32) -> Self {
        self.min_score = Some(min_score);
        self
    }

    pub fn with_explain(mut self, explain: bool) -> Self {
        self.explain = explain;
        self
    }
}

/// Resultado completo de la recuperación híbrida con métricas y contexto seguro (SRS §13.2, §32).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridRetrieveResult {
    /// Lista de recuerdos clasificados y puntuados.
    pub items: Vec<ScoredMemory>,
    /// Métricas de observabilidad de los canales.
    pub metrics: RetrievalMetrics,
    /// Contexto seguro formateado con delimitadores anti-prompt-injection.
    pub assembled_context: String,
}

/// Caso de Uso: Recuperación Híbrida Avanzada (SRS §13, §14, §56, §58).
#[derive(Clone)]
pub struct HybridRetrieveUseCase {
    memory_repo: Arc<dyn MemoryRepository>,
    vector_repo: Arc<dyn VectorRepository>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    fts_repo: Arc<dyn FullTextSearchRepository>,
    graph_repo: Option<Arc<dyn GraphRepository>>,
    pipeline: HybridRetrievalPipeline,
}

impl HybridRetrieveUseCase {
    /// Inicializa el caso de uso con sus puertos secundarios requeridos y configuración.
    pub fn new(
        memory_repo: Arc<dyn MemoryRepository>,
        vector_repo: Arc<dyn VectorRepository>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
        fts_repo: Arc<dyn FullTextSearchRepository>,
        graph_repo: Option<Arc<dyn GraphRepository>>,
        config: RetrievalConfig,
    ) -> Self {
        let pipeline = HybridRetrievalPipeline::new(config);
        Self {
            memory_repo,
            vector_repo,
            embedding_provider,
            fts_repo,
            graph_repo,
            pipeline,
        }
    }

    /// Retorna una referencia al pipeline de recuperación subyacente.
    pub fn pipeline(&self) -> &HybridRetrievalPipeline {
        &self.pipeline
    }

    /// Ejecuta el pipeline completo de recuperación híbrida:
    /// Query → Intent/Filtros → Vector Search + FTS + Graph → Fusion RRF → Scoring + Decay → Reranking → Context Assembly
    #[instrument(skip(self), fields(query = %query.query))]
    pub async fn execute(
        &self,
        query: HybridRetrieveQuery,
    ) -> Result<HybridRetrieveResult, ApplicationError> {
        let trimmed_query = query.query.trim();
        if trimmed_query.is_empty() {
            return Err(ApplicationError::InvalidQuery(
                "La consulta de recuperación no puede estar vacía".into(),
            ));
        }

        let limit = query
            .limit
            .unwrap_or(self.pipeline.config().limits.default_limit)
            .clamp(1, self.pipeline.config().limits.max_limit);

        let fetch_candidate_limit = (limit * 4).max(30);

        // 1. Canal Vectorial (búsqueda densa con pgvector)
        let query_vec = self
            .embedding_provider
            .embed(trimmed_query)
            .await
            .map_err(|e| {
                ApplicationError::Domain(DomainError::RepositoryError(format!(
                    "Fallo al generar embedding de consulta: {e}"
                )))
            })?;

        let vector_results = self
            .vector_repo
            .search_similar(&query_vec, fetch_candidate_limit)
            .await
            .unwrap_or_default();

        // 2. Canal FTS Léxico (PostgreSQL tsvector / BM25)
        let fts_results = self
            .fts_repo
            .search_fulltext(trimmed_query, fetch_candidate_limit)
            .await
            .unwrap_or_default();

        // 3. Canal de Grafo (expansión de vecindad de conceptos)
        let mut graph_results: Vec<(MemoryId, usize, f32)> = Vec::new();
        if let Some(ref graph) = self.graph_repo {
            let mut seed_ids = Vec::new();
            for &(id, _) in vector_results.iter().take(3) {
                seed_ids.push(id);
            }
            for &(id, _) in fts_results.iter().take(2) {
                if !seed_ids.contains(&id) {
                    seed_ids.push(id);
                }
            }

            for seed_id in seed_ids {
                let root_node = brain_graph::model::NodeId::from_memory_id(&seed_id);
                let mut options = brain_graph::model::GraphTraversalOptions::new();
                options.max_depth = self.pipeline.config().limits.graph_max_depth as u32;
                options.limit = self.pipeline.config().limits.graph_expansion_limit;

                if let Ok(subgraph) = graph.traverse(&root_node, &options).await {
                    for node in subgraph.nodes {
                        if let Some(target_mid) = node.memory_id {
                            if target_mid != seed_id {
                                graph_results.push((target_mid, 1, 0.8));
                            }
                        }
                    }
                }
            }
        }

        // 4. Fusión de candidatos mediante Reciprocal Rank Fusion (RRF)
        let fused_candidates =
            self.pipeline
                .fuse_candidates(&vector_results, &fts_results, &graph_results);

        let merged_unique_count = fused_candidates.len();

        // 5. Cargar memorias y aplicar filtros de metadatos (proyecto, tipo, estado activo)
        let mut loaded_map: HashMap<MemoryId, Memory> = HashMap::new();
        for candidate in &fused_candidates {
            if let Ok(Some(memory)) = self.memory_repo.find_by_id(&candidate.memory_id).await {
                if !memory.is_active() {
                    continue;
                }
                if let Some(ref proj) = query.project {
                    if memory.project.as_deref() != Some(proj) {
                        continue;
                    }
                }
                if let Some(mtype) = query.memory_type {
                    if memory.memory_type != mtype {
                        continue;
                    }
                }
                loaded_map.insert(candidate.memory_id, memory);
            }
        }

        // 6. Scoring multidimensional, decaimiento temporal y reranking
        let ranked_items = self
            .pipeline
            .rank_and_filter(&fused_candidates, loaded_map, limit, query.min_score)
            .map_err(ApplicationError::from)?;

        // 7. Refrescar timestamp de última recuperación para retención cognitiva (SRS §58)
        for item in &ranked_items {
            let mut updated_mem = item.memory.clone();
            updated_mem.mark_retrieved();
            let _ = self.memory_repo.update(&updated_mem).await;
        }

        // 8. Ensamblado de contexto seguro
        let assembled_context =
            self.pipeline
                .assemble_context(trimmed_query, &ranked_items, query.explain);

        let metrics = RetrievalMetrics {
            vector_candidates_count: vector_results.len(),
            fts_candidates_count: fts_results.len(),
            graph_candidates_count: graph_results.len(),
            merged_unique_count,
            final_returned_count: ranked_items.len(),
        };

        debug!(
            returned = ranked_items.len(),
            vectors = vector_results.len(),
            fts = fts_results.len(),
            "Recuperación híbrida completada exitosamente"
        );

        Ok(HybridRetrieveResult {
            items: ranked_items,
            metrics,
            assembled_context,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brain_domain::model::{Importance, MemoryContent, MemoryOrigin, MemoryType, Provenance};
    use brain_domain::ports::{
        InMemoryEmbeddingProvider, InMemoryMemoryRepository, InMemoryVectorRepository,
    };
    use brain_retrieval::InMemoryFullTextSearchRepository;

    #[tokio::test]
    async fn test_hybrid_retrieve_end_to_end_flow() {
        let mem_repo = Arc::new(InMemoryMemoryRepository::new());
        let vec_repo = Arc::new(InMemoryVectorRepository::new());
        let emb_provider = Arc::new(InMemoryEmbeddingProvider::new());
        let fts_repo = Arc::new(InMemoryFullTextSearchRepository::new());

        let mut memory = Memory::new(
            MemoryContent::new("Hexagonal Architecture in Rust").unwrap(),
            MemoryType::Episodic,
            Provenance::new(MemoryOrigin::Observation).with_agent("test-agent"),
        );
        memory.project = Some("local-brain".into());
        memory.importance = Importance::new(0.95).unwrap();

        let id = memory.id;
        mem_repo.save(&memory).await.unwrap();

        // Indexar en vector repository y fts
        let vec = emb_provider
            .embed("Hexagonal Architecture in Rust")
            .await
            .unwrap();
        vec_repo.store_embedding(&id, &vec).await.unwrap();
        fts_repo.index_document(id, "Hexagonal Architecture in Rust");

        let use_case = HybridRetrieveUseCase::new(
            mem_repo.clone(),
            vec_repo.clone(),
            emb_provider.clone(),
            fts_repo.clone(),
            None,
            RetrievalConfig::default(),
        );

        let query = HybridRetrieveQuery::new("Hexagonal Rust")
            .with_project("local-brain")
            .with_explain(true);

        let result = use_case.execute(query).await.unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].memory.id, id);
        assert!(result.items[0].explanation.final_score > 0.5);
        assert!(result
            .assembled_context
            .contains("<untrusted_memory_context>"));
        assert!(result
            .assembled_context
            .contains("Hexagonal Architecture in Rust"));
        assert!(result.metrics.vector_candidates_count >= 1);
        assert!(result.metrics.fts_candidates_count >= 1);
    }
}
