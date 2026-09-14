//! Casos de uso de la Capa de Aplicación de Local Brain (SRS §8, §25).

use std::sync::Arc;

use thiserror::Error;
use tracing::{info, instrument};

use brain_domain::model::{
    Confidence, DomainError, Importance, Memory, MemoryContent, MemoryId, MemoryOrigin, MemoryType,
    Provenance,
};
use brain_domain::ports::MemoryRepository;

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

/// Caso de Uso: Registrar y persistir un nuevo recuerdo cognitivo.
#[derive(Clone)]
pub struct RememberUseCase {
    repository: Arc<dyn MemoryRepository>,
}

impl RememberUseCase {
    pub fn new(repository: Arc<dyn MemoryRepository>) -> Self {
        Self { repository }
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

        self.repository.save(&memory).await?;
        info!(memory_id = %memory.id, "Nuevo recuerdo persistido exitosamente");

        Ok(memory)
    }
}

/// Parámetros de consulta para recuperar recuerdos.
#[derive(Debug, Clone, Default)]
pub struct RecallQuery {
    pub id: Option<MemoryId>,
    pub project: Option<String>,
    pub memory_type: Option<MemoryType>,
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

    pub fn with_type(mut self, mem_type: MemoryType) -> Self {
        self.memory_type = Some(mem_type);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Caso de Uso: Recuperar recuerdos por ID o criterios de filtrado.
#[derive(Clone)]
pub struct RecallUseCase {
    repository: Arc<dyn MemoryRepository>,
}

impl RecallUseCase {
    pub fn new(repository: Arc<dyn MemoryRepository>) -> Self {
        Self { repository }
    }

    #[instrument(skip(self))]
    pub async fn execute(&self, query: RecallQuery) -> Result<Vec<Memory>, ApplicationError> {
        let limit = query.limit.unwrap_or(10);

        if let Some(id) = query.id {
            let maybe_mem = self.repository.find_by_id(&id).await?;
            match maybe_mem {
                Some(mut memory) if memory.is_active() => {
                    memory.mark_retrieved();
                    let _ = self.repository.update(&memory).await;
                    Ok(vec![memory])
                }
                _ => Err(ApplicationError::NotFound(id)),
            }
        } else if let Some(ref project) = query.project {
            let mut memories = self.repository.find_by_project(project, limit).await?;
            if let Some(mtype) = query.memory_type {
                memories.retain(|m| m.memory_type == mtype);
            }
            for mem in &mut memories {
                mem.mark_retrieved();
                let _ = self.repository.update(mem).await;
            }
            Ok(memories)
        } else if let Some(mtype) = query.memory_type {
            let mut memories = self.repository.find_by_type(mtype, limit).await?;
            for mem in &mut memories {
                mem.mark_retrieved();
                let _ = self.repository.update(mem).await;
            }
            Ok(memories)
        } else {
            Err(ApplicationError::InvalidQuery(
                "Debe especificarse al menos un ID, proyecto o tipo de memoria para recall".into(),
            ))
        }
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
}
