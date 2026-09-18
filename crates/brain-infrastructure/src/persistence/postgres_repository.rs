//! Adaptador de Persistencia PostgreSQL para Local Brain (SRS §8, §9.1, §25.1).

use async_trait::async_trait;
use sqlx::{PgPool, Row};

use brain_domain::model::{
    Confidence, DomainError, Importance, Memory, MemoryContent, MemoryId, MemoryStatus, MemoryType,
    MemoryTypeData, Provenance, Utility, Version,
};
use brain_domain::ports::{MemoryRepository, VectorRepository};
use pgvector::Vector;

/// Adaptador secundario de persistencia relacional en PostgreSQL mediante SQLx (SRS §25.1).
#[derive(Debug, Clone)]
pub struct PostgresMemoryRepository {
    pool: PgPool,
}

impl PostgresMemoryRepository {
    /// Inicializa un nuevo repositorio con un pool de conexiones existente.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Retorna una referencia al pool de conexiones subyacente.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Ejecuta las migraciones embebidas de forma programática e idempotente.
    pub async fn run_migrations(&self) -> Result<(), DomainError> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| {
                DomainError::RepositoryError(format!("Error al ejecutar migraciones SQLx: {e}"))
            })?;
        Ok(())
    }
}

#[async_trait]
impl MemoryRepository for PostgresMemoryRepository {
    async fn save(&self, memory: &Memory) -> Result<(), DomainError> {
        let provenance_json = serde_json::to_value(&memory.source).map_err(|e| {
            DomainError::RepositoryError(format!("Error al serializar provenance: {e}"))
        })?;
        let metadata_json = serde_json::to_value(&memory.type_data).map_err(|e| {
            DomainError::RepositoryError(format!("Error al serializar type_data metadata: {e}"))
        })?;
        let pg_vector = memory.embedding.as_ref().map(|v| Vector::from(v.clone()));

        let result = sqlx::query(
            r#"
            INSERT INTO memories (
                id, memory_type, content_text, content_hash, summary,
                project, agent, importance, confidence, utility,
                status, version, provenance, metadata, created_at,
                updated_at, last_retrieved_at, embedding, expires_at
            ) VALUES (
                $1, $2, $3, $4, $5,
                $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15,
                $16, $17, $18, $19
            )
            "#,
        )
        .bind(memory.id.as_uuid())
        .bind(memory_type_to_str(memory.memory_type))
        .bind(memory.content.text())
        .bind(memory.content.hash())
        .bind(&memory.summary)
        .bind(&memory.project)
        .bind(&memory.agent)
        .bind(memory.importance.value())
        .bind(memory.confidence.value())
        .bind(memory.utility.value())
        .bind(memory_status_to_str(memory.status))
        .bind(memory.version.value() as i32)
        .bind(provenance_json)
        .bind(metadata_json)
        .bind(memory.created_at)
        .bind(memory.updated_at)
        .bind(memory.last_retrieved_at)
        .bind(pg_vector)
        .bind(memory.expires_at)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => Err(
                DomainError::RepositoryError(format!("La memoria con id {} ya existe", memory.id)),
            ),
            Err(e) => Err(DomainError::RepositoryError(format!(
                "Error al guardar memoria {}: {e}",
                memory.id
            ))),
        }
    }

    async fn find_by_id(&self, id: &MemoryId) -> Result<Option<Memory>, DomainError> {
        let maybe_row = sqlx::query(
            r#"
            SELECT id, memory_type, content_text, content_hash, summary,
                   project, agent, importance, confidence, utility,
                   status, version, provenance, metadata, created_at,
                   updated_at, last_retrieved_at, embedding, expires_at
            FROM memories
            WHERE id = $1
            "#,
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::RepositoryError(format!("Error al buscar memoria {id}: {e}")))?;

        match maybe_row {
            Some(row) => Ok(Some(row_to_memory(row)?)),
            None => Ok(None),
        }
    }

    async fn find_by_project(
        &self,
        project: &str,
        limit: usize,
    ) -> Result<Vec<Memory>, DomainError> {
        let rows = sqlx::query(
            r#"
            SELECT id, memory_type, content_text, content_hash, summary,
                   project, agent, importance, confidence, utility,
                   status, version, provenance, metadata, created_at,
                   updated_at, last_retrieved_at, embedding, expires_at
            FROM memories
            WHERE project = $1 AND status NOT IN ('soft_deleted', 'archived')
              AND (expires_at IS NULL OR expires_at > NOW())
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(project)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            DomainError::RepositoryError(format!(
                "Error al buscar memorias del proyecto {project}: {e}"
            ))
        })?;

        rows.into_iter().map(row_to_memory).collect()
    }

    async fn find_by_type(
        &self,
        memory_type: MemoryType,
        limit: usize,
    ) -> Result<Vec<Memory>, DomainError> {
        let rows = sqlx::query(
            r#"
            SELECT id, memory_type, content_text, content_hash, summary,
                   project, agent, importance, confidence, utility,
                   status, version, provenance, metadata, created_at,
                   updated_at, last_retrieved_at, embedding, expires_at
            FROM memories
            WHERE memory_type = $1 AND status NOT IN ('soft_deleted', 'archived')
              AND (expires_at IS NULL OR expires_at > NOW())
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(memory_type_to_str(memory_type))
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            DomainError::RepositoryError(format!("Error al buscar memorias por tipo: {e}"))
        })?;

        rows.into_iter().map(row_to_memory).collect()
    }

    async fn find_by_status(
        &self,
        status: MemoryStatus,
        limit: usize,
    ) -> Result<Vec<Memory>, DomainError> {
        let rows = sqlx::query(
            r#"
            SELECT id, memory_type, content_text, content_hash, summary,
                   project, agent, importance, confidence, utility,
                   status, version, provenance, metadata, created_at,
                   updated_at, last_retrieved_at, embedding, expires_at
            FROM memories
            WHERE status = $1
            ORDER BY created_at ASC
            LIMIT $2
            "#,
        )
        .bind(memory_status_to_str(status))
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            DomainError::RepositoryError(format!(
                "Error al buscar memorias por estado {:?}: {e}",
                status
            ))
        })?;

        rows.into_iter().map(row_to_memory).collect()
    }

    async fn find_active_by_session(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<Memory>, DomainError> {
        let rows = sqlx::query(
            r#"
            SELECT id, memory_type, content_text, content_hash, summary,
                   project, agent, importance, confidence, utility,
                   status, version, provenance, metadata, created_at,
                   updated_at, last_retrieved_at, embedding, expires_at
            FROM memories
            WHERE provenance->>'session_id' = $1
              AND memory_type = 'working'
              AND status NOT IN ('soft_deleted', 'archived')
              AND (expires_at IS NULL OR expires_at > NOW())
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(session_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            DomainError::RepositoryError(format!(
                "Error al buscar memorias de sesión {session_id}: {e}"
            ))
        })?;

        rows.into_iter().map(row_to_memory).collect()
    }

    async fn expire_session(&self, session_id: &str) -> Result<usize, DomainError> {
        let result = sqlx::query(
            r#"
            UPDATE memories SET
                status = 'archived',
                updated_at = NOW()
            WHERE provenance->>'session_id' = $1
              AND memory_type = 'working'
              AND status NOT IN ('soft_deleted', 'archived')
            "#,
        )
        .bind(session_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            DomainError::RepositoryError(format!(
                "Error al archivar sesión de trabajo {session_id}: {e}"
            ))
        })?;

        Ok(result.rows_affected() as usize)
    }

    async fn purge_expired(&self) -> Result<usize, DomainError> {
        let result = sqlx::query(
            r#"
            UPDATE memories SET
                status = 'archived',
                updated_at = NOW()
            WHERE expires_at IS NOT NULL
              AND expires_at <= NOW()
              AND status NOT IN ('soft_deleted', 'archived')
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            DomainError::RepositoryError(format!("Error al purgar memorias vencidas: {e}"))
        })?;

        Ok(result.rows_affected() as usize)
    }

    async fn find_associations(
        &self,
        concept: &str,
        limit: usize,
    ) -> Result<Vec<Memory>, DomainError> {
        let pattern = format!("%{concept}%");
        let rows = sqlx::query(
            r#"
            SELECT id, memory_type, content_text, content_hash, summary,
                   project, agent, importance, confidence, utility,
                   status, version, provenance, metadata, created_at,
                   updated_at, last_retrieved_at, embedding, expires_at
            FROM memories
            WHERE memory_type = 'associative'
              AND status NOT IN ('soft_deleted', 'archived')
              AND (
                  metadata->'associative'->>'source_concept' ILIKE $1
                  OR metadata->'associative'->>'target_concept' ILIKE $1
                  OR metadata->>'source_concept' ILIKE $1
                  OR metadata->>'target_concept' ILIKE $1
              )
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(pattern)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            DomainError::RepositoryError(format!(
                "Error al buscar asociaciones para '{concept}': {e}"
            ))
        })?;

        rows.into_iter().map(row_to_memory).collect()
    }

    async fn update(&self, memory: &Memory) -> Result<(), DomainError> {
        let provenance_json = serde_json::to_value(&memory.source).map_err(|e| {
            DomainError::RepositoryError(format!("Error al serializar provenance: {e}"))
        })?;
        let metadata_json = serde_json::to_value(&memory.type_data).map_err(|e| {
            DomainError::RepositoryError(format!("Error al serializar type_data metadata: {e}"))
        })?;
        let pg_vector = memory.embedding.as_ref().map(|v| Vector::from(v.clone()));

        let result = sqlx::query(
            r#"
            UPDATE memories SET
                memory_type = $2,
                content_text = $3,
                content_hash = $4,
                summary = $5,
                project = $6,
                agent = $7,
                importance = $8,
                confidence = $9,
                utility = $10,
                status = $11,
                version = $12,
                provenance = $13,
                metadata = $14,
                updated_at = $15,
                last_retrieved_at = $16,
                embedding = $17,
                expires_at = $18
            WHERE id = $1
            "#,
        )
        .bind(memory.id.as_uuid())
        .bind(memory_type_to_str(memory.memory_type))
        .bind(memory.content.text())
        .bind(memory.content.hash())
        .bind(&memory.summary)
        .bind(&memory.project)
        .bind(&memory.agent)
        .bind(memory.importance.value())
        .bind(memory.confidence.value())
        .bind(memory.utility.value())
        .bind(memory_status_to_str(memory.status))
        .bind(memory.version.value() as i32)
        .bind(provenance_json)
        .bind(metadata_json)
        .bind(memory.updated_at)
        .bind(memory.last_retrieved_at)
        .bind(pg_vector)
        .bind(memory.expires_at)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            DomainError::RepositoryError(format!("Error al actualizar memoria {}: {e}", memory.id))
        })?;

        if result.rows_affected() == 0 {
            return Err(DomainError::RepositoryError(format!(
                "No se puede actualizar: memoria {} no encontrada",
                memory.id
            )));
        }

        Ok(())
    }

    async fn soft_delete(&self, id: &MemoryId) -> Result<(), DomainError> {
        let result = sqlx::query(
            r#"
            UPDATE memories SET
                status = 'soft_deleted',
                updated_at = NOW()
            WHERE id = $1 AND status != 'soft_deleted'
            "#,
        )
        .bind(id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            DomainError::RepositoryError(format!("Error al eliminar memoria {id}: {e}"))
        })?;

        if result.rows_affected() == 0 {
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM memories WHERE id = $1)")
                    .bind(id.as_uuid())
                    .fetch_one(&self.pool)
                    .await
                    .unwrap_or(false);

            if !exists {
                return Err(DomainError::RepositoryError(format!(
                    "No se puede eliminar: memoria {} no encontrada",
                    id
                )));
            }
        }

        Ok(())
    }
}

