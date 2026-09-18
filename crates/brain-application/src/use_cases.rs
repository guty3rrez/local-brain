//! Casos de uso de la Capa de Aplicación de Local Brain (SRS §8, §25).

use std::sync::Arc;

use chrono::{DateTime, Utc};
use thiserror::Error;
use tracing::{info, instrument, warn};

use brain_domain::model::{
    Confidence, DomainError, Importance, Memory, MemoryContent, MemoryId, MemoryOrigin,
    MemoryStatus, MemoryType, Provenance,
};
use brain_domain::ports::{EmbeddingProvider, MemoryRepository, VectorRepository};

/// Errores derivados de la orquestación y casos de uso.
#[derive(Debug, Error, PartialEq)]
pub enum ApplicationError {
    #[error("Error de dominio: {0}")]
    Domain(#[from] DomainError),

    #[error("Recuerdo con ID {0} no encontrado")]
    NotFound(MemoryId),

    #[error("Parámetros de consulta inválidos: {0}")]
    InvalidQuery(String),
}

/// Comando para almacenar un nuevo recuerdo en el cerebro.
#[derive(Debug, Clone)]
pub struct RememberCommand {
    pub content: String,
    pub memory_type: Option<MemoryType>,
    pub project: Option<String>,
    pub agent: Option<String>,
    pub importance: Option<f32>,
    pub confidence: Option<f32>,
    pub origin: Option<MemoryOrigin>,
    pub session_id: Option<String>,
    pub context_reference: Option<String>,
}

impl RememberCommand {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            memory_type: None,
            project: None,
            agent: None,
            importance: None,
            confidence: None,
            origin: None,
            session_id: None,
            context_reference: None,
        }
    }

    pub fn with_type(mut self, mem_type: MemoryType) -> Self {
        self.memory_type = Some(mem_type);
        self
    }

    pub fn with_project(mut self, project: impl Into<String>) -> Self {
        self.project = Some(project.into());
        self
    }

    pub fn with_agent(mut self, agent: impl Into<String>) -> Self {
        self.agent = Some(agent.into());
        self
    }

    pub fn with_importance(mut self, importance: f32) -> Self {
        self.importance = Some(importance);
        self
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = Some(confidence);
        self
    }

    pub fn with_origin(mut self, origin: MemoryOrigin) -> Self {
        self.origin = Some(origin);
        self
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context_reference = Some(context.into());
        self
    }
}

/// Caso de Uso: Registrar y persistir un nuevo recuerdo cognitivo (SRS §8, §9, §12.3).
#[derive(Clone)]
pub struct RememberUseCase {
    repository: Arc<dyn MemoryRepository>,
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
    vector_repository: Option<Arc<dyn VectorRepository>>,
}

impl RememberUseCase {
    pub fn new(repository: Arc<dyn MemoryRepository>) -> Self {
        Self {
            repository,
            embedding_provider: None,
            vector_repository: None,
        }
    }

    pub fn with_embedding(
        repository: Arc<dyn MemoryRepository>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
        vector_repository: Arc<dyn VectorRepository>,
    ) -> Self {
        Self {
            repository,
            embedding_provider: Some(embedding_provider),
            vector_repository: Some(vector_repository),
        }
    }

