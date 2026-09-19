//! Definición de errores tipados para el módulo de aprendizaje y conocimiento candidato (SRS §15, §59, §60).

use thiserror::Error;

/// Errores específicos del motor de aprendizaje y gestión de conocimiento candidato.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum LearningError {
    #[error(
        "El valor numérico de confianza debe estar en el rango [0.0, 1.0]. Valor recibido: {0}"
    )]
    OutOfRange(String),

    #[error("La afirmación o creencia no puede estar vacía ni contener únicamente espacios")]
    EmptyStatement,

    #[error("La afirmación excede el límite máximo permitido de {max} bytes (recibidos: {actual} bytes)")]
    StatementTooLarge { actual: usize, max: usize },

    #[error("La evidencia no puede estar vacía")]
    EmptyEvidence,

    #[error("Transición de etapa de aprendizaje inválida de '{from}' hacia '{to}': {reason}")]
    InvalidStageTransition {
        from: String,
        to: String,
        reason: String,
    },

    #[error("Evidencia insuficiente para validar conocimiento: se requieren al menos {required} evidencias empíricas (actuales: {actual})")]
    InsufficientEvidenceForValidation { required: usize, actual: usize },

    #[error("Contradicción insalvable detectada en candidato {candidate_id}: {details}")]
    ContradictionDetected {
        candidate_id: String,
        details: String,
    },

    #[error("Conocimiento candidato no encontrado: {0}")]
    NotFound(String),

    #[error("Identificador de candidato o evidencia inválido: {0}")]
    InvalidId(String),

    #[error("Tipo de fuente de evidencia inválido: {0}")]
    InvalidSourceType(String),

    #[error("Error en repositorio de persistencia de aprendizaje: {0}")]
    StorageError(String),
}
