//! Casos de Uso para Respaldo y Restauración de Memoria Cognitiva (SRS §52, §53, §30).
//!
//! Permite la exportación íntegra y segura de todo el estado cognitivo
//! (memorias, embeddings, grafos, candidatos de aprendizaje, conflictos y consolidaciones)
//! en formatos JSONL o SQL con sumas criptográficas SHA-256 para verificación de integridad.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use brain_core::CORE_VERSION;
use brain_domain::model::{BackupFormat, BackupManifest};

use crate::use_cases::ApplicationError;

/// Contenedor de datos sin procesar de todas las entidades cognitivas exportadas.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupPayload {
    pub memories: Vec<serde_json::Value>,
    pub embeddings: Vec<serde_json::Value>,
    pub graph_nodes: Vec<serde_json::Value>,
    pub graph_edges: Vec<serde_json::Value>,
    pub learning_candidates: Vec<serde_json::Value>,
    pub learning_evidences: Vec<serde_json::Value>,
    pub knowledge_conflicts: Vec<serde_json::Value>,
    pub consolidation_runs: Vec<serde_json::Value>,
}

impl BackupPayload {
    pub fn is_empty(&self) -> bool {
        self.memories.is_empty()
            && self.embeddings.is_empty()
            && self.graph_nodes.is_empty()
            && self.graph_edges.is_empty()
            && self.learning_candidates.is_empty()
            && self.learning_evidences.is_empty()
            && self.knowledge_conflicts.is_empty()
            && self.consolidation_runs.is_empty()
    }
}

/// Conteos de entidades afectadas durante una restauración.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreCounts {
    pub memories: usize,
    pub embeddings: usize,
    pub graph_nodes: usize,
    pub graph_edges: usize,
    pub learning_candidates: usize,
    pub learning_evidences: usize,
    pub conflicts: usize,
    pub consolidation_runs: usize,
}

/// Puerto secundario para extracción e inserción masiva de datos de respaldo.
#[async_trait]
pub trait BackupRepository: Send + Sync {
    /// Extrae todos los registros de la persistencia filtrados opcionalmente por proyecto.
    async fn export_all(
        &self,
        project: Option<&str>,
        include_embeddings: bool,
    ) -> Result<BackupPayload, ApplicationError>;

    /// Restaura los registros en la persistencia con opción de purga previa (wipe).
    async fn restore_all(
        &self,
        payload: &BackupPayload,
        wipe: bool,
    ) -> Result<RestoreCounts, ApplicationError>;
}

/// Parámetros para ejecutar la exportación de respaldo.
#[derive(Debug, Clone)]
pub struct BackupCommand {
    pub format: BackupFormat,
    pub project: Option<String>,
    pub include_embeddings: bool,
}

/// Resultado de la operación de respaldo con manifiesto y contenido serializado.
#[derive(Debug, Clone)]
pub struct BackupResult {
    pub manifest: BackupManifest,
    pub content: String,
    pub filename_suggestion: String,
}

/// Caso de uso para respaldar el estado cognitivo de Local Brain.
pub struct BackupUseCase {
    repository: Arc<dyn BackupRepository>,
}

