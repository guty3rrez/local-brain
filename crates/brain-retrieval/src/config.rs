//! Configuración del pipeline de recuperación híbrida, scoring multidimensional y decaimiento (SRS §13, §14, §56, §58).

use serde::{Deserialize, Serialize};

use crate::errors::RetrievalError;

/// Estrategia matemática de decaimiento de relevancia temporal (SRS §58).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecayStrategy {
    /// Decaimiento por vida media: factor = 0.5 ^ (delta_días / half_life_días)
    #[default]
    HalfLife,
    /// Decaimiento exponencial estándar: factor = exp(-lambda * delta_días)
    Exponential,
    /// Decaimiento logarítmico amortiguado: factor = 1.0 / (1.0 + alpha * ln(1 + delta_días))
    Logarithmic,
    /// Sin decaimiento temporal (recencia no penaliza recuerdos antiguos)
    None,
}

/// Parámetros de la política de decaimiento y olvido temporal (SRS §58, F8-03).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayConfig {
    /// Estrategia de decaimiento adoptada.
    pub strategy: DecayStrategy,
    /// Número de días para que la relevancia temporal caiga al 50% (en HalfLife). Por defecto 30.0.
    pub half_life_days: f32,
    /// Coeficiente de decaimiento exponencial lambda (en Exponential). Por defecto 0.023 (aprox ~30 días vida media).
    pub lambda: f32,
    /// Factor mínimo al que puede decaer un recuerdo no protegido (piso de decaimiento). Por defecto 0.10.
    pub min_decay_factor: f32,
    /// Umbral de importancia intrínseca a partir del cual el recuerdo está protegido contra decaimiento (SRS §58).
    /// Por defecto 0.85: las decisiones y directrices fundamentales no pierden relevancia.
    pub protected_importance_threshold: f32,
}

impl Default for DecayConfig {
    fn default() -> Self {
        Self {
            strategy: DecayStrategy::HalfLife,
            half_life_days: 30.0,
            lambda: 0.023,
            min_decay_factor: 0.10,
            protected_importance_threshold: 0.85,
        }
    }
}

/// Pesos para la combinación en Reciprocal Rank Fusion (RRF) (SRS §13.2, F8-01).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionWeights {
    /// Ponderación del canal de búsqueda vectorial semántica (pgvector).
    pub vector_weight: f32,
    /// Ponderación del canal de búsqueda léxica full-text (PostgreSQL tsvector / BM25).
    pub fts_weight: f32,
    /// Ponderación del canal de expansión de grafo (vecindad de conceptos).
    pub graph_weight: f32,
    /// Constante de suavizado de ranking RRF k (estándar: 60.0).
    pub rrf_k: f32,
}

impl Default for FusionWeights {
    fn default() -> Self {
        Self {
            vector_weight: 0.50,
            fts_weight: 0.30,
            graph_weight: 0.20,
            rrf_k: 60.0,
        }
    }
}

/// Pesos y factores para el cálculo de scoring multidimensional (SRS §14, §56, F8-02).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringWeights {
    /// Peso relativo de la similitud semántica / de consulta.
    pub semantic_weight: f32,
    /// Peso relativo de la importancia intrínseca del recuerdo (0.0 - 1.0).
    pub importance_weight: f32,
    /// Peso relativo de la confianza empírica / certidumbre (0.0 - 1.0).
    pub confidence_weight: f32,
    /// Peso relativo de la utilidad histórica acumulada (0.0 - 1.0).
    pub utility_weight: f32,
    /// Peso relativo del factor temporal de recencia (0.0 - 1.0).
    pub recency_weight: f32,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            semantic_weight: 0.40,
            importance_weight: 0.25,
            confidence_weight: 0.20,
            utility_weight: 0.15,
            recency_weight: 0.10,
        }
    }
}

