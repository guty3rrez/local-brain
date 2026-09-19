//! Adaptador de persistencia relacional para Contradicciones y Consolidación en PostgreSQL (SRS §16, §17, §18, Fase 7).

use std::str::FromStr;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use brain_consolidation::errors::ConsolidationError;
use brain_consolidation::model::{
    ConflictId, ConflictStatus, ConflictType, Contradiction, ReflectionReport,
};
use brain_consolidation::ports::ConflictRepository;
use brain_domain::model::MemoryId;

/// Adaptador secundario de persistencia de conflictos y consolidación sobre PostgreSQL.
#[derive(Debug, Clone)]
pub struct PostgresConflictRepository {
    pool: PgPool,
}

impl PostgresConflictRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Ejecuta las migraciones de base de datos de forma idempotente.
    pub async fn run_migrations(&self) -> Result<(), ConsolidationError> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| {
                ConsolidationError::RepositoryError(format!(
                    "Error al ejecutar migraciones SQLx: {e}"
                ))
            })?;
        Ok(())
    }

    fn map_conflict_row(row: &sqlx::postgres::PgRow) -> Result<Contradiction, ConsolidationError> {
        let id_uuid: Uuid = row
            .try_get("id")
            .map_err(|e| ConsolidationError::RepositoryError(e.to_string()))?;
        let src_uuid: Uuid = row
            .try_get("source_memory_id")
            .map_err(|e| ConsolidationError::RepositoryError(e.to_string()))?;
        let target_uuid: Uuid = row
            .try_get("conflicting_memory_id")
            .map_err(|e| ConsolidationError::RepositoryError(e.to_string()))?;
        let ctype_str: String = row
            .try_get("conflict_type")
            .map_err(|e| ConsolidationError::RepositoryError(e.to_string()))?;
        let reason: String = row
            .try_get("reason")
            .map_err(|e| ConsolidationError::RepositoryError(e.to_string()))?;
        let suggested_resolution: Option<String> = row
            .try_get("suggested_resolution")
            .map_err(|e| ConsolidationError::RepositoryError(e.to_string()))?;
        let status_str: String = row
            .try_get("status")
            .map_err(|e| ConsolidationError::RepositoryError(e.to_string()))?;
        let resolution_context: Option<String> = row
            .try_get("resolution_context")
            .map_err(|e| ConsolidationError::RepositoryError(e.to_string()))?;
        let detected_at: DateTime<Utc> = row
            .try_get("detected_at")
            .map_err(|e| ConsolidationError::RepositoryError(e.to_string()))?;
        let resolved_at: Option<DateTime<Utc>> = row
            .try_get("resolved_at")
            .map_err(|e| ConsolidationError::RepositoryError(e.to_string()))?;
        let metadata: serde_json::Value = row
            .try_get("metadata")
            .map_err(|e| ConsolidationError::RepositoryError(e.to_string()))?;

        let conflict_type = ConflictType::from_str(&ctype_str)?;
        let status = ConflictStatus::from_str(&status_str)?;

        Ok(Contradiction {
            id: ConflictId::from_uuid(id_uuid),
            source_memory_id: MemoryId::from_uuid(src_uuid),
            conflicting_memory_id: MemoryId::from_uuid(target_uuid),
            conflict_type,
            reason,
            suggested_resolution,
            status,
            resolution_context,
            detected_at,
            resolved_at,
            metadata,
        })
    }
}

#[async_trait]
impl ConflictRepository for PostgresConflictRepository {
    async fn save_conflict(&self, conflict: &Contradiction) -> Result<(), ConsolidationError> {
        let id = *conflict.id.as_uuid();
        let src_id = *conflict.source_memory_id.as_uuid();
        let target_id = *conflict.conflicting_memory_id.as_uuid();
        let ctype = conflict.conflict_type.as_str();
        let status = conflict.status.as_str();

        sqlx::query(
            r#"
            INSERT INTO knowledge_conflicts (
                id, source_memory_id, conflicting_memory_id, conflict_type,
                reason, suggested_resolution, status, resolution_context,
                detected_at, resolved_at, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (id) DO UPDATE SET
                status = EXCLUDED.status,
                resolution_context = EXCLUDED.resolution_context,
                resolved_at = EXCLUDED.resolved_at,
                metadata = EXCLUDED.metadata
            "#,
        )
        .bind(id)
        .bind(src_id)
        .bind(target_id)
        .bind(ctype)
        .bind(&conflict.reason)
        .bind(&conflict.suggested_resolution)
        .bind(status)
        .bind(&conflict.resolution_context)
        .bind(conflict.detected_at)
        .bind(conflict.resolved_at)
        .bind(&conflict.metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            ConsolidationError::RepositoryError(format!("Error guardando conflicto: {e}"))
        })?;

        Ok(())
    }

