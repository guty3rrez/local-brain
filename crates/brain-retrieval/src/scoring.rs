//! Algoritmos de scoring multidimensional y explicabilidad de ranking (SRS §14, §56, §57, F8-02).

use serde::{Deserialize, Serialize};

use crate::config::ScoringWeights;

/// Desglose explicativo detallado del cálculo de score para un recuerdo (SRS §14, §57).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoreExplanation {
    /// Similitud bruta de entrada (e.g. coseno o RRF normalizado).
    pub raw_similarity: f32,
    /// Similitud semántica normalizada [0.0 - 1.0].
    pub semantic_similarity: f32,
    /// Importancia intrínseca evaluada [0.0 - 1.0].
    pub importance: f32,
    /// Confianza empírica del conocimiento [0.0 - 1.0].
    pub confidence: f32,
    /// Utilidad histórica acumulada [0.0 - 1.0].
    pub utility: f32,
    /// Factor de recencia temporal tras decaimiento [0.0 - 1.0].
    pub recency_factor: f32,
    /// Puntuación compuesta final normalizada [0.0 - 1.0].
    pub final_score: f32,
    /// Señales cualitativas o advertencias que influyeron en el ranking.
    pub signals: Vec<String>,
}

impl ScoreExplanation {
    /// Genera un resumen textual legible para humanos y agentes (SRS §57).
    pub fn to_formatted_summary(&self) -> String {
        format!(
            "Score: {:.4} [Similitud: {:.2} | Importancia: {:.2} | Confianza: {:.2} | Utilidad: {:.2} | Recencia: {:.2}]",
            self.final_score,
            self.semantic_similarity,
            self.importance,
            self.confidence,
            self.utility,
            self.recency_factor
        )
    }
}

/// Evaluador de scoring multidimensional para memorias candidatas (SRS §14, §56).
#[derive(Debug, Clone)]
pub struct MultidimensionalScorer {
    weights: ScoringWeights,
}

impl MultidimensionalScorer {
    /// Inicializa el scorer con los pesos configurados.
    pub fn new(weights: ScoringWeights) -> Self {
        Self { weights }
    }

    /// Calcula la puntuación multidimensional para una memoria combinando sus dimensiones cognitivas.
    ///
    /// Fórmula base (SRS §14):
    /// score = (similitud^w_sim) * (importancia^w_imp) * (confianza^w_conf) * (utilidad^w_util) * recency_factor
    ///
    /// O suma ponderada normalizada cuando se requiere sensibilidad lineal.
    pub fn compute_score(
        &self,
        raw_similarity: f32,
        importance: f32,
        confidence: f32,
        utility: f32,
        recency_factor: f32,
    ) -> ScoreExplanation {
        let sim = raw_similarity.clamp(0.0, 1.0);
        let imp = importance.clamp(0.0, 1.0);
        let conf = confidence.clamp(0.0, 1.0);
        let util = utility.clamp(0.0, 1.0);
        let rec = recency_factor.clamp(0.0, 1.0);

        let mut signals = Vec::new();
        if imp >= 0.85 {
            signals.push("Alta importancia intrínseca (protegida)".to_string());
        }
        if conf >= 0.80 {
            signals.push("Alta confianza empírica validada".to_string());
        }
        if rec < 0.50 {
            signals.push("Penalizada por decaimiento temporal".to_string());
        }
        if util > 0.80 {
            signals.push("Alta utilidad práctica comprobada".to_string());
        }

        // Combinación lineal ponderada normalizada por la suma de pesos:
        let total_weights = self.weights.semantic_weight
            + self.weights.importance_weight
            + self.weights.confidence_weight
            + self.weights.utility_weight
            + self.weights.recency_weight;

        let normalized_weights = if total_weights > 0.0 {
            total_weights
        } else {
            1.0
        };

        let weighted_sum = (sim * self.weights.semantic_weight
            + imp * self.weights.importance_weight
            + conf * self.weights.confidence_weight
            + util * self.weights.utility_weight
            + rec * self.weights.recency_weight)
            / normalized_weights;

        // Modulador multiplicativo suave de afinidad semántica (si sim es 0, penaliza fuertemente):
        let semantic_floor = 0.15;
        let semantic_multiplier = (sim + semantic_floor).min(1.0);

        let final_score = (weighted_sum * semantic_multiplier).clamp(0.0, 1.0);

        ScoreExplanation {
            raw_similarity,
            semantic_similarity: sim,
            importance: imp,
            confidence: conf,
            utility: util,
            recency_factor: rec,
            final_score,
            signals,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_scores_yield_high_score() {
        let scorer = MultidimensionalScorer::new(ScoringWeights::default());
        let result = scorer.compute_score(1.0, 1.0, 1.0, 1.0, 1.0);

        assert!(result.final_score > 0.95);
        assert_eq!(result.signals.len(), 3);
    }

    #[test]
    fn test_zero_similarity_heavily_penalizes_score() {
        let scorer = MultidimensionalScorer::new(ScoringWeights::default());
        let result = scorer.compute_score(0.0, 1.0, 1.0, 1.0, 1.0);

        assert!(
            result.final_score < 0.20,
            "Sin similitud semántica, el score debe ser muy bajo a pesar de alta importancia"
        );
    }

    #[test]
    fn test_high_importance_and_confidence_boost_rank() {
        let scorer = MultidimensionalScorer::new(ScoringWeights::default());
        let high_quality = scorer.compute_score(0.7, 0.9, 0.85, 0.8, 1.0);
        let low_quality = scorer.compute_score(0.7, 0.2, 0.2, 0.1, 1.0);

        assert!(
            high_quality.final_score > low_quality.final_score,
            "Calidad empírica e importancia deben aumentar sensiblemente el score"
        );
    }

    #[test]
    fn test_recency_penalty_reduces_score() {
        let scorer = MultidimensionalScorer::new(ScoringWeights::default());
        let fresh = scorer.compute_score(0.8, 0.5, 0.5, 0.5, 1.0);
        let aged = scorer.compute_score(0.8, 0.5, 0.5, 0.5, 0.2);

        assert!(fresh.final_score > aged.final_score);
        assert!(aged
            .signals
            .contains(&"Penalizada por decaimiento temporal".to_string()));
    }
}