#[async_trait]
impl VectorRepository for PostgresMemoryRepository {
    async fn store_embedding(&self, id: &MemoryId, embedding: &[f32]) -> Result<(), DomainError> {
        let pg_vector = Vector::from(embedding.to_vec());
        let result = sqlx::query(
            r#"
            UPDATE memories
            SET embedding = $1, updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(pg_vector)
        .bind(id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            DomainError::RepositoryError(format!(
                "Error al almacenar embedding para memoria {id}: {e}"
            ))
        })?;

        if result.rows_affected() == 0 {
            return Err(DomainError::RepositoryError(format!(
                "No se pudo guardar embedding: memoria {id} no encontrada"
            )));
        }

        Ok(())
    }

    async fn search_similar(
        &self,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(MemoryId, f32)>, DomainError> {
        let pg_vector = Vector::from(embedding.to_vec());
        let rows = sqlx::query(
            r#"
            SELECT id, (1.0 - (embedding <=> $1)) AS similarity
            FROM memories
            WHERE status = 'active' AND embedding IS NOT NULL
              AND (expires_at IS NULL OR expires_at > NOW())
            ORDER BY embedding <=> $1 ASC
            LIMIT $2
            "#,
        )
        .bind(pg_vector)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            DomainError::RepositoryError(format!("Error en búsqueda vectorial similar: {e}"))
        })?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            let id: uuid::Uuid = row
                .try_get("id")
                .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
            let similarity: f64 = row
                .try_get("similarity")
                .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
            results.push((MemoryId::from_uuid(id), similarity as f32));
        }

        Ok(results)
    }
}