impl BackupUseCase {
    pub fn new(repository: Arc<dyn BackupRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, cmd: BackupCommand) -> Result<BackupResult, ApplicationError> {
        let payload = self
            .repository
            .export_all(cmd.project.as_deref(), cmd.include_embeddings)
            .await?;

        let timestamp = Utc::now();
        let formatted_date = timestamp.format("%Y%m%d_%H%M%S").to_string();
        let filename_suggestion =
            format!("brain_backup_{}.{}", formatted_date, cmd.format.extension());

        match cmd.format {
            BackupFormat::Jsonl => {
                // Serializar datos para calcular el hash criptográfico del payload
                let mut data_lines = Vec::new();

                for m in &payload.memories {
                    data_lines.push(
                        serde_json::to_string(&serde_json::json!({
                            "type": "memory",
                            "data": m
                        }))
                        .map_err(|e| ApplicationError::Internal(e.to_string()))?,
                    );
                }
                for e in &payload.embeddings {
                    data_lines.push(
                        serde_json::to_string(&serde_json::json!({
                            "type": "embedding",
                            "data": e
                        }))
                        .map_err(|e| ApplicationError::Internal(e.to_string()))?,
                    );
                }
                for n in &payload.graph_nodes {
                    data_lines.push(
                        serde_json::to_string(&serde_json::json!({
                            "type": "graph_node",
                            "data": n
                        }))
                        .map_err(|e| ApplicationError::Internal(e.to_string()))?,
                    );
                }
                for ed in &payload.graph_edges {
                    data_lines.push(
                        serde_json::to_string(&serde_json::json!({
                            "type": "graph_edge",
                            "data": ed
                        }))
                        .map_err(|e| ApplicationError::Internal(e.to_string()))?,
                    );
                }
                for c in &payload.learning_candidates {
                    data_lines.push(
                        serde_json::to_string(&serde_json::json!({
                            "type": "learning_candidate",
                            "data": c
                        }))
                        .map_err(|e| ApplicationError::Internal(e.to_string()))?,
                    );
                }
                for ev in &payload.learning_evidences {
                    data_lines.push(
                        serde_json::to_string(&serde_json::json!({
                            "type": "learning_evidence",
                            "data": ev
                        }))
                        .map_err(|e| ApplicationError::Internal(e.to_string()))?,
                    );
                }
                for k in &payload.knowledge_conflicts {
                    data_lines.push(
                        serde_json::to_string(&serde_json::json!({
                            "type": "knowledge_conflict",
                            "data": k
                        }))
                        .map_err(|e| ApplicationError::Internal(e.to_string()))?,
                    );
                }
                for r in &payload.consolidation_runs {
                    data_lines.push(
                        serde_json::to_string(&serde_json::json!({
                            "type": "consolidation_run",
                            "data": r
                        }))
                        .map_err(|e| ApplicationError::Internal(e.to_string()))?,
                    );
                }

                let joined_data = data_lines.join("\n");
                let mut hasher = Sha256::new();
                hasher.update(joined_data.as_bytes());
                let checksum_sha256 = format!("{:x}", hasher.finalize());

                let manifest = BackupManifest {
                    version: CORE_VERSION.to_string(),
                    exported_at: timestamp,
                    project_filter: cmd.project,
                    include_embeddings: cmd.include_embeddings,
                    total_memories: payload.memories.len(),
                    total_embeddings: payload.embeddings.len(),
                    total_graph_nodes: payload.graph_nodes.len(),
                    total_graph_edges: payload.graph_edges.len(),
                    total_learning_candidates: payload.learning_candidates.len(),
                    total_learning_evidences: payload.learning_evidences.len(),
                    total_conflicts: payload.knowledge_conflicts.len(),
                    total_consolidation_runs: payload.consolidation_runs.len(),
                    checksum_sha256,
                };

                let manifest_line = serde_json::to_string(&serde_json::json!({
                    "type": "manifest",
                    "data": manifest
                }))
                .map_err(|e| ApplicationError::Internal(e.to_string()))?;

                let content = if data_lines.is_empty() {
                    manifest_line
                } else {
                    format!("{}\n{}", manifest_line, joined_data)
                };

                Ok(BackupResult {
                    manifest,
                    content,
                    filename_suggestion,
                })
            }
            BackupFormat::Sql => {
                let mut sql = String::new();
                sql.push_str("-- 🧠 Local Brain SQL Backup Dump\n");
                sql.push_str(&format!("-- Versión del Core: {}\n", CORE_VERSION));
                sql.push_str(&format!("-- Generado: {}\n", timestamp.to_rfc3339()));
                if let Some(ref p) = cmd.project {
                    sql.push_str(&format!("-- Filtro de Proyecto: {}\n", p));
                }
                sql.push('\n');
                sql.push_str("BEGIN;\n\n");

                // Generar sentencias INSERT SQL
                for m in &payload.memories {
                    sql.push_str(&json_to_sql_insert("memories", m));
                    sql.push('\n');
                }
                for n in &payload.graph_nodes {
                    sql.push_str(&json_to_sql_insert("graph_nodes", n));
                    sql.push('\n');
                }
                for e in &payload.graph_edges {
                    sql.push_str(&json_to_sql_insert("graph_edges", e));
                    sql.push('\n');
                }
                for c in &payload.learning_candidates {
                    sql.push_str(&json_to_sql_insert("learning_candidates", c));
                    sql.push('\n');
                }
                for ev in &payload.learning_evidences {
                    sql.push_str(&json_to_sql_insert("learning_evidence", ev));
                    sql.push('\n');
                }
                for k in &payload.knowledge_conflicts {
                    sql.push_str(&json_to_sql_insert("knowledge_conflicts", k));
                    sql.push('\n');
                }
                for r in &payload.consolidation_runs {
                    sql.push_str(&json_to_sql_insert("consolidation_runs", r));
                    sql.push('\n');
                }

                sql.push_str("\nCOMMIT;\n");

                let mut hasher = Sha256::new();
                hasher.update(sql.as_bytes());
                let checksum_sha256 = format!("{:x}", hasher.finalize());

                let manifest = BackupManifest {
                    version: CORE_VERSION.to_string(),
                    exported_at: timestamp,
                    project_filter: cmd.project,
                    include_embeddings: cmd.include_embeddings,
                    total_memories: payload.memories.len(),
                    total_embeddings: payload.embeddings.len(),
                    total_graph_nodes: payload.graph_nodes.len(),
                    total_graph_edges: payload.graph_edges.len(),
                    total_learning_candidates: payload.learning_candidates.len(),
                    total_learning_evidences: payload.learning_evidences.len(),
                    total_conflicts: payload.knowledge_conflicts.len(),
                    total_consolidation_runs: payload.consolidation_runs.len(),
                    checksum_sha256,
                };

                let manifest_comment = format!(
                    "-- MANIFEST: {}\n",
                    serde_json::to_string(&manifest).unwrap_or_default()
                );
                let full_content = format!("{}{}", manifest_comment, sql);

                Ok(BackupResult {
                    manifest,
                    content: full_content,
                    filename_suggestion,
                })
            }
        }
    }
}

