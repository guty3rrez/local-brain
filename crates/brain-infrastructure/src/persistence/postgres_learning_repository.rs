//! Adaptador de persistencia relacional para el Motor de Aprendizaje y Conocimiento Candidato en PostgreSQL (SRS §15, §59, §60, Fase 6).

use std::str::FromStr;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use brain_domain::model::{Confidence, MemoryId};
use brain_learning::errors::LearningError;
use brain_learning::model::{
    CandidateId, CandidateKnowledge, Evidence, EvidenceId, EvidenceSourceType, LearningStage,
};
use brain_learning::ports::LearningRepository;

/// Adaptador secundario de persistencia de aprendizaje sobre PostgreSQL mediante SQLx.
#[derive(Debug, Clone)]
pub struct PostgresLearningRepository {
    pool: PgPool,
}

impl PostgresLearningRepository {
    /// Inicializa el repositorio con un pool de conexiones existente.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Retorna una referencia al pool de conexiones.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Ejecuta las migraciones de base de datos de forma programática e idempotente.
    pub async fn run_migrations(&self) -> Result<(), LearningError> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| {
                LearningError::StorageError(format!("Error al ejecutar migraciones SQLx: {e}"))
            })?;
        Ok(())
    }

    /// Helper privado para mapear una fila de `learning_evidence` a la estructura de dominio `Evidence`.
    fn map_evidence_row(row: &sqlx::postgres::PgRow) -> Result<Evidence, LearningError> {
        let id_uuid: Uuid = row
            .try_get("id")
            .map_err(|e| LearningError::StorageError(e.to_string()))?;
        let cand_uuid: Uuid = row
            .try_get("candidate_id")
            .map_err(|e| LearningError::StorageError(e.to_string()))?;
        let src_str: String = row
            .try_get("source_type")
            .map_err(|e| LearningError::StorageError(e.to_string()))?;
        let content: String = row
            .try_get("content")
            .map_err(|e| LearningError::StorageError(e.to_string()))?;
        let mem_uuid: Option<Uuid> = row
            .try_get("memory_id")
            .map_err(|e| LearningError::StorageError(e.to_string()))?;
        let agent: Option<String> = row
            .try_get("agent")
            .map_err(|e| LearningError::StorageError(e.to_string()))?;
        let weight: f32 = row
            .try_get("confidence_weight")
            .map_err(|e| LearningError::StorageError(e.to_string()))?;
        let is_supporting: bool = row
            .try_get("is_supporting")
            .map_err(|e| LearningError::StorageError(e.to_string()))?;
        let recorded_at: DateTime<Utc> = row
            .try_get("recorded_at")
            .map_err(|e| LearningError::StorageError(e.to_string()))?;

        let source_type = EvidenceSourceType::from_str(&src_str)?;

        Ok(Evidence {
            id: EvidenceId::from_uuid(id_uuid),
            candidate_id: CandidateId::from_uuid(cand_uuid),
            source_type,
            content,
            memory_id: mem_uuid.map(MemoryId::from_uuid),
            agent,
            is_supporting,
            confidence_weight: weight,
            recorded_at,
        })
    }

    /// Helper privado para mapear una fila de `learning_candidates` a `CandidateKnowledge`.
    fn map_candidate_row(
        row: &sqlx::postgres::PgRow,
        evidences: Vec<Evidence>,
    ) -> Result<CandidateKnowledge, LearningError> {
        let id_uuid: Uuid = row
            .try_get("id")
            .map_err(|e| LearningError::StorageError(e.to_string()))?;
        let statement: String = row
            .try_get("statement")
            .map_err(|e| LearningError::StorageError(e.to_string()))?;
        let stage_str: String = row
            .try_get("stage")
            .map_err(|e| LearningError::StorageError(e.to_string()))?;
        let conf_val: f32 = row
            .try_get("confidence")
            .map_err(|e| LearningError::StorageError(e.to_string()))?;
        let domain: Option<String> = row
            .try_get("domain")
            .map_err(|e| LearningError::StorageError(e.to_string()))?;
        let human_validated: bool = row
            .try_get("human_validated")
            .map_err(|e| LearningError::StorageError(e.to_string()))?;
        let mem_uuid: Option<Uuid> = row
            .try_get("memory_id")
            .map_err(|e| LearningError::StorageError(e.to_string()))?;
        let metadata: serde_json::Value = row
            .try_get("metadata")
            .map_err(|e| LearningError::StorageError(e.to_string()))?;
        let created_at: DateTime<Utc> = row
            .try_get("created_at")
            .map_err(|e| LearningError::StorageError(e.to_string()))?;
        let updated_at: DateTime<Utc> = row
            .try_get("updated_at")
            .map_err(|e| LearningError::StorageError(e.to_string()))?;

        let stage = LearningStage::from_str(&stage_str)?;
        let confidence =
            Confidence::new(conf_val).map_err(|e| LearningError::OutOfRange(e.to_string()))?;

        Ok(CandidateKnowledge {
            id: CandidateId::from_uuid(id_uuid),
            statement,
            stage,
            confidence,
            domain,
            human_validated,
            memory_id: mem_uuid.map(MemoryId::from_uuid),
            evidences,
            metadata,
            created_at,
            updated_at,
        })
    }
}

