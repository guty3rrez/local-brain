//! Modelos de Dominio de Memoria según SRS §9 y §10.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errores de invariantes de dominio.
#[derive(Debug, Error, PartialEq)]
pub enum DomainError {
    #[error("El valor numérico debe estar en el rango [0.0, 1.0]. Valor recibido: {0}")]
    OutOfRange(f32),

    #[error("El contenido de la memoria no puede estar vacío")]
    EmptyContent,
}

/// Tipos de memoria cognitiva soportados (SRS §10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    /// Memoria de trabajo volátil asociada a la sesión activa (SRS §10.1).
    Working,
    /// Experiencias y episodios concretos (SRS §10.2).
    Episodic,
    /// Conocimiento generalizado y hechos (SRS §10.3).
    Semantic,
    /// Procedimientos y métodos paso a paso (SRS §10.4).
    Procedural,
    /// Vínculos y asociaciones conceptuales (SRS §10.5).
    Associative,
}

/// Estado del ciclo de vida de una memoria (SRS §9.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryStatus {
    Active,
    Archived,
    PendingEmbedding,
    Conflict,
    SoftDeleted,
}

/// Grado de importancia o relevancia (0.0 a 1.0).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Importance(f32);

impl Importance {
    pub fn new(val: f32) -> Result<Self, DomainError> {
        if (0.0..=1.0).contains(&val) {
            Ok(Self(val))
        } else {
            Err(DomainError::OutOfRange(val))
        }
    }

    pub fn value(&self) -> f32 {
        self.0
    }
}

/// Nivel de confianza o evidencia empírica (0.0 a 1.0) (SRS §60).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Confidence(f32);

impl Confidence {
    pub fn new(val: f32) -> Result<Self, DomainError> {
        if (0.0..=1.0).contains(&val) {
            Ok(Self(val))
        } else {
            Err(DomainError::OutOfRange(val))
        }
    }

    pub fn value(&self) -> f32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn importance_valid_range() {
        assert!(Importance::new(0.0).is_ok());
        assert!(Importance::new(0.5).is_ok());
        assert!(Importance::new(1.0).is_ok());
        assert_eq!(Importance::new(-0.1), Err(DomainError::OutOfRange(-0.1)));
        assert_eq!(Importance::new(1.01), Err(DomainError::OutOfRange(1.01)));
    }

    #[test]
    fn confidence_valid_range() {
        assert!(Confidence::new(0.85).is_ok());
        assert_eq!(Confidence::new(1.5), Err(DomainError::OutOfRange(1.5)));
    }
}
