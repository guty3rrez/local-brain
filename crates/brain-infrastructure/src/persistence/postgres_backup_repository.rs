//! Adaptador de persistencia PostgreSQL para Respaldo y Restauración (SRS §52, §53, §30).
//!
//! Permite la extracción masiva y la restauración atómica bajo transacción de todas las entidades
//! cognitivas del sistema respetando estrictamente el orden de claves foráneas.

use async_trait::async_trait;
use sqlx::{PgPool, Row};
use tracing::instrument;

use brain_application::backup_use_cases::{BackupPayload, BackupRepository, RestoreCounts};
use brain_application::ApplicationError;

/// Repositorio de respaldo y restauración implementado sobre PostgreSQL 17.
#[derive(Debug, Clone)]
pub struct PostgresBackupRepository {
    pool: PgPool,
}

impl PostgresBackupRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BackupRepository for PostgresBackupRepository {
    #[instrument(skip(self))]
    async fn export_all(
        &self,
        project: Option<&str>,
        include_embeddings: bool,
    ) -> Result<BackupPayload, ApplicationError> {
        let mut payload = BackupPayload::default();

        // 1. Memorias
        let memory_rows = if let Some(proj) = project {
            sqlx::query(
                r#"
                SELECT 
                    id::text, memory_type, content_text, content_hash, summary,
                    project, agent, importance, confidence, utility, status, version,
                    provenance, metadata, created_at, updated_at, last_retrieved_at, expires_at
                FROM memories
                WHERE project = $1
                ORDER BY created_at ASC
                "#,
            )
            .bind(proj)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query(
                r#"
                SELECT 
                    id::text, memory_type, content_text, content_hash, summary,
                    project, agent, importance, confidence, utility, status, version,
                    provenance, metadata, created_at, updated_at, last_retrieved_at, expires_at
                FROM memories
                ORDER BY created_at ASC
                "#,
            )
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| ApplicationError::Internal(format!("Error exportando memorias: {e}")))?;

        for row in memory_rows {
            let id: String = row.get("id");
            let memory_type: String = row.get("memory_type");
            let content_text: String = row.get("content_text");
            let content_hash: String = row.get("content_hash");
            let summary: Option<String> = row.get("summary");
            let project: Option<String> = row.get("project");
            let agent: Option<String> = row.get("agent");
            let importance: f32 = row.get("importance");
            let confidence: f32 = row.get("confidence");
            let utility: f32 = row.get("utility");
            let status: String = row.get("status");
            let version: i32 = row.get("version");
            let provenance: serde_json::Value = row.get("provenance");
            let metadata: serde_json::Value = row.get("metadata");
            let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
            let updated_at: chrono::DateTime<chrono::Utc> = row.get("updated_at");
            let last_retrieved_at: Option<chrono::DateTime<chrono::Utc>> =
                row.get("last_retrieved_at");
            let expires_at: Option<chrono::DateTime<chrono::Utc>> = row.get("expires_at");

            payload.memories.push(serde_json::json!({
                "id": id,
                "memory_type": memory_type,
                "content_text": content_text,
                "content_hash": content_hash,
                "summary": summary,
                "project": project,
                "agent": agent,
                "importance": importance,
                "confidence": confidence,
                "utility": utility,
                "status": status,
                "version": version,
                "provenance": provenance,
                "metadata": metadata,
                "created_at": created_at.to_rfc3339(),
                "updated_at": updated_at.to_rfc3339(),
                "last_retrieved_at": last_retrieved_at.map(|t| t.to_rfc3339()),
                "expires_at": expires_at.map(|t| t.to_rfc3339()),
            }));
        }

        // 2. Embeddings (si se solicitaron)
        if include_embeddings {
            let embedding_rows = if let Some(proj) = project {
                sqlx::query(
                    r#"
                    SELECT id::text, embedding::text AS emb_text
                    FROM memories
                    WHERE embedding IS NOT NULL AND project = $1
                    "#,
                )
                .bind(proj)
                .fetch_all(&self.pool)
                .await
            } else {
                sqlx::query(
                    r#"
                    SELECT id::text, embedding::text AS emb_text
                    FROM memories
                    WHERE embedding IS NOT NULL
                    "#,
                )
                .fetch_all(&self.pool)
                .await
            }
            .map_err(|e| ApplicationError::Internal(format!("Error exportando embeddings: {e}")))?;

            for row in embedding_rows {
                let id: String = row.get("id");
                let emb_text: String = row.get("emb_text");
                // Parsear formato "[0.1, 0.2, ...]"
                let clean = emb_text.trim_matches(|c| c == '[' || c == ']');
                let floats: Vec<f32> = clean
                    .split(',')
                    .filter_map(|s| s.trim().parse::<f32>().ok())
                    .collect();

                payload.embeddings.push(serde_json::json!({
                    "memory_id": id,
                    "vector": floats,
                }));
            }
        }

        // 3. Nodos del Grafo
        let node_rows = sqlx::query(
            r#"
            SELECT id::text, node_type, label, memory_id::text, metadata, created_at, updated_at
            FROM graph_nodes
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Internal(format!("Error exportando nodos de grafo: {e}")))?;

        for row in node_rows {
            let id: String = row.get("id");
            let node_type: String = row.get("node_type");
            let label: String = row.get("label");
            let memory_id: Option<String> = row.get("memory_id");
            let metadata: serde_json::Value = row.get("metadata");
            let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
            let updated_at: chrono::DateTime<chrono::Utc> = row.get("updated_at");

            payload.graph_nodes.push(serde_json::json!({
                "id": id,
                "node_type": node_type,
                "label": label,
                "memory_id": memory_id,
                "metadata": metadata,
                "created_at": created_at.to_rfc3339(),
                "updated_at": updated_at.to_rfc3339(),
            }));
        }

        // 4. Aristas del Grafo
        let edge_rows = sqlx::query(
            r#"
            SELECT id::text, source_id::text, target_id::text, relation_type, weight, metadata, created_at, updated_at
            FROM graph_edges
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Internal(format!("Error exportando aristas de grafo: {e}")))?;

        for row in edge_rows {
            let id: String = row.get("id");
            let source_id: String = row.get("source_id");
            let target_id: String = row.get("target_id");
            let relation_type: String = row.get("relation_type");
            let weight: f32 = row.get("weight");
            let metadata: serde_json::Value = row.get("metadata");
            let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
            let updated_at: chrono::DateTime<chrono::Utc> = row.get("updated_at");

            payload.graph_edges.push(serde_json::json!({
                "id": id,
                "source_id": source_id,
                "target_id": target_id,
                "relation_type": relation_type,
                "weight": weight,
                "metadata": metadata,
                "created_at": created_at.to_rfc3339(),
                "updated_at": updated_at.to_rfc3339(),
            }));
        }

        // 5. Candidatos de Aprendizaje
        let candidate_rows = sqlx::query(
            r#"
            SELECT id::text, statement, stage, confidence, domain, human_validated, memory_id::text, metadata, created_at, updated_at
            FROM learning_candidates
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Internal(format!("Error exportando candidatos: {e}")))?;

        for row in candidate_rows {
            let id: String = row.get("id");
            let statement: String = row.get("statement");
            let stage: String = row.get("stage");
            let confidence: f32 = row.get("confidence");
            let domain: Option<String> = row.get("domain");
            let human_validated: bool = row.get("human_validated");
            let memory_id: Option<String> = row.get("memory_id");
            let metadata: serde_json::Value = row.get("metadata");
            let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
            let updated_at: chrono::DateTime<chrono::Utc> = row.get("updated_at");

            payload.learning_candidates.push(serde_json::json!({
                "id": id,
                "statement": statement,
                "stage": stage,
                "confidence": confidence,
                "domain": domain,
                "human_validated": human_validated,
                "memory_id": memory_id,
                "metadata": metadata,
                "created_at": created_at.to_rfc3339(),
                "updated_at": updated_at.to_rfc3339(),
            }));
        }

        // 6. Evidencias de Aprendizaje
        let evidence_rows = sqlx::query(
            r#"
            SELECT id::text, candidate_id::text, source_type, content, memory_id::text, agent, confidence_weight, is_supporting, metadata, recorded_at
            FROM learning_evidence
            ORDER BY recorded_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Internal(format!("Error exportando evidencias: {e}")))?;

        for row in evidence_rows {
            let id: String = row.get("id");
            let candidate_id: String = row.get("candidate_id");
            let source_type: String = row.get("source_type");
            let content: String = row.get("content");
            let memory_id: Option<String> = row.get("memory_id");
            let agent: Option<String> = row.get("agent");
            let confidence_weight: f32 = row.get("confidence_weight");
            let is_supporting: bool = row.get("is_supporting");
            let metadata: serde_json::Value = row.get("metadata");
            let recorded_at: chrono::DateTime<chrono::Utc> = row.get("recorded_at");

            payload.learning_evidences.push(serde_json::json!({
                "id": id,
                "candidate_id": candidate_id,
                "source_type": source_type,
                "content": content,
                "memory_id": memory_id,
                "agent": agent,
                "confidence_weight": confidence_weight,
                "is_supporting": is_supporting,
                "metadata": metadata,
                "recorded_at": recorded_at.to_rfc3339(),
            }));
        }

        // 7. Conflictos Cognitivos
        let conflict_rows = sqlx::query(
            r#"
            SELECT id::text, source_memory_id::text, conflicting_memory_id::text, conflict_type, reason, suggested_resolution, status, resolution_context, detected_at, resolved_at, metadata
            FROM knowledge_conflicts
            ORDER BY detected_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Internal(format!("Error exportando conflictos: {e}")))?;

        for row in conflict_rows {
            let id: String = row.get("id");
            let source_memory_id: String = row.get("source_memory_id");
            let conflicting_memory_id: String = row.get("conflicting_memory_id");
            let conflict_type: String = row.get("conflict_type");
            let reason: String = row.get("reason");
            let suggested_resolution: Option<String> = row.get("suggested_resolution");
            let status: String = row.get("status");
            let resolution_context: Option<String> = row.get("resolution_context");
            let detected_at: chrono::DateTime<chrono::Utc> = row.get("detected_at");
            let resolved_at: Option<chrono::DateTime<chrono::Utc>> = row.get("resolved_at");
            let metadata: serde_json::Value = row.get("metadata");

            payload.knowledge_conflicts.push(serde_json::json!({
                "id": id,
                "source_memory_id": source_memory_id,
                "conflicting_memory_id": conflicting_memory_id,
                "conflict_type": conflict_type,
                "reason": reason,
                "suggested_resolution": suggested_resolution,
                "status": status,
                "resolution_context": resolution_context,
                "detected_at": detected_at.to_rfc3339(),
                "resolved_at": resolved_at.map(|t| t.to_rfc3339()),
                "metadata": metadata,
            }));
        }

        // 8. Sesiones de Consolidación
        let run_rows = sqlx::query(
            r#"
            SELECT id::text, project, memories_analyzed, clusters_count, hypotheses_count, conflicts_count, summary, executed_at
            FROM consolidation_runs
            ORDER BY executed_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Internal(format!("Error exportando corridas de consolidación: {e}")))?;

        for row in run_rows {
            let id: String = row.get("id");
            let project: Option<String> = row.get("project");
            let memories_analyzed: i32 = row.get("memories_analyzed");
            let clusters_count: i32 = row.get("clusters_count");
            let hypotheses_count: i32 = row.get("hypotheses_count");
            let conflicts_count: i32 = row.get("conflicts_count");
            let summary: Option<String> = row.get("summary");
            let executed_at: chrono::DateTime<chrono::Utc> = row.get("executed_at");

            payload.consolidation_runs.push(serde_json::json!({
                "id": id,
                "project": project,
                "memories_analyzed": memories_analyzed,
                "clusters_count": clusters_count,
                "hypotheses_count": hypotheses_count,
                "conflicts_count": conflicts_count,
                "summary": summary,
                "executed_at": executed_at.to_rfc3339(),
            }));
        }

        Ok(payload)
    }

