//! Tipos de error específicos del dominio de recuperación híbrida y scoring (SRS §13, §14).

use thiserror::Error;

/// Errores producidos durante la configuración, ejecución del pipeline de recuperación o scoring.
#[derive(Debug, Error, PartialEq)]
pub enum RetrievalError {
    #[error("Consulta de recuperación vacía")]
    EmptyQuery,

    #[error("Configuración de pesos inválida: {0}")]
    InvalidWeight(String),

    #[error("Límite de candidatos inválido: {0}")]
    InvalidLimit(String),

    #[error("Error al cargar o validar configuración de recuperación: {0}")]
    ConfigError(String),

    #[error("Fallo durante el pipeline de recuperación híbrida: {0}")]
    PipelineError(String),

    #[error("Error de dominio: {0}")]
    Domain(#[from] brain_domain::model::DomainError),
}
