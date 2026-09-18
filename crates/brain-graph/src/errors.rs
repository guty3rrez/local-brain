//! Definición de errores de invariantes y operaciones del grafo de conocimiento (SRS §11).

use thiserror::Error;

/// Errores derivados de la validación y operaciones en el grafo de conocimiento.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum GraphError {
    #[error("Tipo de relación inválido: '{0}'. Debe ser uno de los 10 tipos admitidos (SRS §11)")]
    InvalidRelation(String),

    #[error("No se permiten auto-bucles (self-loops) en el nodo '{node_id}' con la relación '{relation}'")]
    SelfLoopForbidden { node_id: String, relation: String },

    #[error("Ciclo inválido detectado al intentar crear la relación acíclica '{relation}' desde '{from}' hacia '{to}'")]
    CycleDetected {
        from: String,
        to: String,
        relation: String,
    },

    #[error("Nodo de grafo con identificador '{0}' no encontrado")]
    NodeNotFound(String),

    #[error("Arista de grafo con identificador '{0}' no encontrada")]
    EdgeNotFound(String),

    #[error(
        "Arista duplicada: ya existe una relación '{relation}' entre '{source_id}' y '{target_id}'"
    )]
    DuplicateEdge {
        source_id: String,
        target_id: String,
        relation: String,
    },

    #[error("El peso de la relación debe estar en el rango [0.0, 1.0]. Recibido: {0}")]
    InvalidWeight(f32),

    #[error("La profundidad de traversal debe estar entre 1 y {max}. Recibido: {actual}")]
    InvalidDepth { actual: u32, max: u32 },

    #[error("La etiqueta (label) del nodo no puede estar vacía ni contener solo espacios")]
    EmptyLabel,

    #[error("Error de almacenamiento en repositorio de grafos: {0}")]
    StorageError(String),
}

impl From<brain_domain::model::DomainError> for GraphError {
    fn from(err: brain_domain::model::DomainError) -> Self {
        GraphError::StorageError(err.to_string())
    }
}
