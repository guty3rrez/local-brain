//! Casos de uso de la Capa de Aplicación de Local Brain (SRS §8, §25).

use std::sync::Arc;

use chrono::{DateTime, Utc};
use thiserror::Error;
use tracing::{info, instrument, warn};

use brain_domain::model::{
    AssociativeMemoryData, Confidence, DomainError, EpisodicMemoryData, Importance, Memory,
    MemoryContent, MemoryId, MemoryOrigin, MemoryStatus, MemoryType, MemoryTypeData,
    ProceduralMemoryData, ProcedureStep, Provenance, SemanticMemoryData, WorkingMemoryData,
};
use brain_domain::ports::{EmbeddingProvider, MemoryRepository, VectorRepository};

/// Errores derivados de la orquestación y casos de uso.
#[derive(Debug, Error, PartialEq)]
pub enum ApplicationError {
    #[error("Error de dominio: {0}")]
    Domain(#[from] DomainError),

    #[error("Error de grafo: {0}")]
    Graph(#[from] brain_graph::GraphError),

    #[error("Recuerdo con ID {0} no encontrado")]
    NotFound(MemoryId),

    #[error("Elemento no encontrado: {0}")]
    NotFoundString(String),

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
    pub type_data: Option<MemoryTypeData>,
    pub ttl_seconds: Option<u64>,
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
            type_data: None,
            ttl_seconds: None,
        }
    }

    pub fn working(
        session_id: impl Into<String>,
        content: impl Into<String>,
        ttl_seconds: Option<u64>,
    ) -> Result<Self, DomainError> {
        let sid = session_id.into();
        let mut data = WorkingMemoryData::new(sid.clone())?;
        if let Some(ttl) = ttl_seconds {
            data = data.with_ttl(ttl)?;
        }
        let txt = content.into();
        data = data.with_goal(txt.clone());
        let mut cmd = Self::new(txt)
            .with_type(MemoryType::Working)
            .with_session(sid);
        cmd.type_data = Some(MemoryTypeData::Working(data));
        cmd.ttl_seconds = ttl_seconds;
        Ok(cmd)
    }

    pub fn episodic(
        project: impl Into<String>,
        agent: impl Into<String>,
        context: impl Into<String>,
        action: impl Into<String>,
        outcome: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let data = EpisodicMemoryData::new(
            project.into(),
            agent.into(),
            context.into(),
            action.into(),
            outcome.into(),
        )?;
        let txt = data.to_content_text();
        let mut cmd = Self::new(txt)
            .with_type(MemoryType::Episodic)
            .with_project(data.project.clone())
            .with_agent(data.agent.clone());
        cmd.type_data = Some(MemoryTypeData::Episodic(data));
        Ok(cmd)
    }

    pub fn semantic(
        statement: impl Into<String>,
        confidence: f32,
        evidence_ids: Vec<MemoryId>,
    ) -> Result<Self, DomainError> {
        let conf = Confidence::new(confidence)?;
        let data = SemanticMemoryData::new(statement.into(), conf, evidence_ids)?;
        let mut cmd = Self::new(data.statement.clone())
            .with_type(MemoryType::Semantic)
            .with_confidence(confidence);
        cmd.type_data = Some(MemoryTypeData::Semantic(data));
        Ok(cmd)
    }

    pub fn procedural(
        name: impl Into<String>,
        goal: impl Into<String>,
        steps: Vec<ProcedureStep>,
    ) -> Result<Self, DomainError> {
        let data = ProceduralMemoryData::new(name.into(), goal.into(), steps)?;
        let txt = data.to_content_text();
        let mut cmd = Self::new(txt).with_type(MemoryType::Procedural);
        cmd.type_data = Some(MemoryTypeData::Procedural(data));
        Ok(cmd)
    }

    pub fn associative(
        source_concept: impl Into<String>,
        target_concept: impl Into<String>,
        predicate: impl Into<String>,
        strength: f32,
    ) -> Result<Self, DomainError> {
        let data = AssociativeMemoryData::new(source_concept, target_concept, predicate, strength)?;
        let txt = data.to_content_text();
        let mut cmd = Self::new(txt)
            .with_type(MemoryType::Associative)
            .with_importance(strength);
        cmd.type_data = Some(MemoryTypeData::Associative(data));
        Ok(cmd)
    }

    pub fn with_type_data(mut self, type_data: MemoryTypeData) -> Self {
        self.type_data = Some(type_data);
        self
    }