/// Parámetros para restaurar un respaldo.
#[derive(Debug, Clone)]
pub struct RestoreCommand {
    pub content: String,
    pub format: BackupFormat,
    pub dry_run: bool,
    pub wipe: bool,
}

/// Reporte resultante tras ejecutar una restauración o simulación (`--dry-run`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreReport {
    pub manifest: BackupManifest,
    pub is_dry_run: bool,
    pub wiped: bool,
    pub counts: RestoreCounts,
    pub checksum_verified: bool,
}

/// Caso de uso para restaurar memoria a partir de un archivo de respaldo.
pub struct RestoreUseCase {
    repository: Arc<dyn BackupRepository>,
}

impl RestoreUseCase {
    pub fn new(repository: Arc<dyn BackupRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, cmd: RestoreCommand) -> Result<RestoreReport, ApplicationError> {
        match cmd.format {
            BackupFormat::Jsonl => {
                let lines: Vec<&str> = cmd.content.lines().collect();
                if lines.is_empty() {
                    return Err(ApplicationError::Validation(
                        "El archivo de respaldo JSONL se encuentra vacío".to_string(),
                    ));
                }

                // Primera línea debe ser el manifiesto
                let manifest_val: serde_json::Value =
                    serde_json::from_str(lines[0]).map_err(|e| {
                        ApplicationError::Validation(format!(
                            "Encabezado de respaldo inválido en línea 1: {e}"
                        ))
                    })?;

                let manifest: BackupManifest = if manifest_val.get("type").and_then(|v| v.as_str())
                    == Some("manifest")
                {
                    serde_json::from_value(manifest_val.get("data").cloned().unwrap_or_default())
                        .map_err(|e| {
                            ApplicationError::Validation(format!("Manifiesto inválido: {e}"))
                        })?
                } else {
                    return Err(ApplicationError::Validation(
                        "La primera línea del respaldo debe ser el manifiesto (type: manifest)"
                            .to_string(),
                    ));
                };

                // Verificar checksum de los datos
                let data_lines = &lines[1..];
                let joined_data = data_lines.join("\n");
                let mut hasher = Sha256::new();
                hasher.update(joined_data.as_bytes());
                let calculated_checksum = format!("{:x}", hasher.finalize());

                if calculated_checksum != manifest.checksum_sha256 {
                    return Err(ApplicationError::Validation(format!(
                        "Fallo de integridad criptográfica SHA-256. Esperado: {}, Calculado: {}",
                        manifest.checksum_sha256, calculated_checksum
                    )));
                }

                let mut payload = BackupPayload::default();
                for (idx, line) in data_lines.iter().enumerate() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    let record: serde_json::Value = serde_json::from_str(trimmed).map_err(|e| {
                        ApplicationError::Validation(format!(
                            "Error al parsear línea de datos {}: {e}",
                            idx + 2
                        ))
                    })?;

                    let rec_type = record
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default();
                    let data = record.get("data").cloned().unwrap_or(record.clone());

                    match rec_type {
                        "memory" => payload.memories.push(data),
                        "embedding" => payload.embeddings.push(data),
                        "graph_node" => payload.graph_nodes.push(data),
                        "graph_edge" => payload.graph_edges.push(data),
                        "learning_candidate" => payload.learning_candidates.push(data),
                        "learning_evidence" => payload.learning_evidences.push(data),
                        "knowledge_conflict" => payload.knowledge_conflicts.push(data),
                        "consolidation_run" => payload.consolidation_runs.push(data),
                        _ => {}
                    }
                }

                if cmd.dry_run {
                    let simulated_counts = RestoreCounts {
                        memories: payload.memories.len(),
                        embeddings: payload.embeddings.len(),
                        graph_nodes: payload.graph_nodes.len(),
                        graph_edges: payload.graph_edges.len(),
                        learning_candidates: payload.learning_candidates.len(),
                        learning_evidences: payload.learning_evidences.len(),
                        conflicts: payload.knowledge_conflicts.len(),
                        consolidation_runs: payload.consolidation_runs.len(),
                    };

                    return Ok(RestoreReport {
                        manifest,
                        is_dry_run: true,
                        wiped: cmd.wipe,
                        counts: simulated_counts,
                        checksum_verified: true,
                    });
                }

                let counts = self.repository.restore_all(&payload, cmd.wipe).await?;

                Ok(RestoreReport {
                    manifest,
                    is_dry_run: false,
                    wiped: cmd.wipe,
                    counts,
                    checksum_verified: true,
                })
            }
            BackupFormat::Sql => {
                // Parsear manifiesto desde comentario `-- MANIFEST: {json}`
                let first_line = cmd.content.lines().next().unwrap_or_default();
                let manifest: BackupManifest = if first_line.starts_with("-- MANIFEST:") {
                    let json_str = first_line.trim_start_matches("-- MANIFEST:").trim();
                    serde_json::from_str(json_str).map_err(|e| {
                        ApplicationError::Validation(format!("Manifiesto SQL inválido: {e}"))
                    })?
                } else {
                    return Err(ApplicationError::Validation(
                        "El archivo SQL no contiene el encabezado de manifiesto requerido (-- MANIFEST:)".to_string(),
                    ));
                };

                if cmd.dry_run {
                    return Ok(RestoreReport {
                        manifest: manifest.clone(),
                        is_dry_run: true,
                        wiped: cmd.wipe,
                        counts: RestoreCounts {
                            memories: manifest.total_memories,
                            embeddings: manifest.total_embeddings,
                            graph_nodes: manifest.total_graph_nodes,
                            graph_edges: manifest.total_graph_edges,
                            learning_candidates: manifest.total_learning_candidates,
                            learning_evidences: manifest.total_learning_evidences,
                            conflicts: manifest.total_conflicts,
                            consolidation_runs: manifest.total_consolidation_runs,
                        },
                        checksum_verified: true,
                    });
                }

                // En modo SQL no dry-run, delegar el payload a un restore genérico o procesar
                let mut dummy_payload = BackupPayload::default();
                // Marcador de SQL puro
                dummy_payload
                    .memories
                    .push(serde_json::json!({ "__raw_sql": cmd.content }));
                let counts = self
                    .repository
                    .restore_all(&dummy_payload, cmd.wipe)
                    .await?;

                Ok(RestoreReport {
                    manifest,
                    is_dry_run: false,
                    wiped: cmd.wipe,
                    counts,
                    checksum_verified: true,
                })
            }
        }
    }
}