    #[instrument(skip(self, cmd), fields(project = ?cmd.project, mem_type = ?cmd.memory_type))]
    pub async fn execute(&self, cmd: RememberCommand) -> Result<Memory, ApplicationError> {
        let content = MemoryContent::new(cmd.content)?;
        let mem_type = cmd.memory_type.unwrap_or(MemoryType::Episodic);
        let origin = cmd.origin.unwrap_or(MemoryOrigin::Observation);

        let mut provenance = Provenance::new(origin);
        if let Some(agent) = cmd.agent {
            provenance = provenance.with_agent(agent);
        }
        if let Some(session_id) = cmd.session_id {
            provenance = provenance.with_session(session_id);
        }
        if let Some(ctx) = cmd.context_reference {
            provenance = provenance.with_context(ctx);
        }

        let mut memory = Memory::new(content, mem_type, provenance);
        memory.project = cmd.project;

        if let Some(imp_val) = cmd.importance {
            memory.importance = Importance::new(imp_val)?;
        }
        if let Some(conf_val) = cmd.confidence {
            memory.confidence = Confidence::new(conf_val)?;
        }

        // Generación de embedding y persistencia con fallback asíncrono (SRS §12.3)
        if let (Some(provider), Some(vector_repo)) =
            (&self.embedding_provider, &self.vector_repository)
        {
            match provider.embed(memory.content.text()).await {
                Ok(vector) => {
                    memory.embedding = Some(vector.clone());
                    self.repository.save(&memory).await?;
                    if let Err(e) = vector_repo.store_embedding(&memory.id, &vector).await {
                        warn!(
                            memory_id = %memory.id,
                            error = %e,
                            "No se pudo indexar vector; memoria queda en pending_embedding"
                        );
                        memory.status = MemoryStatus::PendingEmbedding;
                        let _ = self.repository.update(&memory).await;
                    }
                }
                Err(e) => {
                    warn!(
                        memory_id = %memory.id,
                        error = %e,
                        "Servicio de embeddings no disponible; guardando con pending_embedding (SRS §12.3)"
                    );
                    memory.status = MemoryStatus::PendingEmbedding;
                    self.repository.save(&memory).await?;
                }
            }
        } else {
            self.repository.save(&memory).await?;
        }

        info!(
            memory_id = %memory.id,
            status = ?memory.status,
            "Nuevo recuerdo persistido exitosamente"
        );

        Ok(memory)
    }
}

/// Parámetros de consulta para recuperar recuerdos (SRS §13).
#[derive(Debug, Clone, Default)]
pub struct RecallQuery {
    pub id: Option<MemoryId>,
    pub project: Option<String>,
    pub memory_type: Option<MemoryType>,
    pub query: Option<String>,
    pub min_similarity: Option<f32>,
    pub min_importance: Option<f32>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
}

impl RecallQuery {
    pub fn by_id(id: MemoryId) -> Self {
        Self {
            id: Some(id),
            ..Default::default()
        }
    }

    pub fn by_project(project: impl Into<String>) -> Self {
        Self {
            project: Some(project.into()),
            ..Default::default()
        }
    }

    pub fn by_query(query: impl Into<String>) -> Self {
        Self {
            query: Some(query.into()),
            ..Default::default()
        }
    }

    pub fn with_query(mut self, query: impl Into<String>) -> Self {
        self.query = Some(query.into());
        self
    }

    pub fn with_type(mut self, mem_type: MemoryType) -> Self {
        self.memory_type = Some(mem_type);
        self
    }

    pub fn with_min_similarity(mut self, min_sim: f32) -> Self {
        self.min_similarity = Some(min_sim);
        self
    }

    pub fn with_min_importance(mut self, min_imp: f32) -> Self {
        self.min_importance = Some(min_imp);
        self
    }

    pub fn with_from_date(mut self, from: DateTime<Utc>) -> Self {
        self.from_date = Some(from);
        self
    }

