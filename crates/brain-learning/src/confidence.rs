//! Algoritmo de cálculo de confianza probabilística y heurística (SRS §60, [F6-02]).
//!
//! Evalúa cantidad de evidencias, calidad de fuentes, consistencia histórica,
//! presencia de contradicciones y validación humana explícita.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use brain_domain::model::Confidence;

use crate::model::{Evidence, LearningStage};

/// Desglose explicativo de los factores que componen el score de confianza (SRS §57, §60).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfidenceBreakdown {
    /// Puntuación final normalizada en el rango [0.0, 1.0].
    pub final_score: f32,
    /// Número total de evidencias que respaldan la afirmación.
    pub supporting_count: usize,
    /// Número total de evidencias que contradicen o refutan la afirmación.
    pub contradicting_count: usize,
    /// Suma ponderada de la calidad de evidencias de soporte.
    pub supporting_weight_sum: f32,
    /// Suma ponderada de la calidad de evidencias contradictorias.
    pub contradicting_weight_sum: f32,
    /// Factor de consistencia histórica (1.0 = consistencia perfecta, < 0.5 = contradicciones dominantes).
    pub consistency_factor: f32,
    /// Factor de saturación por volumen de evidencia empírica acumulada.
    pub volume_factor: f32,
    /// Indica si cuenta con validación y aprobación humana explícita.
    pub human_validated: bool,
}

/// Calculador determinista y heurístico de confianza para conocimientos candidatos.
pub struct ConfidenceCalculator;

impl ConfidenceCalculator {
    /// Calcula el score de confianza y el desglose de factores para un conjunto de evidencias.
    pub fn calculate(
        evidences: &[Evidence],
        human_validated: bool,
        stage: LearningStage,
        as_of: DateTime<Utc>,
    ) -> (Confidence, ConfidenceBreakdown) {
        let mut supporting_count = 0usize;
        let mut contradicting_count = 0usize;
        let mut supporting_weight_sum = 0.0f32;
        let mut contradicting_weight_sum = 0.0f32;

        for ev in evidences {
            // Factor de antigüedad suave: las evidencias retienen al menos 75% de su peso
            // incluso tras 180 días, garantizando que el historial no se degrade artificialmente.
            let days_old = (as_of - ev.recorded_at).num_days().max(0) as f32;
            let recency_factor = (1.0 - (days_old / 720.0)).clamp(0.75, 1.0);
            let effective_weight = ev.confidence_weight * recency_factor;

            if ev.is_supporting {
                supporting_count += 1;
                supporting_weight_sum += effective_weight;
            } else {
                contradicting_count += 1;
                contradicting_weight_sum += effective_weight;
            }
        }

        // 1. Factor de consistencia C: relación entre soporte y contraevidencia
        let total_weight = supporting_weight_sum + contradicting_weight_sum;
        let consistency_factor = if total_weight <= 0.0001 {
            1.0
        } else {
            // Penalización acelerada ante contraevidencias
            let penalized_contradiction = contradicting_weight_sum * 1.5;
            let denom = supporting_weight_sum + penalized_contradiction;
            if denom <= 0.0 {
                0.0
            } else {
                (supporting_weight_sum / denom).clamp(0.0, 1.0)
            }
        };

        // 2. Factor de volumen y saturación asintótica V
        // V(S) = S / (S + 1.0)
        let volume_factor = if supporting_weight_sum <= 0.0 {
            0.0
        } else {
            (supporting_weight_sum / (supporting_weight_sum + 1.0)).clamp(0.0, 1.0)
        };

        // 3. Puntuación base en función de etapa, volumen y consistencia
        let base_score = match stage {
            LearningStage::Observation => 0.25 + 0.15 * volume_factor,
            LearningStage::Rejected => 0.05,
            _ => 0.20 + 0.75 * (volume_factor * consistency_factor),
        };

        // Si existen contradicciones activas, aplicamos penalización cuadrática sobre la consistencia
        let score_after_contradictions = if contradicting_count > 0 {
            base_score * (consistency_factor * consistency_factor)
        } else {
            base_score
        };

        // 4. Bonificación por validación humana explícita (SRS §60)
        let final_raw = if human_validated {
            if contradicting_count == 0 {
                // Validación humana sin contradicciones garantiza mínimo 0.85
                score_after_contradictions.clamp(0.85, 1.0)
            } else {
                // Con contradicciones, la validación humana atenúa pero no elimina la alerta
                (score_after_contradictions + 0.30).clamp(0.40, 0.85)
            }
        } else {
            score_after_contradictions.clamp(0.0, 1.0)
        };

        // Normalización final estricta [0.0, 1.0] contra NaN o infinitos
        let safe_final = if final_raw.is_nan() || final_raw.is_infinite() {
            0.0
        } else {
            final_raw.clamp(0.0, 1.0)
        };

        let confidence = Confidence::new(safe_final).unwrap_or_else(|_| Confidence::tentative());

        let breakdown = ConfidenceBreakdown {
            final_score: safe_final,
            supporting_count,
            contradicting_count,
            supporting_weight_sum,
            contradicting_weight_sum,
            consistency_factor,
            volume_factor,
            human_validated,
        };

        (confidence, breakdown)
    }
}