    #[instrument(skip(self, payload))]
    async fn restore_all(
        &self,
        payload: &BackupPayload,
        wipe: bool,
    ) -> Result<RestoreCounts, ApplicationError> {
        let mut tx = self.pool.begin().await.map_err(|e| {
            ApplicationError::Internal(format!("Fallo iniciando transacción SQL: {e}"))
        })?;

        if wipe {
            sqlx::query("TRUNCATE TABLE consolidation_runs, knowledge_conflicts, learning_evidence, learning_candidates, graph_edges, graph_nodes, memories CASCADE")
                .execute(&mut *tx)
                .await
                .map_err(|e| ApplicationError::Internal(format!("Error al ejecutar purga wipe: {e}")))?;
        }

        // Si contiene SQL sin procesar (formato SQL directo)
        if let Some(first_mem) = payload.memories.first() {
            if let Some(sql) = first_mem.get("__raw_sql").and_then(|v| v.as_str()) {
                // Ejecutar sentencias SQL línea por línea o bloques
                for stmt in sql.split(';') {
                    let trimmed = stmt.trim();
                    if trimmed.is_empty()
                        || trimmed.starts_with("--")
                        || trimmed.eq_ignore_ascii_case("BEGIN")
                        || trimmed.eq_ignore_ascii_case("COMMIT")
                    {
                        continue;
                    }
                    sqlx::query(trimmed).execute(&mut *tx).await.map_err(|e| {
                        ApplicationError::Internal(format!(
                            "Error ejecutando sentencia de restore SQL: {e} (SQL: {trimmed})"
                        ))
                    })?;
                }

                tx.commit().await.map_err(|e| {
                    ApplicationError::Internal(format!("Error al confirmar transacción: {e}"))
                })?;

                return Ok(RestoreCounts::default());
            }
        }

        let mut counts = RestoreCounts::default();

        // 1. Restaurar Memorias
        for m in &payload.memories {
            let id = m.get("id").and_then(|v| v.as_str()).unwrap_or_default();
            let mtype = m
                .get("memory_type")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let content = m
                .get("content_text")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let hash = m
                .get("content_hash")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let summary = m.get("summary").and_then(|v| v.as_str());
            let project = m.get("project").and_then(|v| v.as_str());
            let agent = m.get("agent").and_then(|v| v.as_str());
            let importance = m.get("importance").and_then(|v| v.as_f64()).unwrap_or(0.5) as f32;
            let confidence = m.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.5) as f32;
            let utility = m.get("utility").and_then(|v| v.as_f64()).unwrap_or(0.5) as f32;
            let status = m.get("status").and_then(|v| v.as_str()).unwrap_or("active");
            let version = m.get("version").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
            let provenance = m
                .get("provenance")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            let metadata = m
                .get("metadata")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));

            let uid: uuid::Uuid = id.parse().map_err(|e| {
                ApplicationError::Validation(format!("UUID de memoria inválido: {e}"))
            })?;

            sqlx::query(
                r#"
                INSERT INTO memories (
                    id, memory_type, content_text, content_hash, summary,
                    project, agent, importance, confidence, utility, status, version,
                    provenance, metadata
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14
                )
                ON CONFLICT (id) DO UPDATE SET
                    content_text = EXCLUDED.content_text,
                    summary = EXCLUDED.summary,
                    importance = EXCLUDED.importance,
                    confidence = EXCLUDED.confidence,
                    utility = EXCLUDED.utility,
                    status = EXCLUDED.status,
                    version = EXCLUDED.version,
                    provenance = EXCLUDED.provenance,
                    metadata = EXCLUDED.metadata,
                    updated_at = NOW();
                "#,
            )
            .bind(uid)
            .bind(mtype)
            .bind(content)
            .bind(hash)
            .bind(summary)
            .bind(project)
            .bind(agent)
            .bind(importance)
            .bind(confidence)
            .bind(utility)
            .bind(status)
            .bind(version)
            .bind(provenance)
            .bind(metadata)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                ApplicationError::Internal(format!("Error restaurando memoria {id}: {e}"))
            })?;

            counts.memories += 1;
        }

        // 2. Restaurar Embeddings
        for e in &payload.embeddings {
            let id = e
                .get("memory_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            if let Some(arr) = e.get("vector").and_then(|v| v.as_array()) {
                let floats: Vec<f32> = arr
                    .iter()
                    .filter_map(|x| x.as_f64().map(|f| f as f32))
                    .collect();
                if !floats.is_empty() {
                    let uid: uuid::Uuid = id.parse().map_err(|err| {
                        ApplicationError::Validation(format!("UUID inválido: {err}"))
                    })?;
                    let vec_text = format!(
                        "[{}]",
                        floats
                            .iter()
                            .map(|f| f.to_string())
                            .collect::<Vec<_>>()
                            .join(",")
                    );

                    sqlx::query("UPDATE memories SET embedding = $1::vector WHERE id = $2")
                        .bind(vec_text)
                        .bind(uid)
                        .execute(&mut *tx)
                        .await
                        .map_err(|err| {
                            ApplicationError::Internal(format!(
                                "Error restaurando embedding {id}: {err}"
                            ))
                        })?;

                    counts.embeddings += 1;
                }
            }
        }

        // 3. Restaurar Nodos del Grafo
        for n in &payload.graph_nodes {
            let id = n.get("id").and_then(|v| v.as_str()).unwrap_or_default();
            let node_type = n
                .get("node_type")
                .and_then(|v| v.as_str())
                .unwrap_or("concept");
            let label = n.get("label").and_then(|v| v.as_str()).unwrap_or_default();
            let memory_id = n
                .get("memory_id")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<uuid::Uuid>().ok());
            let metadata = n
                .get("metadata")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));

            let uid: uuid::Uuid = id
                .parse()
                .map_err(|e| ApplicationError::Validation(format!("UUID de nodo inválido: {e}")))?;

            sqlx::query(
                r#"
                INSERT INTO graph_nodes (id, node_type, label, memory_id, metadata)
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (id) DO NOTHING;
                "#,
            )
            .bind(uid)
            .bind(node_type)
            .bind(label)
            .bind(memory_id)
            .bind(metadata)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                ApplicationError::Internal(format!("Error restaurando nodo de grafo {id}: {e}"))
            })?;

            counts.graph_nodes += 1;
        }

        // 4. Restaurar Aristas del Grafo
        for ed in &payload.graph_edges {
            let id = ed.get("id").and_then(|v| v.as_str()).unwrap_or_default();
            let source_id = ed
                .get("source_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let target_id = ed
                .get("target_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let relation = ed
                .get("relation_type")
                .and_then(|v| v.as_str())
                .unwrap_or("RELATED_TO");
            let weight = ed.get("weight").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
            let metadata = ed
                .get("metadata")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));

            let uid: uuid::Uuid = id.parse().map_err(|e| {
                ApplicationError::Validation(format!("UUID de arista inválido: {e}"))
            })?;
            let src_uid: uuid::Uuid = source_id
                .parse()
                .map_err(|e| ApplicationError::Validation(format!("Source UUID inválido: {e}")))?;
            let tgt_uid: uuid::Uuid = target_id
                .parse()
                .map_err(|e| ApplicationError::Validation(format!("Target UUID inválido: {e}")))?;

            sqlx::query(
                r#"
                INSERT INTO graph_edges (id, source_id, target_id, relation_type, weight, metadata)
                VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (source_id, target_id, relation_type) DO UPDATE SET
                    weight = EXCLUDED.weight,
                    metadata = EXCLUDED.metadata;
                "#,
            )
            .bind(uid)
            .bind(src_uid)
            .bind(tgt_uid)
            .bind(relation)
            .bind(weight)
            .bind(metadata)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                ApplicationError::Internal(format!("Error restaurando arista de grafo {id}: {e}"))
            })?;

            counts.graph_edges += 1;
        }

        // 5. Restaurar Candidatos de Aprendizaje
        for c in &payload.learning_candidates {
            let id = c.get("id").and_then(|v| v.as_str()).unwrap_or_default();
            let statement = c
                .get("statement")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let stage = c
                .get("stage")
                .and_then(|v| v.as_str())
                .unwrap_or("observation");
            let confidence = c.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.5) as f32;
            let domain = c.get("domain").and_then(|v| v.as_str());
            let human_validated = c
                .get("human_validated")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let memory_id = c
                .get("memory_id")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<uuid::Uuid>().ok());
            let metadata = c
                .get("metadata")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));

            let uid: uuid::Uuid = id.parse().map_err(|e| {
                ApplicationError::Validation(format!("UUID candidato inválido: {e}"))
            })?;

            sqlx::query(
                r#"
                INSERT INTO learning_candidates (id, statement, stage, confidence, domain, human_validated, memory_id, metadata)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                ON CONFLICT (id) DO NOTHING;
                "#,
            )
            .bind(uid)
            .bind(statement)
            .bind(stage)
            .bind(confidence)
            .bind(domain)
            .bind(human_validated)
            .bind(memory_id)
            .bind(metadata)
            .execute(&mut *tx)
            .await
            .map_err(|e| ApplicationError::Internal(format!("Error restaurando candidato {id}: {e}")))?;

            counts.learning_candidates += 1;
        }

        // 6. Restaurar Evidencias de Aprendizaje
        for ev in &payload.learning_evidences {
            let id = ev.get("id").and_then(|v| v.as_str()).unwrap_or_default();
            let cand_id = ev
                .get("candidate_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let source_type = ev
                .get("source_type")
                .and_then(|v| v.as_str())
                .unwrap_or("direct-observation");
            let content = ev
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let memory_id = ev
                .get("memory_id")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<uuid::Uuid>().ok());
            let agent = ev.get("agent").and_then(|v| v.as_str());
            let conf_weight = ev
                .get("confidence_weight")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.5) as f32;
            let is_supporting = ev
                .get("is_supporting")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let metadata = ev
                .get("metadata")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));

            let uid: uuid::Uuid = id.parse().map_err(|e| {
                ApplicationError::Validation(format!("UUID evidencia inválido: {e}"))
            })?;
            let c_uid: uuid::Uuid = cand_id.parse().map_err(|e| {
                ApplicationError::Validation(format!("Candidate UUID inválido: {e}"))
            })?;

            sqlx::query(
                r#"
                INSERT INTO learning_evidence (id, candidate_id, source_type, content, memory_id, agent, confidence_weight, is_supporting, metadata)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                ON CONFLICT (id) DO NOTHING;
                "#,
            )
            .bind(uid)
            .bind(c_uid)
            .bind(source_type)
            .bind(content)
            .bind(memory_id)
            .bind(agent)
            .bind(conf_weight)
            .bind(is_supporting)
            .bind(metadata)
            .execute(&mut *tx)
            .await
            .map_err(|e| ApplicationError::Internal(format!("Error restaurando evidencia {id}: {e}")))?;

            counts.learning_evidences += 1;
        }

        // 7. Restaurar Conflictos Cognitivos
        for k in &payload.knowledge_conflicts {
            let id = k.get("id").and_then(|v| v.as_str()).unwrap_or_default();
            let src_m_id = k
                .get("source_memory_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let tgt_m_id = k
                .get("conflicting_memory_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let ctype = k
                .get("conflict_type")
                .and_then(|v| v.as_str())
                .unwrap_or("contradiction");
            let reason = k.get("reason").and_then(|v| v.as_str()).unwrap_or_default();
            let sug_res = k.get("suggested_resolution").and_then(|v| v.as_str());
            let status = k
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("pending");
            let res_ctx = k.get("resolution_context").and_then(|v| v.as_str());
            let metadata = k
                .get("metadata")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));

            let uid: uuid::Uuid = id.parse().map_err(|e| {
                ApplicationError::Validation(format!("UUID conflicto inválido: {e}"))
            })?;
            let src_uid: uuid::Uuid = src_m_id
                .parse()
                .map_err(|e| ApplicationError::Validation(format!("UUID origen inválido: {e}")))?;
            let tgt_uid: uuid::Uuid = tgt_m_id
                .parse()
                .map_err(|e| ApplicationError::Validation(format!("UUID destino inválido: {e}")))?;

            sqlx::query(
                r#"
                INSERT INTO knowledge_conflicts (id, source_memory_id, conflicting_memory_id, conflict_type, reason, suggested_resolution, status, resolution_context, metadata)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                ON CONFLICT (id) DO NOTHING;
                "#,
            )
            .bind(uid)
            .bind(src_uid)
            .bind(tgt_uid)
            .bind(ctype)
            .bind(reason)
            .bind(sug_res)
            .bind(status)
            .bind(res_ctx)
            .bind(metadata)
            .execute(&mut *tx)
            .await
            .map_err(|e| ApplicationError::Internal(format!("Error restaurando conflicto {id}: {e}")))?;

            counts.conflicts += 1;
        }

        // 8. Restaurar Corridas de Consolidación
        for r in &payload.consolidation_runs {
            let id = r.get("id").and_then(|v| v.as_str()).unwrap_or_default();
            let project = r.get("project").and_then(|v| v.as_str());
            let mems = r
                .get("memories_analyzed")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;
            let clusters = r
                .get("clusters_count")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;
            let hyps = r
                .get("hypotheses_count")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;
            let confs = r
                .get("conflicts_count")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;
            let summary = r.get("summary").and_then(|v| v.as_str());

            let uid: uuid::Uuid = id
                .parse()
                .map_err(|e| ApplicationError::Validation(format!("UUID corrida inválido: {e}")))?;

            sqlx::query(
                r#"
                INSERT INTO consolidation_runs (id, project, memories_analyzed, clusters_count, hypotheses_count, conflicts_count, summary)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT (id) DO NOTHING;
                "#,
            )
            .bind(uid)
            .bind(project)
            .bind(mems)
            .bind(clusters)
            .bind(hyps)
            .bind(confs)
            .bind(summary)
            .execute(&mut *tx)
            .await
            .map_err(|e| ApplicationError::Internal(format!("Error restaurando corrida {id}: {e}")))?;

            counts.consolidation_runs += 1;
        }

        tx.commit().await.map_err(|e| {
            ApplicationError::Internal(format!("Error confirmando transacción de restore: {e}"))
        })?;

        Ok(counts)
    }
}