fn memory_type_to_str(m: MemoryType) -> &'static str {
    match m {
        MemoryType::Working => "working",
        MemoryType::Episodic => "episodic",
        MemoryType::Semantic => "semantic",
        MemoryType::Procedural => "procedural",
        MemoryType::Associative => "associative",
    }
}

fn parse_memory_type(s: &str) -> Result<MemoryType, DomainError> {
    match s {
        "working" => Ok(MemoryType::Working),
        "episodic" => Ok(MemoryType::Episodic),
        "semantic" => Ok(MemoryType::Semantic),
        "procedural" => Ok(MemoryType::Procedural),
        "associative" => Ok(MemoryType::Associative),
        other => Err(DomainError::RepositoryError(format!(
            "Tipo de memoria desconocido en base de datos: {other}"
        ))),
    }
}

fn memory_status_to_str(s: MemoryStatus) -> &'static str {
    match s {
        MemoryStatus::Active => "active",
        MemoryStatus::Archived => "archived",
        MemoryStatus::PendingEmbedding => "pending_embedding",
        MemoryStatus::Conflict => "conflict",
        MemoryStatus::SoftDeleted => "soft_deleted",
    }
}

fn parse_memory_status(s: &str) -> Result<MemoryStatus, DomainError> {
    match s {
        "active" => Ok(MemoryStatus::Active),
        "archived" => Ok(MemoryStatus::Archived),
        "pending_embedding" => Ok(MemoryStatus::PendingEmbedding),
        "conflict" => Ok(MemoryStatus::Conflict),
        "soft_deleted" => Ok(MemoryStatus::SoftDeleted),
        other => Err(DomainError::RepositoryError(format!(
            "Estado de memoria desconocido en base de datos: {other}"
        ))),
    }
}

