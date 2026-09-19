//! Invariantes de progresión y mitigación de alucinaciones (SRS §15, §59, §60, [F6-01]).

use chrono::Utc;

use crate::confidence::ConfidenceCalculator;
use crate::errors::LearningError;
use crate::model::{CandidateKnowledge, LearningStage};

/// Umbral mínimo de evidencias empíricas independientes para validar conocimiento automáticamente sin intervención humana.
pub const MIN_EVIDENCES_FOR_AUTO_VALIDATION: usize = 3;

/// Umbral de confianza mínimo para permitir la validación automática.
pub const MIN_CONFIDENCE_FOR_VALIDATION: f32 = 0.65;

/// Reglas e invariantes del ciclo cognitivo de aprendizaje.
pub struct LearningInvariants;

impl LearningInvariants {
    /// Evalúa y aplica la progresión de etapa y actualización de confianza de un candidato.
    ///
    /// Retorna `Ok(true)` si la etapa cambió, `Ok(false)` si se mantuvo igual.
    pub fn evaluate_progression(candidate: &mut CandidateKnowledge) -> Result<bool, LearningError> {
        let now = Utc::now();
        let (new_confidence, breakdown) = ConfidenceCalculator::calculate(
            &candidate.evidences,
            candidate.human_validated,
            candidate.stage,
            now,
        );

        candidate.confidence = new_confidence;
        candidate.updated_at = now;

        let previous_stage = candidate.stage;
        let target_stage = Self::determine_target_stage(candidate, &breakdown)?;

        if previous_stage != target_stage {
            if !previous_stage.can_transition_to(target_stage) {
                return Err(LearningError::InvalidStageTransition {
                    from: previous_stage.to_string(),
                    to: target_stage.to_string(),
                    reason: "Transición directa no permitida por las reglas de invariantes del ciclo cognitivo".to_string(),
                });
            }
            candidate.stage = target_stage;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Determina la etapa objetivo en función de las evidencias y la validación.
    fn determine_target_stage(
        candidate: &CandidateKnowledge,
        breakdown: &crate::confidence::ConfidenceBreakdown,
    ) -> Result<LearningStage, LearningError> {
        // Si las contradicciones superan significativamente al soporte, la hipótesis se rechaza
        if breakdown.contradicting_count > 0
            && breakdown.contradicting_weight_sum > breakdown.supporting_weight_sum
        {
            return Ok(LearningStage::Rejected);
        }

        match candidate.stage {
            LearningStage::Observation => {
                // Si una observación recibe evidencias adicionales de soporte, evoluciona a Candidate
                if breakdown.supporting_count >= 2 || candidate.human_validated {
                    Ok(LearningStage::Candidate)
                } else {
                    Ok(LearningStage::Observation)
                }
            }
            LearningStage::Candidate => {
                // Validación humana explícita promueve inmediatamente a Validated si no hay contradicciones
                if candidate.human_validated && breakdown.contradicting_count == 0 {
                    return Ok(LearningStage::Validated);
                }

                // Validación automática por acumulación empírica:
                // Requiere >= 3 evidencias de soporte, confianza >= 0.65 y 0 contradicciones
                if breakdown.supporting_count >= MIN_EVIDENCES_FOR_AUTO_VALIDATION
                    && breakdown.final_score >= MIN_CONFIDENCE_FOR_VALIDATION
                    && breakdown.contradicting_count == 0
                {
                    Ok(LearningStage::Validated)
                } else {
                    Ok(LearningStage::Candidate)
                }
            }
            LearningStage::Validated => {
                // Si surgen contradicciones, desciende a Candidate o Rejected
                if breakdown.contradicting_count > 0 {
                    if breakdown.contradicting_weight_sum >= breakdown.supporting_weight_sum {
                        Ok(LearningStage::Rejected)
                    } else {
                        Ok(LearningStage::Candidate)
                    }
                } else {
                    Ok(LearningStage::Validated)
                }
            }
            LearningStage::Consolidated => {
                if breakdown.contradicting_count > 0 {
                    Ok(LearningStage::Candidate)
                } else {
                    Ok(LearningStage::Consolidated)
                }
            }
            LearningStage::Rejected => {
                // Si posteriormente se acumula suficiente soporte nuevo, puede reactivarse como Candidate
                if breakdown.supporting_weight_sum > breakdown.contradicting_weight_sum * 1.5 {
                    Ok(LearningStage::Candidate)
                } else {
                    Ok(LearningStage::Rejected)
                }
            }
        }
    }
}