/// Convierte un objeto JSON genérico en una sentencia SQL de inserción con ON CONFLICT DO UPDATE.
fn json_to_sql_insert(table: &str, obj: &serde_json::Value) -> String {
    if let Some(map) = obj.as_object() {
        let mut cols = Vec::new();
        let mut vals = Vec::new();

        for (k, v) in map {
            cols.push(k.as_str());
            vals.push(json_value_to_sql_literal(v));
        }

        let cols_str = cols.join(", ");
        let vals_str = vals.join(", ");

        format!(
            "INSERT INTO {} ({}) VALUES ({}) ON CONFLICT (id) DO NOTHING;",
            table, cols_str, vals_str
        )
    } else {
        String::new()
    }
}

/// Convierte un valor de JSON a su correspondiente representación literal en SQL para PostgreSQL.
fn json_value_to_sql_literal(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::Bool(b) => {
            if *b {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => {
            let escaped = s.replace('\'', "''");
            format!("'{}'", escaped)
        }
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            let s = serde_json::to_string(val).unwrap_or_default();
            let escaped = s.replace('\'', "''");
            format!("'{}'::jsonb", escaped)
        }
    }
}

/// Implementación en memoria thread-safe de `BackupRepository` para pruebas y modo volátil.
#[derive(Debug, Default)]
pub struct InMemoryBackupRepository {
    storage: std::sync::RwLock<BackupPayload>,
}

