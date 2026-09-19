//! Políticas de decaimiento temporal y olvido controlado (SRS §58, F8-03).

use chrono::{DateTime, Utc};

use crate::config::{DecayConfig, DecayStrategy};

/// Calcula el factor de recencia y decaimiento temporal para una memoria (SRS §58, F8-03).
///
/// Retorna un valor en el rango `[min_decay_factor, 1.0]`.
///
/// Principios clave del SRS:
/// - Antigüedad y ausencia de recuperación disminuyen la relevancia.
/// - Recuperaciones recientes refrescan la retención cognitiva.
/// - Baja utilidad y baja confianza aceleran la pérdida de prioridad.
/// - **Regla de Salvaguarda:** Recuerdos con `importance >= protected_importance_threshold`
///   están blindados contra el decaimiento temporal, previniendo la pérdida de decisiones arquitectónicas clave.
pub fn calculate_decay_factor(
    created_at: DateTime<Utc>,
    last_retrieved_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    importance: f32,
    confidence: f32,
    utility: f32,
    config: &DecayConfig,
) -> f32 {
    // 1. Salvaguarda inquebrantable para recuerdos fundamentales (SRS §58)
    if importance >= config.protected_importance_threshold {
        return 1.0;
    }

    if config.strategy == DecayStrategy::None {
        return 1.0;
    }

    // 2. Tiempo inactivo efectivo en días desde la última interacción
    let last_interaction = last_retrieved_at.unwrap_or(created_at);
    let elapsed_duration = now.signed_duration_since(last_interaction);
    let delta_days = (elapsed_duration.num_seconds() as f32 / 86400.0).max(0.0);

    // 3. Cálculo base según la estrategia matemática configurada
    let base_factor = match config.strategy {
        DecayStrategy::HalfLife => {
            let half_life = config.half_life_days.max(0.1);
            0.5f32.powf(delta_days / half_life)
        }
        DecayStrategy::Exponential => {
            let lambda = config.lambda.max(0.0001);
            (-lambda * delta_days).exp()
        }
        DecayStrategy::Logarithmic => {
            let alpha = 0.5f32;
            1.0 / (1.0 + alpha * (1.0 + delta_days).ln())
        }
        DecayStrategy::None => 1.0,
    };

    // 4. Modulación por calidad empírica (baja utilidad o baja confianza aceleran el olvido)
    let quality_multiplier = if utility < 0.3 && confidence < 0.4 {
        // Penalización por baja utilidad y baja certidumbre
        ((utility + confidence) / 2.0).clamp(0.5, 1.0)
    } else if utility > 0.8 && confidence > 0.8 {
        // Bonificación por alta utilidad y alta certidumbre
        1.1
    } else {
        1.0
    };

    let modulated = base_factor * quality_multiplier;

    // 5. Aplicación del piso mínimo de decaimiento y tope superior 1.0
    modulated.clamp(config.min_decay_factor, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_zero_elapsed_time_yields_full_factor() {
        let now = Utc::now();
        let config = DecayConfig::default();

        let factor = calculate_decay_factor(now, None, now, 0.5, 0.5, 0.5, &config);
        assert!((factor - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_protected_high_importance_memory_resists_decay() {
        let now = Utc::now();
        let old_time = now - Duration::days(365); // 1 año de antigüedad
        let config = DecayConfig::default(); // protected threshold = 0.85

        let factor = calculate_decay_factor(old_time, None, now, 0.90, 0.2, 0.1, &config);
        assert_eq!(
            factor, 1.0,
            "Memoria con importancia 0.90 debe resistir 100% el decaimiento"
        );
    }

    #[test]
    fn test_half_life_decay_at_exact_half_life() {
        let now = Utc::now();
        let config = DecayConfig {
            strategy: DecayStrategy::HalfLife,
            half_life_days: 30.0,
            min_decay_factor: 0.05,
            protected_importance_threshold: 0.95,
            ..Default::default()
        };

        let past_time = now - Duration::days(30);
        let factor = calculate_decay_factor(past_time, None, now, 0.5, 0.5, 0.5, &config);

        // A los 30 días, con half_life de 30 días, factor base = 0.5
        assert!((factor - 0.5).abs() < 0.02);
    }

    #[test]
    fn test_exponential_decay() {
        let now = Utc::now();
        let config = DecayConfig {
            strategy: DecayStrategy::Exponential,
            lambda: 0.05,
            min_decay_factor: 0.05,
            protected_importance_threshold: 0.95,
            ..Default::default()
        };

        let past_time = now - Duration::days(20);
        let factor = calculate_decay_factor(past_time, None, now, 0.5, 0.5, 0.5, &config);

        // exp(-0.05 * 20) = exp(-1.0) ~ 0.3678
        assert!((factor - 0.3678).abs() < 0.02);
    }

    #[test]
    fn test_min_decay_factor_clamping() {
        let now = Utc::now();
        let very_old = now - Duration::days(1000);
        let config = DecayConfig {
            strategy: DecayStrategy::HalfLife,
            half_life_days: 10.0,
            min_decay_factor: 0.15,
            protected_importance_threshold: 0.95,
            ..Default::default()
        };

        let factor = calculate_decay_factor(very_old, None, now, 0.3, 0.3, 0.3, &config);
        assert!(
            factor >= 0.15,
            "El factor no puede caer por debajo del piso configurado"
        );
        assert_eq!(factor, 0.15);
    }

    #[test]
    fn test_recent_retrieval_refreshes_factor() {
        let now = Utc::now();
        let very_old_created = now - Duration::days(300);
        let recently_retrieved = now - Duration::hours(2); // recuperado hace 2 horas
        let config = DecayConfig::default();

        let factor = calculate_decay_factor(
            very_old_created,
            Some(recently_retrieved),
            now,
            0.5,
            0.5,
            0.5,
            &config,
        );
        assert!(
            factor > 0.98,
            "Una recuperación reciente debe rejuvenecer el factor a casi 1.0"
        );
    }
}
