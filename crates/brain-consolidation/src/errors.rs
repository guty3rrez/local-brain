//! Errores de dominio para el motor de consolidación y reflexión (SRS §16, §17, §18).

use thiserror::Error;

use brain_domain::model::DomainError;
use brain_graph::errors::GraphError;
use brain_learning::errors::LearningError;

use crate::model::ConflictId;

#[derive(Debug, Error, PartialEq)]
pub enum ConsolidationError {
    #[error("Error en la capa de dominio base: {0}")]
    Domain(#[from] DomainError),

    #[error("Error en el grafo de conocimiento: {0}")]
    Graph(#[from] GraphError),

    #[error("Error en el motor de aprendizaje: {0}")]
    Learning(#[from] LearningError),

    #[error("Identificador de conflicto inválido: {0}")]
    InvalidConflictId(String),

    #[error("El tamaño del cluster ({actual}) es menor al mínimo requerido ({minimum})")]
    ClusterTooSmall { actual: usize, minimum: usize },

    #[error("La confianza de una hipótesis generada por reflexión ({confidence}) excede el límite tentativo de {max}")]
    HypothesisConfidenceTooHigh { confidence: f32, max: f32 },

    #[error("Una hipótesis de consolidación debe originarse de al menos una experiencia previa")]
    EmptySourceMemories,

    #[error("No se permite crear una auto-contradicción sobre la misma memoria: {0}")]
    SelfContradiction(String),

    #[error("Conflicto con ID {0} no fue encontrado")]
    ConflictNotFound(ConflictId),

    #[error("El conflicto {0} ya ha sido resuelto previamente")]
    ConflictAlreadyResolved(ConflictId),

    #[error("Error en repositorio de persistencia de consolidación: {0}")]
    RepositoryError(String),

    #[error("Error en el servicio de inferencia/LLM de consolidación: {0}")]
    InferenceError(String),

    #[error("El umbral de similitud debe estar entre 0.0 y 1.0. Valor recibido: {0}")]
    InvalidSimilarityThreshold(f32),
}