/// Parámetros de límites y umbrales de recuperación.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalLimits {
    /// Límite de recuerdos devueltos por defecto si el usuario o agente no especifica uno.
    pub default_limit: usize,
    /// Límite máximo permitido para salvaguardar latencia y memoria.
    pub max_limit: usize,
    /// Score mínimo total para que un recuerdo sea admitido en la respuesta final.
    pub min_score: f32,
    /// Profundidad máxima de saltos en el grafo de conocimiento para expansión (SRS §11).
    pub graph_max_depth: usize,
    /// Máximo de nodos relacionados a incorporar en la expansión de grafo.
    pub graph_expansion_limit: usize,
}

impl Default for RetrievalLimits {
    fn default() -> Self {
        Self {
            default_limit: 10,
            max_limit: 100,
            min_score: 0.05,
            graph_max_depth: 1,
            graph_expansion_limit: 5,
        }
    }
}

/// Configuración global de recuperación híbrida configurable vía `brain.toml` (SRS §13, §14, §56, §58).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RetrievalConfig {
    pub fusion: FusionWeights,
    pub scoring: ScoringWeights,
    pub decay: DecayConfig,
    pub limits: RetrievalLimits,
}

impl RetrievalConfig {
    /// Crea una nueva configuración con los valores por defecto recomendados.
    pub fn new() -> Self {
        Self::default()
    }

    /// Carga y valida la configuración a partir de una cadena con formato TOML (`brain.toml`).
    pub fn from_toml(content: &str) -> Result<Self, RetrievalError> {
        let config: Self = toml::from_str(content)
            .map_err(|e| RetrievalError::ConfigError(format!("Error de sintaxis TOML: {e}")))?;
        config.validate()?;
        Ok(config)
    }

    /// Serializa la configuración actual a formato TOML.
    pub fn to_toml(&self) -> Result<String, RetrievalError> {
        toml::to_string_pretty(self)
            .map_err(|e| RetrievalError::ConfigError(format!("Error serializando TOML: {e}")))
    }

    /// Valida que las invariantes de rango de pesos, límites y coeficientes se cumplan.
    pub fn validate(&self) -> Result<(), RetrievalError> {
        if self.fusion.vector_weight < 0.0
            || self.fusion.fts_weight < 0.0
            || self.fusion.graph_weight < 0.0
        {
            return Err(RetrievalError::InvalidWeight(
                "Los pesos de fusión (vector, fts, graph) no pueden ser negativos".into(),
            ));
        }

        if self.fusion.rrf_k <= 0.0 {
            return Err(RetrievalError::InvalidWeight(
                "La constante RRF k debe ser estrictamente positiva (ej. 60.0)".into(),
            ));
        }

        if self.scoring.semantic_weight < 0.0
            || self.scoring.importance_weight < 0.0
            || self.scoring.confidence_weight < 0.0
            || self.scoring.utility_weight < 0.0
            || self.scoring.recency_weight < 0.0
        {
            return Err(RetrievalError::InvalidWeight(
                "Los pesos de scoring no pueden ser negativos".into(),
            ));
        }

        if self.decay.half_life_days <= 0.0 {
            return Err(RetrievalError::ConfigError(
                "half_life_days debe ser mayor a 0".into(),
            ));
        }

        if self.decay.min_decay_factor < 0.0 || self.decay.min_decay_factor > 1.0 {
            return Err(RetrievalError::ConfigError(
                "min_decay_factor debe estar entre 0.0 y 1.0".into(),
            ));
        }

        if self.decay.protected_importance_threshold < 0.0
            || self.decay.protected_importance_threshold > 1.0
        {
            return Err(RetrievalError::ConfigError(
                "protected_importance_threshold debe estar entre 0.0 y 1.0".into(),
            ));
        }

        if self.limits.default_limit == 0 || self.limits.max_limit == 0 {
            return Err(RetrievalError::InvalidLimit(
                "Los límites de recuperación deben ser mayores a 0".into(),
            ));
        }

        if self.limits.default_limit > self.limits.max_limit {
            return Err(RetrievalError::InvalidLimit(
                "default_limit no puede ser mayor que max_limit".into(),
            ));
        }

        Ok(())
    }
}