    async fn find_conflict_by_id(
        &self,
        id: &ConflictId,
    ) -> Result<Option<Contradiction>, ConsolidationError> {
        let row_opt = sqlx::query(
            r#"
            SELECT id, source_memory_id, conflicting_memory_id, conflict_type,
                   reason, suggested_resolution, status, resolution_context,
                   detected_at, resolved_at, metadata
            FROM knowledge_conflicts
            WHERE id = $1
            "#,
        )
        .bind(*id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            ConsolidationError::RepositoryError(format!("Error buscando conflicto: {e}"))
        })?;

        match row_opt {
            Some(row) => Ok(Some(Self::map_conflict_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_by_memory_pair(
        &self,
        mem_a: &MemoryId,
        mem_b: &MemoryId,
    ) -> Result<Option<Contradiction>, ConsolidationError> {
        let uuid_a = *mem_a.as_uuid();
        let uuid_b = *mem_b.as_uuid();

        let row_opt = sqlx::query(
            r#"
            SELECT id, source_memory_id, conflicting_memory_id, conflict_type,
                   reason, suggested_resolution, status, resolution_context,
                   detected_at, resolved_at, metadata
            FROM knowledge_conflicts
            WHERE (source_memory_id = $1 AND conflicting_memory_id = $2)
               OR (source_memory_id = $2 AND conflicting_memory_id = $1)
            ORDER BY detected_at DESC
            LIMIT 1
            "#,
        )
        .bind(uuid_a)
        .bind(uuid_b)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            ConsolidationError::RepositoryError(format!("Error buscando par de conflicto: {e}"))
        })?;

        match row_opt {
            Some(row) => Ok(Some(Self::map_conflict_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn list_pending_conflicts(
        &self,
        project: Option<&str>,
    ) -> Result<Vec<Contradiction>, ConsolidationError> {
        let rows = if let Some(proj) = project {
            sqlx::query(
                r#"
                SELECT kc.id, kc.source_memory_id, kc.conflicting_memory_id, kc.conflict_type,
                       kc.reason, kc.suggested_resolution, kc.status, kc.resolution_context,
                       kc.detected_at, kc.resolved_at, kc.metadata
                FROM knowledge_conflicts kc
                JOIN memories m ON m.id = kc.source_memory_id
                WHERE kc.status = 'pending' AND m.project = $1
                ORDER BY kc.detected_at DESC
                "#,
            )
            .bind(proj)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query(
                r#"
                SELECT id, source_memory_id, conflicting_memory_id, conflict_type,
                       reason, suggested_resolution, status, resolution_context,
                       detected_at, resolved_at, metadata
                FROM knowledge_conflicts
                WHERE status = 'pending'
                ORDER BY detected_at DESC
                "#,
            )
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| {
            ConsolidationError::RepositoryError(format!("Error listando conflictos: {e}"))
        })?;

        let mut list = Vec::with_capacity(rows.len());
        for row in rows {
            list.push(Self::map_conflict_row(&row)?);
        }
        Ok(list)
    }

    async fn update_conflict(&self, conflict: &Contradiction) -> Result<(), ConsolidationError> {
        let id = *conflict.id.as_uuid();
        let status = conflict.status.as_str();

        let rows_affected = sqlx::query(
            r#"
            UPDATE knowledge_conflicts
            SET status = $1,
                resolution_context = $2,
                resolved_at = $3,
                metadata = $4
            WHERE id = $5
            "#,
        )
        .bind(status)
        .bind(&conflict.resolution_context)
        .bind(conflict.resolved_at)
        .bind(&conflict.metadata)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            ConsolidationError::RepositoryError(format!("Error actualizando conflicto: {e}"))
        })?
        .rows_affected();

        if rows_affected == 0 {
            return Err(ConsolidationError::ConflictNotFound(conflict.id));
        }

        Ok(())
    }

    async fn record_consolidation_run(
        &self,
        report: &ReflectionReport,
    ) -> Result<(), ConsolidationError> {
        sqlx::query(
            r#"
            INSERT INTO consolidation_runs (
                id, project, memories_analyzed, clusters_count,
                hypotheses_count, conflicts_count, summary, executed_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(report.run_id)
        .bind(&report.project)
        .bind(report.memories_analyzed as i32)
        .bind(report.clusters_formed as i32)
        .bind(report.hypotheses.len() as i32)
        .bind(report.conflicts_detected.len() as i32)
        .bind(&report.summary)
        .bind(report.executed_at)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            ConsolidationError::RepositoryError(format!(
                "Error auditando corrida de consolidación: {e}"
            ))
        })?;

        Ok(())
    }
}