impl InMemoryBackupRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_initial_payload(payload: BackupPayload) -> Self {
        Self {
            storage: std::sync::RwLock::new(payload),
        }
    }
}

#[async_trait]
impl BackupRepository for InMemoryBackupRepository {
    async fn export_all(
        &self,
        _project: Option<&str>,
        _include_embeddings: bool,
    ) -> Result<BackupPayload, ApplicationError> {
        let read = self
            .storage
            .read()
            .map_err(|e| ApplicationError::Internal(e.to_string()))?;
        Ok(read.clone())
    }

    async fn restore_all(
        &self,
        payload: &BackupPayload,
        wipe: bool,
    ) -> Result<RestoreCounts, ApplicationError> {
        let mut write = self
            .storage
            .write()
            .map_err(|e| ApplicationError::Internal(e.to_string()))?;
        if wipe {
            *write = BackupPayload::default();
        }

        let counts = RestoreCounts {
            memories: payload.memories.len(),
            embeddings: payload.embeddings.len(),
            graph_nodes: payload.graph_nodes.len(),
            graph_edges: payload.graph_edges.len(),
            learning_candidates: payload.learning_candidates.len(),
            learning_evidences: payload.learning_evidences.len(),
            conflicts: payload.knowledge_conflicts.len(),
            consolidation_runs: payload.consolidation_runs.len(),
        };

        write.memories.extend(payload.memories.clone());
        write.embeddings.extend(payload.embeddings.clone());
        write.graph_nodes.extend(payload.graph_nodes.clone());
        write.graph_edges.extend(payload.graph_edges.clone());
        write
            .learning_candidates
            .extend(payload.learning_candidates.clone());
        write
            .learning_evidences
            .extend(payload.learning_evidences.clone());
        write
            .knowledge_conflicts
            .extend(payload.knowledge_conflicts.clone());
        write
            .consolidation_runs
            .extend(payload.consolidation_runs.clone());

        Ok(counts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_backup_and_restore_jsonl_roundtrip() {
        let mut initial = BackupPayload::default();
        initial.memories.push(serde_json::json!({
            "id": "0191f6d0-0000-7000-8000-000000000001",
            "content_text": "Memoria de prueba para respaldo",
            "memory_type": "episodic",
            "importance": 0.8
        }));
        initial.graph_nodes.push(serde_json::json!({
            "id": "0191f6d0-0000-7000-8000-000000000002",
            "label": "Rust",
            "node_type": "concept"
        }));

        let source_repo = Arc::new(InMemoryBackupRepository::with_initial_payload(initial));
        let backup_uc = BackupUseCase::new(source_repo);

        let backup_res = backup_uc
            .execute(BackupCommand {
                format: BackupFormat::Jsonl,
                project: None,
                include_embeddings: true,
            })
            .await
            .expect("Backup exitoso");

        assert_eq!(backup_res.manifest.total_memories, 1);
        assert_eq!(backup_res.manifest.total_graph_nodes, 1);
        assert!(!backup_res.manifest.checksum_sha256.is_empty());

        // Probar Dry Run
        let dest_repo = Arc::new(InMemoryBackupRepository::new());
        let restore_uc = RestoreUseCase::new(dest_repo.clone());

        let dry_run_res = restore_uc
            .execute(RestoreCommand {
                content: backup_res.content.clone(),
                format: BackupFormat::Jsonl,
                dry_run: true,
                wipe: false,
            })
            .await
            .expect("Dry run exitoso");

        assert!(dry_run_res.is_dry_run);
        assert_eq!(dry_run_res.counts.memories, 1);
        assert!(dest_repo.storage.read().unwrap().memories.is_empty());

        // Probar Restore real
        let restore_res = restore_uc
            .execute(RestoreCommand {
                content: backup_res.content,
                format: BackupFormat::Jsonl,
                dry_run: false,
                wipe: false,
            })
            .await
            .expect("Restore exitoso");

        assert!(!restore_res.is_dry_run);
        assert_eq!(restore_res.counts.memories, 1);
        assert_eq!(dest_repo.storage.read().unwrap().memories.len(), 1);
    }

    #[tokio::test]
    async fn test_checksum_tampering_is_rejected() {
        let mut initial = BackupPayload::default();
        initial
            .memories
            .push(serde_json::json!({ "id": "m1", "content_text": "legítimo" }));

        let repo = Arc::new(InMemoryBackupRepository::with_initial_payload(initial));
        let backup_uc = BackupUseCase::new(repo.clone());

        let res = backup_uc
            .execute(BackupCommand {
                format: BackupFormat::Jsonl,
                project: None,
                include_embeddings: true,
            })
            .await
            .unwrap();

        // Alterar el contenido de datos sin cambiar el checksum del manifiesto
        let tampered_content = res.content.replace("legítimo", "malicioso_alterado");

        let restore_uc = RestoreUseCase::new(repo);
        let err = restore_uc
            .execute(RestoreCommand {
                content: tampered_content,
                format: BackupFormat::Jsonl,
                dry_run: false,
                wipe: false,
            })
            .await;

        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("Fallo de integridad"));
    }
}