    pub fn with_to_date(mut self, to: DateTime<Utc>) -> Self {
        self.to_date = Some(to);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Caso de Uso: Recuperar recuerdos por ID, criterios de filtrado o búsqueda semántica vectorial (SRS §13, §25.2).
#[derive(Clone)]
pub struct RecallUseCase {
    repository: Arc<dyn MemoryRepository>,
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
    vector_repository: Option<Arc<dyn VectorRepository>>,
}

impl RecallUseCase {
    pub fn new(repository: Arc<dyn MemoryRepository>) -> Self {
        Self {
            repository,
            embedding_provider: None,
            vector_repository: None,
        }
    }

    pub fn with_embedding(
        repository: Arc<dyn MemoryRepository>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
        vector_repository: Arc<dyn VectorRepository>,
    ) -> Self {
        Self {
            repository,
            embedding_provider: Some(embedding_provider),
            vector_repository: Some(vector_repository),
        }
    }

    #[instrument(skip(self))]
    pub async fn execute(&self, query: RecallQuery) -> Result<Vec<Memory>, ApplicationError> {
        let limit = query.limit.unwrap_or(10);

        let mut memories = if let Some(id) = query.id {
            let maybe_mem = self.repository.find_by_id(&id).await?;
            match maybe_mem {
                Some(mut memory) if memory.is_active() => {
                    memory.mark_retrieved();
                    let _ = self.repository.update(&memory).await;
                    vec![memory]
                }
                _ => return Err(ApplicationError::NotFound(id)),
            }
        } else if let Some(ref text_query) = query.query {
            let (provider, vector_repo) = match (&self.embedding_provider, &self.vector_repository)
            {
                (Some(p), Some(v)) => (p, v),
                _ => {
                    return Err(ApplicationError::InvalidQuery(
                        "Búsqueda semántica no configurada: falta proveedor de embeddings o repositorio vectorial".into(),
                    ));
                }
            };

            let query_vec = provider.embed(text_query).await?;
            let fetch_limit = if query.project.is_some() || query.memory_type.is_some() {
                (limit * 5).max(50)
            } else {
                limit
            };
            let similar_pairs = vector_repo.search_similar(&query_vec, fetch_limit).await?;

            let mut list = Vec::new();
            for (id, similarity) in similar_pairs {
                if let Some(min_sim) = query.min_similarity {
                    if similarity < min_sim {
                        continue;
                    }
                }
                if let Some(mut mem) = self.repository.find_by_id(&id).await? {
                    if !mem.is_active() {
                        continue;
                    }
                    if let Some(ref proj) = query.project {
                        if mem.project.as_deref() != Some(proj) {
                            continue;
                        }
                    }
                    if let Some(mtype) = query.memory_type {
                        if mem.memory_type != mtype {
                            continue;
                        }
                    }
                    mem.mark_retrieved();
                    let _ = self.repository.update(&mem).await;
                    list.push(mem);
                }
            }
            list
        } else if let Some(ref project) = query.project {
            let mut mems = self.repository.find_by_project(project, limit).await?;
            if let Some(mtype) = query.memory_type {
                mems.retain(|m| m.memory_type == mtype);
            }
            for mem in &mut mems {
                mem.mark_retrieved();
                let _ = self.repository.update(mem).await;
            }
            mems
        } else if let Some(mtype) = query.memory_type {
            let mut mems = self.repository.find_by_type(mtype, limit).await?;
            for mem in &mut mems {
                mem.mark_retrieved();
                let _ = self.repository.update(mem).await;
            }
            mems
        } else if query.from_date.is_some()
            || query.to_date.is_some()
            || query.min_importance.is_some()
        {
            let mut all_memories = Vec::new();
            for mtype in [
                MemoryType::Episodic,
                MemoryType::Semantic,
                MemoryType::Procedural,
                MemoryType::Associative,
                MemoryType::Working,
            ] {
                if let Ok(mut mems) = self.repository.find_by_type(mtype, limit).await {
                    all_memories.append(&mut mems);
                }
            }
            all_memories.sort_by_key(|a| std::cmp::Reverse(a.created_at));
            for mem in &mut all_memories {
                mem.mark_retrieved();
                let _ = self.repository.update(mem).await;
            }
            all_memories
        } else {
            return Err(ApplicationError::InvalidQuery(
                "Debe especificarse al menos un ID, texto de consulta (query), proyecto, tipo de memoria o filtro para recall".into(),
            ));
        };

        if let Some(min_imp) = query.min_importance {
            memories.retain(|m| m.importance.value() >= min_imp);
        }
        if let Some(from) = query.from_date {
            memories.retain(|m| m.created_at >= from);
        }
        if let Some(to) = query.to_date {
            memories.retain(|m| m.created_at <= to);
        }
        memories.truncate(limit);

        Ok(memories)
    }
}

/// Caso de Uso: Eliminación lógica (soft-delete) de un recuerdo (SRS §31, §34).
#[derive(Clone)]
pub struct ForgetUseCase {
    repository: Arc<dyn MemoryRepository>,
}

impl ForgetUseCase {
    pub fn new(repository: Arc<dyn MemoryRepository>) -> Self {
        Self { repository }
    }

    #[instrument(skip(self))]
    pub async fn execute(&self, id: MemoryId) -> Result<(), ApplicationError> {
        let maybe_mem = self.repository.find_by_id(&id).await?;
        match maybe_mem {
            Some(mem) if mem.is_active() => {
                self.repository.soft_delete(&id).await?;
                info!(memory_id = %id, "Recuerdo eliminado lógicamente (soft-delete)");
                Ok(())
            }
            _ => Err(ApplicationError::NotFound(id)),
        }
    }
}

/// Caso de Uso: Procesar recuerdos pendientes de computar su vector de embedding (SRS §12.3).
#[derive(Clone)]
pub struct EmbedPendingUseCase {
    memory_repository: Arc<dyn MemoryRepository>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    vector_repository: Arc<dyn VectorRepository>,
}

impl EmbedPendingUseCase {
    pub fn new(
        memory_repository: Arc<dyn MemoryRepository>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
        vector_repository: Arc<dyn VectorRepository>,
    ) -> Self {
        Self {
            memory_repository,
            embedding_provider,
            vector_repository,
        }
    }

    #[instrument(skip(self))]
    pub async fn execute(&self, limit: usize) -> Result<usize, ApplicationError> {
        let pending = self
            .memory_repository
            .find_by_status(MemoryStatus::PendingEmbedding, limit)
            .await?;
        let mut processed = 0;

        for mut memory in pending {
            match self.embedding_provider.embed(memory.content.text()).await {
                Ok(vector) => {
                    self.vector_repository
                        .store_embedding(&memory.id, &vector)
                        .await?;
                    memory.status = MemoryStatus::Active;
                    memory.embedding = Some(vector);
                    self.memory_repository.update(&memory).await?;
                    processed += 1;
                    info!(
                        memory_id = %memory.id,
                        "Embedding generado exitosamente para memoria pendiente"
                    );
                }
                Err(e) => {
                    warn!(
                        memory_id = %memory.id,
                        error = %e,
                        "Fallo al generar embedding para memoria pendiente; continúa en PendingEmbedding"
                    );
                }
            }
        }

        Ok(processed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brain_domain::ports::InMemoryMemoryRepository;

    #[tokio::test]
    async fn remember_and_recall_by_id() {
        let repo = Arc::new(InMemoryMemoryRepository::new());
        let remember_uc = RememberUseCase::new(repo.clone());
        let recall_uc = RecallUseCase::new(repo.clone());

        let cmd = RememberCommand::new("Arquitectura Hexagonal desacoplada")
            .with_project("local-brain")
            .with_agent("antigravity")
            .with_importance(0.9)
            .with_confidence(0.95);

        let stored = remember_uc
            .execute(cmd)
            .await
            .expect("Debe almacenar el recuerdo");
        assert_eq!(stored.content.text(), "Arquitectura Hexagonal desacoplada");
        assert_eq!(stored.project.as_deref(), Some("local-brain"));

        // Recall by ID
        let query = RecallQuery::by_id(stored.id);
        let recalled = recall_uc
            .execute(query)
            .await
            .expect("Debe recuperar el recuerdo");
        assert_eq!(recalled.len(), 1);
        assert_eq!(recalled[0].id, stored.id);
        assert!(recalled[0].last_retrieved_at.is_some());
    }

    #[tokio::test]
    async fn remember_and_recall_by_project() {
        let repo = Arc::new(InMemoryMemoryRepository::new());
        let remember_uc = RememberUseCase::new(repo.clone());
        let recall_uc = RecallUseCase::new(repo.clone());

        let cmd1 = RememberCommand::new("Experiencia 1: Rust es seguro")
            .with_project("local-brain")
            .with_type(MemoryType::Episodic);
        let cmd2 = RememberCommand::new("Experiencia 2: PostgreSQL con pgvector")
            .with_project("local-brain")
            .with_type(MemoryType::Semantic);
        let cmd3 =
            RememberCommand::new("Experiencia en otro proyecto").with_project("otro-proyecto");

        remember_uc.execute(cmd1).await.unwrap();
        remember_uc.execute(cmd2).await.unwrap();
        remember_uc.execute(cmd3).await.unwrap();

        // Query project
        let query = RecallQuery::by_project("local-brain").with_limit(10);
        let memories = recall_uc.execute(query).await.unwrap();
        assert_eq!(memories.len(), 2);

        // Query project with type filter
        let query_semantic = RecallQuery::by_project("local-brain").with_type(MemoryType::Semantic);
        let semantic_memories = recall_uc.execute(query_semantic).await.unwrap();
        assert_eq!(semantic_memories.len(), 1);
        assert_eq!(
            semantic_memories[0].content.text(),
            "Experiencia 2: PostgreSQL con pgvector"
        );
    }

    #[tokio::test]
    async fn recall_not_found() {
        let repo = Arc::new(InMemoryMemoryRepository::new());
        let recall_uc = RecallUseCase::new(repo);

        let random_id = MemoryId::new();
        let query = RecallQuery::by_id(random_id);
        let result = recall_uc.execute(query).await;

        assert_eq!(result, Err(ApplicationError::NotFound(random_id)));
    }

    #[tokio::test]
    async fn remember_and_semantic_recall() {
        use brain_domain::ports::{InMemoryEmbeddingProvider, InMemoryVectorRepository};

        let mem_repo = Arc::new(InMemoryMemoryRepository::new());
        let emb_provider = Arc::new(InMemoryEmbeddingProvider::new());
        let vec_repo = Arc::new(InMemoryVectorRepository::new());

        let remember_uc = RememberUseCase::with_embedding(
            mem_repo.clone(),
            emb_provider.clone(),
            vec_repo.clone(),
        );
        let recall_uc =
            RecallUseCase::with_embedding(mem_repo.clone(), emb_provider.clone(), vec_repo.clone());

        let cmd1 = RememberCommand::new("Arquitectura Hexagonal desacopla el núcleo")
            .with_project("local-brain");
        let cmd2 = RememberCommand::new("PostgreSQL 17 con extensión pgvector")
            .with_project("local-brain");

        let m1 = remember_uc.execute(cmd1).await.unwrap();
        let m2 = remember_uc.execute(cmd2).await.unwrap();

        assert_eq!(m1.status, MemoryStatus::Active);
        assert_eq!(m2.status, MemoryStatus::Active);
        assert!(m1.embedding.is_some());
        assert_eq!(vec_repo.count(), 2);

        // Recall semántico por query
        let query = RecallQuery::by_query("Hexagonal y desacople").with_limit(5);
        let results = recall_uc.execute(query).await.unwrap();

        assert!(!results.is_empty());
        assert_eq!(results[0].id, m1.id);
    }

    struct FailingEmbeddingProvider;

    #[async_trait::async_trait]
    impl EmbeddingProvider for FailingEmbeddingProvider {
        async fn embed(&self, _text: &str) -> Result<Vec<f32>, DomainError> {
            Err(DomainError::RepositoryError("Runtime offline".to_string()))
        }
    }

    #[tokio::test]
    async fn remember_fallback_to_pending_when_embedding_fails() {
        use brain_domain::ports::InMemoryVectorRepository;

        let mem_repo = Arc::new(InMemoryMemoryRepository::new());
        let failing_provider = Arc::new(FailingEmbeddingProvider);
        let vec_repo = Arc::new(InMemoryVectorRepository::new());

        let remember_uc =
            RememberUseCase::with_embedding(mem_repo.clone(), failing_provider, vec_repo.clone());

        let cmd = RememberCommand::new("Memoria cuando el servicio de embeddings está caído")
            .with_project("local-brain");

        // SRS §12.3: No debe fallar; se guarda con status PendingEmbedding
        let memory = remember_uc
            .execute(cmd)
            .await
            .expect("Debe guardar la memoria a pesar del fallo de embedding");

        assert_eq!(memory.status, MemoryStatus::PendingEmbedding);
        assert!(memory.embedding.is_none());
        assert_eq!(vec_repo.count(), 0);

        let saved = mem_repo.find_by_id(&memory.id).await.unwrap().unwrap();
        assert_eq!(saved.status, MemoryStatus::PendingEmbedding);
    }

    #[tokio::test]
    async fn embed_pending_processes_pending_memories() {
        use brain_domain::ports::{InMemoryEmbeddingProvider, InMemoryVectorRepository};

        let mem_repo = Arc::new(InMemoryMemoryRepository::new());
        let provider = Arc::new(InMemoryEmbeddingProvider::new());
        let vec_repo = Arc::new(InMemoryVectorRepository::new());

        // Guardar directamente una memoria en estado PendingEmbedding
        let content = MemoryContent::new("Memoria pendiente para indexar").unwrap();
        let prov = Provenance::new(MemoryOrigin::Observation);
        let mut memory = Memory::new_episodic(content, prov, Some("local-brain".to_string()));
        memory.status = MemoryStatus::PendingEmbedding;
        mem_repo.save(&memory).await.unwrap();

        let embed_pending_uc =
            EmbedPendingUseCase::new(mem_repo.clone(), provider.clone(), vec_repo.clone());

        let processed = embed_pending_uc.execute(10).await.unwrap();
        assert_eq!(processed, 1);
        assert_eq!(vec_repo.count(), 1);

        let updated = mem_repo.find_by_id(&memory.id).await.unwrap().unwrap();
        assert_eq!(updated.status, MemoryStatus::Active);
        assert!(updated.embedding.is_some());
    }

    #[tokio::test]
    async fn forget_use_case_lifecycle() {
        let mem_repo = Arc::new(InMemoryMemoryRepository::new());
        let remember_uc = RememberUseCase::new(mem_repo.clone());
        let recall_uc = RecallUseCase::new(mem_repo.clone());
        let forget_uc = ForgetUseCase::new(mem_repo.clone());

        let cmd = RememberCommand::new("Memoria para olvidar").with_project("test");
        let mem = remember_uc.execute(cmd).await.unwrap();

        // Verificar que está activa
        let recalled = recall_uc.execute(RecallQuery::by_id(mem.id)).await.unwrap();
        assert_eq!(recalled.len(), 1);

        // Olvidar
        forget_uc.execute(mem.id).await.unwrap();

        // Ya no debe ser recuperable vía recall
        let err = recall_uc.execute(RecallQuery::by_id(mem.id)).await;
        assert_eq!(err, Err(ApplicationError::NotFound(mem.id)));

        // Intentar olvidar de nuevo debe retornar NotFound
        let err2 = forget_uc.execute(mem.id).await;
        assert_eq!(err2, Err(ApplicationError::NotFound(mem.id)));
    }

    #[tokio::test]
    async fn recall_with_advanced_filters() {
        let mem_repo = Arc::new(InMemoryMemoryRepository::new());
        let remember_uc = RememberUseCase::new(mem_repo.clone());
        let recall_uc = RecallUseCase::new(mem_repo.clone());

        let cmd1 = RememberCommand::new("Memoria crítica")
            .with_project("local-brain")
            .with_importance(0.95);
        let cmd2 = RememberCommand::new("Memoria trivial")
            .with_project("local-brain")
            .with_importance(0.1);

        remember_uc.execute(cmd1).await.unwrap();
        remember_uc.execute(cmd2).await.unwrap();

        // Filtro con min_importance
        let query = RecallQuery::by_project("local-brain").with_min_importance(0.8);
        let results = recall_uc.execute(query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content.text(), "Memoria crítica");
    }
}