#[async_trait]
impl LearningRepository for PostgresLearningRepository {
    async fn save_candidate(&self, candidate: &CandidateKnowledge) -> Result<(), LearningError> {
        let mut tx = self.pool.begin().await.map_err(|e| {
            LearningError::StorageError(format!("Error al iniciar transacción: {e}"))
        })?;

        let mem_uuid = candidate.memory_id.as_ref().map(|m| *m.as_uuid());

        sqlx::query(
            r#"
            INSERT INTO learning_candidates (
                id, statement, stage, confidence, domain, human_validated, memory_id, metadata, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (id) DO UPDATE SET
                statement = EXCLUDED.statement,
                stage = EXCLUDED.stage,
                confidence = EXCLUDED.confidence,
                domain = EXCLUDED.domain,
                human_validated = EXCLUDED.human_validated,
                memory_id = EXCLUDED.memory_id,
                metadata = EXCLUDED.metadata,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(candidate.id.as_uuid())
        .bind(&candidate.statement)
        .bind(candidate.stage.as_str())
        .bind(candidate.confidence.value())
        .bind(&candidate.domain)
        .bind(candidate.human_validated)
        .bind(mem_uuid)
        .bind(&candidate.metadata)
        .bind(candidate.created_at)
        .bind(candidate.updated_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| LearningError::StorageError(format!("Error al insertar candidato: {e}")))?;

        for ev in &candidate.evidences {
            let ev_mem_uuid = ev.memory_id.as_ref().map(|m| *m.as_uuid());
            sqlx::query(
                r#"
                INSERT INTO learning_evidence (
                    id, candidate_id, source_type, content, memory_id, agent, confidence_weight, is_supporting, metadata, recorded_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                ON CONFLICT (id) DO NOTHING
                "#,
            )
            .bind(ev.id.as_uuid())
            .bind(candidate.id.as_uuid())
            .bind(ev.source_type.as_str())
            .bind(&ev.content)
            .bind(ev_mem_uuid)
            .bind(&ev.agent)
            .bind(ev.confidence_weight)
            .bind(ev.is_supporting)
            .bind(serde_json::json!({}))
            .bind(ev.recorded_at)
            .execute(&mut *tx)
            .await
            .map_err(|e| LearningError::StorageError(format!("Error al insertar evidencia: {e}")))?;
        }

        tx.commit().await.map_err(|e| {
            LearningError::StorageError(format!("Error al confirmar transacción: {e}"))
        })?;

        Ok(())
    }

    async fn find_candidate_by_id(
        &self,
        id: &CandidateId,
    ) -> Result<Option<CandidateKnowledge>, LearningError> {
        let row_opt = sqlx::query(
            r#"
            SELECT id, statement, stage, confidence, domain, human_validated, memory_id, metadata, created_at, updated_at
            FROM learning_candidates
            WHERE id = $1
            "#,
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| LearningError::StorageError(format!("Error al buscar candidato por ID: {e}")))?;

        let row = match row_opt {
            Some(r) => r,
            None => return Ok(None),
        };

        let evidences = self.get_evidences(id).await?;
        let candidate = Self::map_candidate_row(&row, evidences)?;

        Ok(Some(candidate))
    }

    async fn find_candidates_by_stage(
        &self,
        stage: LearningStage,
        limit: usize,
    ) -> Result<Vec<CandidateKnowledge>, LearningError> {
        let rows = sqlx::query(
            r#"
            SELECT id, statement, stage, confidence, domain, human_validated, memory_id, metadata, created_at, updated_at
            FROM learning_candidates
            WHERE stage = $1
            ORDER BY updated_at DESC
            LIMIT $2
            "#,
        )
        .bind(stage.as_str())
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| LearningError::StorageError(format!("Error al listar candidatos por etapa: {e}")))?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            let id_uuid: Uuid = row
                .try_get("id")
                .map_err(|e| LearningError::StorageError(e.to_string()))?;
            let cand_id = CandidateId::from_uuid(id_uuid);
            let evidences = self.get_evidences(&cand_id).await?;
            results.push(Self::map_candidate_row(&row, evidences)?);
        }

        Ok(results)
    }

    async fn find_candidates_by_domain(
        &self,
        domain: &str,
        limit: usize,
    ) -> Result<Vec<CandidateKnowledge>, LearningError> {
        let rows = sqlx::query(
            r#"
            SELECT id, statement, stage, confidence, domain, human_validated, memory_id, metadata, created_at, updated_at
            FROM learning_candidates
            WHERE lower(domain) = lower($1)
            ORDER BY confidence DESC, updated_at DESC
            LIMIT $2
            "#,
        )
        .bind(domain)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| LearningError::StorageError(format!("Error al buscar candidatos por dominio: {e}")))?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            let id_uuid: Uuid = row
                .try_get("id")
                .map_err(|e| LearningError::StorageError(e.to_string()))?;
            let cand_id = CandidateId::from_uuid(id_uuid);
            let evidences = self.get_evidences(&cand_id).await?;
            results.push(Self::map_candidate_row(&row, evidences)?);
        }