    pub fn with_ttl(mut self, ttl: u64) -> Self {
        self.ttl_seconds = Some(ttl);
        self
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
        let mem_type = cmd.memory_type.unwrap_or(MemoryType::Episodic);
        let origin = cmd.origin.unwrap_or(MemoryOrigin::Observation);

        let mut provenance = Provenance::new(origin);
        if let Some(agent) = cmd.agent.clone() {
            provenance = provenance.with_agent(agent);
        }
        if let Some(session_id) = cmd.session_id.clone() {
            provenance = provenance.with_session(session_id);
        }
        if let Some(ctx) = cmd.context_reference {
            provenance = provenance.with_context(ctx);
        }

        let mut memory = match cmd.type_data {
            Some(MemoryTypeData::Working(w)) => Memory::new_working_specialized(w, provenance)?,
            Some(MemoryTypeData::Episodic(e)) => {
                let imp = cmd.importance.map(Importance::new).transpose()?;
                Memory::new_episodic_specialized(e, provenance, imp)?
            }
            Some(MemoryTypeData::Semantic(s)) => {
                let conf = match cmd.confidence {
                    Some(c) => Confidence::new(c)?,
                    None => Confidence::default(),
                };
                Memory::new_semantic_specialized(s, conf, provenance)?
            }
            Some(MemoryTypeData::Procedural(p)) => {
                Memory::new_procedural_specialized(p, provenance)?
            }
            Some(MemoryTypeData::Associative(a)) => {
                Memory::new_associative_specialized(a, provenance)?
            }
            Some(MemoryTypeData::Generic) | None => {
                let content = MemoryContent::new(cmd.content)?;
                let mut mem = Memory::new(content, mem_type, provenance);
                if let Some(ttl) = cmd.ttl_seconds {
                    mem.expires_at = Some(mem.created_at + chrono::Duration::seconds(ttl as i64));
                }
                mem
            }
        };

        if memory.project.is_none() {
            memory.project = cmd.project;
        }
        if memory.agent.is_none() {
            memory.agent = cmd.agent;
        }

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
    pub session_id: Option<String>,
    pub concept: Option<String>,
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

    pub fn by_session(session_id: impl Into<String>) -> Self {
        Self {
            session_id: Some(session_id.into()),
            ..Default::default()
        }
    }

    pub fn by_concept(concept: impl Into<String>) -> Self {
        Self {
            concept: Some(concept.into()),
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

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_concept(mut self, concept: impl Into<String>) -> Self {
        self.concept = Some(concept.into());
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
        } else if let Some(ref session_id) = query.session_id {
            let mut mems = self
                .repository
                .find_active_by_session(session_id, limit)
                .await?;
            for mem in &mut mems {
                mem.mark_retrieved();
                let _ = self.repository.update(mem).await;
            }
            mems
        } else if let Some(ref concept) = query.concept {
            let mut mems = self.repository.find_associations(concept, limit).await?;
            for mem in &mut mems {
                mem.mark_retrieved();
                let _ = self.repository.update(mem).await;
            }
            mems
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

        memories.retain(|m| m.is_active());
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

/// Caso de Uso: Finalizar sesión activa y archivar sus memorias de trabajo (SRS §10.1, F4-01).
#[derive(Clone)]
pub struct ExpireSessionUseCase {
    repository: Arc<dyn MemoryRepository>,
}

impl ExpireSessionUseCase {
    pub fn new(repository: Arc<dyn MemoryRepository>) -> Self {
        Self { repository }
    }

    #[instrument(skip(self))]
    pub async fn execute(&self, session_id: &str) -> Result<usize, ApplicationError> {
        let count = self.repository.expire_session(session_id).await?;
        info!(
            session_id = %session_id,
            count,
            "Sesión de trabajo finalizada y memorias archivadas"
        );
        Ok(count)
    }
}

/// Caso de Uso: Archivar de forma masiva todas las memorias con TTL vencido (SRS §10.1, F4-01).
#[derive(Clone)]
pub struct PurgeExpiredUseCase {
    repository: Arc<dyn MemoryRepository>,
}

impl PurgeExpiredUseCase {
    pub fn new(repository: Arc<dyn MemoryRepository>) -> Self {
        Self { repository }
    }

    #[instrument(skip(self))]
    pub async fn execute(&self) -> Result<usize, ApplicationError> {
        let count = self.repository.purge_expired().await?;
        info!(
            count,
            "Memorias vencidas por TTL purgadas y archivadas exitosamente"
        );
        Ok(count)
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

    #[tokio::test]
    async fn remember_and_recall_specialized_memory_types() {
        let mem_repo = Arc::new(InMemoryMemoryRepository::new());
        let remember_uc = RememberUseCase::new(mem_repo.clone());
        let recall_uc = RecallUseCase::new(mem_repo.clone());
        let expire_session_uc = ExpireSessionUseCase::new(mem_repo.clone());

        // 1. Episodic
        let ep_cmd = RememberCommand::episodic(
            "local-brain",
            "antigravity",
            "Testing de arquitectura",
            "Ejecutar tests puros",
            "100% de éxito en milisegundos",
        )
        .unwrap();
        let ep_mem = remember_uc.execute(ep_cmd).await.unwrap();
        assert_eq!(ep_mem.memory_type, MemoryType::Episodic);
        assert_eq!(ep_mem.project.as_deref(), Some("local-brain"));

        // 2. Semantic
        let sem_cmd = RememberCommand::semantic(
            "Arquitectura Hexagonal desacopla infraestructura del dominio",
            0.9,
            vec![ep_mem.id],
        )
        .unwrap();
        let sem_mem = remember_uc.execute(sem_cmd).await.unwrap();
        assert_eq!(sem_mem.memory_type, MemoryType::Semantic);

        // 3. Procedural
        let steps = vec![
            ProcedureStep::new(1, "Escribir test").unwrap(),
            ProcedureStep::new(2, "Implementar código").unwrap(),
            ProcedureStep::new(3, "Refactorizar").unwrap(),
        ];
        let proc_cmd =
            RememberCommand::procedural("TDD Cycle", "Ciclo Red-Green-Refactor", steps).unwrap();
        let proc_mem = remember_uc.execute(proc_cmd).await.unwrap();
        assert_eq!(proc_mem.memory_type, MemoryType::Procedural);

        // 4. Associative y consulta por concepto
        let assoc_cmd =
            RememberCommand::associative("PostgreSQL", "pgvector", "extends_with", 0.95).unwrap();
        remember_uc.execute(assoc_cmd).await.unwrap();

        let assoc_results = recall_uc
            .execute(RecallQuery::by_concept("pgvector"))
            .await
            .unwrap();
        assert_eq!(assoc_results.len(), 1);

        // 5. Working Memory con sesión y TTL
        let work_cmd =
            RememberCommand::working("sess-xyz", "Investigar latencia", Some(3600)).unwrap();
        let work_mem = remember_uc.execute(work_cmd).await.unwrap();
        assert_eq!(work_mem.memory_type, MemoryType::Working);
        assert!(work_mem.expires_at.is_some());

        // Recall by session
        let session_results = recall_uc
            .execute(RecallQuery::by_session("sess-xyz"))
            .await
            .unwrap();
        assert_eq!(session_results.len(), 1);

        // Expire session
        let expired_count = expire_session_uc.execute("sess-xyz").await.unwrap();
        assert_eq!(expired_count, 1);

        // Tras expirar la sesión, no debe aparecer en recall activo
        let session_after = recall_uc
            .execute(RecallQuery::by_session("sess-xyz"))
            .await
            .unwrap();
        assert!(session_after.is_empty());
    }

    #[tokio::test]
    async fn purge_expired_use_case_cleans_expired_memories() {
        let mem_repo = Arc::new(InMemoryMemoryRepository::new());
        let remember_uc = RememberUseCase::new(mem_repo.clone());
        let recall_uc = RecallUseCase::new(mem_repo.clone());
        let purge_uc = PurgeExpiredUseCase::new(mem_repo.clone());

        // Crear memoria de trabajo
        let cmd = RememberCommand::working("sess-abc", "Hipótesis corta", Some(60)).unwrap();
        let mut mem = remember_uc.execute(cmd).await.unwrap();

        // Forzar fecha de expiración en el pasado
        mem.expires_at = Some(Utc::now() - chrono::Duration::seconds(5));
        mem_repo.update(&mem).await.unwrap();

        // Recall no la debe retornar porque is_active() la descarta
        let results = recall_uc
            .execute(RecallQuery::by_session("sess-abc"))
            .await
            .unwrap();
        assert!(results.is_empty());

        // PurgeExpiredUseCase la archiva formalmente
        let purged = purge_uc.execute().await.unwrap();
        assert_eq!(purged, 1);

        let saved = mem_repo.find_by_id(&mem.id).await.unwrap().unwrap();
        assert_eq!(saved.status, MemoryStatus::Archived);
    }
}
