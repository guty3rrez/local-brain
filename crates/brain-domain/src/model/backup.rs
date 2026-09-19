//! Modelos de Dominio para Respaldo y Restauración de Memoria Cognitiva (SRS §52, §53, §30).
//!
//! Define los formatos de archivo y el manifiesto con sumas criptográficas SHA-256
//! para asegurar la integridad referencial y de datos durante la exportación/importación.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Formato admitido para archivos de respaldo y exportación de Local Brain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackupFormat {
    /// Formato Newline Delimited JSON (.jsonl). Portable, auditable y procesable en streaming.
    Jsonl,
    /// Script SQL con transacciones e instrucciones INSERT para PostgreSQL (.sql).
    Sql,
}

impl BackupFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Jsonl => "jsonl",
            Self::Sql => "sql",
        }
    }

    pub fn from_path_or_str(s: &str) -> Self {
        let lower = s.to_lowercase();
        if lower.ends_with(".sql") || lower == "sql" {
            Self::Sql
        } else {
            Self::Jsonl
        }
    }
}

/// Metadatos y manifiesto de integridad de un respaldo (SRS §30, §52).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupManifest {
    /// Versión del núcleo Local Brain que generó el respaldo.
    pub version: String,
    /// Timestamp UTC de generación del archivo de respaldo.
    pub exported_at: DateTime<Utc>,
    /// Filtro de proyecto aplicado si el respaldo fue parcial.
    pub project_filter: Option<String>,
    /// Si los vectores de embedding se incluyeron en el archivo.
    pub include_embeddings: bool,
    /// Cantidad total de recuerdos exportados.
    pub total_memories: usize,
    /// Cantidad total de embeddings exportados.
    pub total_embeddings: usize,
    /// Cantidad total de nodos de grafo exportados.
    pub total_graph_nodes: usize,
    /// Cantidad total de aristas de grafo exportadas.
    pub total_graph_edges: usize,
    /// Cantidad total de conocimientos candidatos exportados.
    pub total_learning_candidates: usize,
    /// Cantidad total de evidencias empíricas exportadas.
    pub total_learning_evidences: usize,
    /// Cantidad total de conflictos/contradicciones exportadas.
    pub total_conflicts: usize,
    /// Cantidad total de corridas de consolidación exportadas.
    pub total_consolidation_runs: usize,
    /// Suma de comprobación criptográfica SHA-256 del contenido de datos.
    pub checksum_sha256: String,
}

impl BackupManifest {
    pub fn total_entities(&self) -> usize {
        self.total_memories
            + self.total_graph_nodes
            + self.total_graph_edges
            + self.total_learning_candidates
            + self.total_learning_evidences
            + self.total_conflicts
            + self.total_consolidation_runs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_format_parsing() {
        assert_eq!(
            BackupFormat::from_path_or_str("dump.sql"),
            BackupFormat::Sql
        );
        assert_eq!(BackupFormat::from_path_or_str("sql"), BackupFormat::Sql);
        assert_eq!(
            BackupFormat::from_path_or_str("archive.jsonl"),
            BackupFormat::Jsonl
        );
        assert_eq!(
            BackupFormat::from_path_or_str("custom.txt"),
            BackupFormat::Jsonl
        );
    }

    #[test]
    fn test_backup_manifest_totals() {
        let manifest = BackupManifest {
            version: "0.1.0".to_string(),
            exported_at: Utc::now(),
            project_filter: None,
            include_embeddings: true,
            total_memories: 10,
            total_embeddings: 10,
            total_graph_nodes: 5,
            total_graph_edges: 4,
            total_learning_candidates: 2,
            total_learning_evidences: 3,
            total_conflicts: 1,
            total_consolidation_runs: 1,
            checksum_sha256: "abcd".to_string(),
        };

        assert_eq!(manifest.total_entities(), 26);
    }
}