        Ok(results)
    }

    async fn search_candidates(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<CandidateKnowledge>, LearningError> {
        let pattern = format!("%{query}%");
        let rows = sqlx::query(
            r#"
            SELECT id, statement, stage, confidence, domain, human_validated, memory_id, metadata, created_at, updated_at
            FROM learning_candidates
            WHERE statement ILIKE $1 OR domain ILIKE $1
            ORDER BY confidence DESC, updated_at DESC
            LIMIT $2
            "#,
        )
        .bind(pattern)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| LearningError::StorageError(format!("Error al buscar candidatos: {e}")))?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            let id_uuid: Uuid = row
                .try_get("id")
                .map_err(|e| LearningError::StorageError(e.to_string()))?;
            let cand_id = CandidateId::from_uuid(id_uuid);
            let evidences = self.get_evidences(&cand_id).await?;
            results.push(Self::map_candidate_row(&row, evidences)?);
        }

        Ok(results)
    }

    async fn add_evidence(
        &self,
        candidate_id: &CandidateId,
        evidence: &Evidence,
    ) -> Result<(), LearningError> {
        let ev_mem_uuid = evidence.memory_id.as_ref().map(|m| *m.as_uuid());

        sqlx::query(
            r#"
            INSERT INTO learning_evidence (
                id, candidate_id, source_type, content, memory_id, agent, confidence_weight, is_supporting, metadata, recorded_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(evidence.id.as_uuid())
        .bind(candidate_id.as_uuid())
        .bind(evidence.source_type.as_str())
        .bind(&evidence.content)
        .bind(ev_mem_uuid)
        .bind(&evidence.agent)
        .bind(evidence.confidence_weight)
        .bind(evidence.is_supporting)
        .bind(serde_json::json!({}))
        .bind(evidence.recorded_at)
        .execute(&self.pool)
        .await
        .map_err(|e| LearningError::StorageError(format!("Error al añadir evidencia: {e}")))?;

        Ok(())
    }

    async fn get_evidences(
        &self,
        candidate_id: &CandidateId,
    ) -> Result<Vec<Evidence>, LearningError> {
        let rows = sqlx::query(
            r#"
            SELECT id, candidate_id, source_type, content, memory_id, agent, confidence_weight, is_supporting, metadata, recorded_at
            FROM learning_evidence
            WHERE candidate_id = $1
            ORDER BY recorded_at ASC
            "#,
        )
        .bind(candidate_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| LearningError::StorageError(format!("Error al obtener evidencias: {e}")))?;

        let mut list = Vec::with_capacity(rows.len());
        for row in &rows {
            list.push(Self::map_evidence_row(row)?);
        }

        Ok(list)
    }

    async fn update_candidate(&self, candidate: &CandidateKnowledge) -> Result<(), LearningError> {
        let mem_uuid = candidate.memory_id.as_ref().map(|m| *m.as_uuid());

        let res = sqlx::query(
            r#"
            UPDATE learning_candidates SET
                statement = $2,
                stage = $3,
                confidence = $4,
                domain = $5,
                human_validated = $6,
                memory_id = $7,
                metadata = $8,
                updated_at = $9
            WHERE id = $1
            "#,
        )
        .bind(candidate.id.as_uuid())
        .bind(&candidate.statement)
        .bind(candidate.stage.as_str())
        .bind(candidate.confidence.value())
        .bind(&candidate.domain)
        .bind(candidate.human_validated)
        .bind(mem_uuid)
        .bind(&candidate.metadata)
        .bind(candidate.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| LearningError::StorageError(format!("Error al actualizar candidato: {e}")))?;

        if res.rows_affected() == 0 {
            return Err(LearningError::NotFound(candidate.id.to_string()));
        }

        Ok(())
    }

    async fn delete_candidate(&self, id: &CandidateId) -> Result<(), LearningError> {
        let res = sqlx::query(
            r#"
            DELETE FROM learning_candidates WHERE id = $1
            "#,
        )
        .bind(id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(|e| LearningError::StorageError(format!("Error al eliminar candidato: {e}")))?;

        if res.rows_affected() == 0 {
            return Err(LearningError::NotFound(id.to_string()));
        }

        Ok(())
    }
}