fn row_to_memory(row: sqlx::postgres::PgRow) -> Result<Memory, DomainError> {
    let id: uuid::Uuid = row
        .try_get("id")
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
    let memory_type_str: String = row
        .try_get("memory_type")
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
    let content_text: String = row
        .try_get("content_text")
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
    let content_hash: String = row
        .try_get("content_hash")
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
    let summary: Option<String> = row
        .try_get("summary")
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
    let project: Option<String> = row
        .try_get("project")
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
    let agent: Option<String> = row
        .try_get("agent")
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
    let importance: f32 = row
        .try_get("importance")
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
    let confidence: f32 = row
        .try_get("confidence")
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
    let utility: f32 = row
        .try_get("utility")
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
    let status_str: String = row
        .try_get("status")
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
    let version: i32 = row
        .try_get("version")
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
    let provenance_val: serde_json::Value = row
        .try_get("provenance")
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
    let metadata_val: serde_json::Value = row.try_get("metadata").unwrap_or(serde_json::json!({}));
    let created_at: chrono::DateTime<chrono::Utc> = row
        .try_get("created_at")
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
    let updated_at: chrono::DateTime<chrono::Utc> = row
        .try_get("updated_at")
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
    let last_retrieved_at: Option<chrono::DateTime<chrono::Utc>> = row
        .try_get("last_retrieved_at")
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
    let expires_at: Option<chrono::DateTime<chrono::Utc>> =
        row.try_get("expires_at").unwrap_or(None);

    let memory_type = parse_memory_type(&memory_type_str)?;
    let status = parse_memory_status(&status_str)?;
    let content = MemoryContent::new(&content_text)?;

    if content.hash() != content_hash {
        return Err(DomainError::RepositoryError(format!(
            "Discrepancia de integridad SHA-256 para memoria {id}: esperado {content_hash}, calculado {}",
            content.hash()
        )));
    }

    let source: Provenance = serde_json::from_value(provenance_val).map_err(|e| {
        DomainError::RepositoryError(format!(
            "Error al deserializar provenance de memoria {id}: {e}"
        ))
    })?;

    let type_data: MemoryTypeData =
        serde_json::from_value(metadata_val).unwrap_or(MemoryTypeData::Generic);

    Ok(Memory {
        id: MemoryId::from_uuid(id),
        memory_type,
        content,
        summary,
        source,
        project,
        agent,
        importance: Importance::new(importance)?,
        confidence: Confidence::new(confidence)?,
        utility: Utility::new(utility)?,
        created_at,
        updated_at,
        last_retrieved_at,
        status,
        version: Version::new(version as u32)?,
        embedding: row
            .try_get::<Option<Vector>, _>("embedding")
            .ok()
            .flatten()
            .map(|v| v.to_vec()),
        type_data,
        expires_at,
    })
}
