//! Detección de Contradicciones y Conflictos Cognitivos (SRS §18).

use crate::errors::ConsolidationError;
use crate::model::{ClusterableMemory, ConflictType, Contradiction};

/// Detector determinista y heurístico de contradicciones entre dos memorias o conocimientos.
pub struct ContradictionDetector;

impl ContradictionDetector {
    /// Evalúa si existe una contradicción lógica, semántica o de preferencia entre dos recuerdos.
    pub fn check_contradiction(
        mem_a: &ClusterableMemory,
        mem_b: &ClusterableMemory,
    ) -> Result<Option<Contradiction>, ConsolidationError> {
        if mem_a.id == mem_b.id {
            return Ok(None);
        }

        let text_a = mem_a.content.to_lowercase();
        let text_b = mem_b.content.to_lowercase();

        // 1. Detección de preferencias mutuamente excluyentes (Ejemplo Canónico SRS §18: Supabase vs .NET)
        if let Some(conflict) = Self::detect_preference_conflict(mem_a, mem_b, &text_a, &text_b)? {
            return Ok(Some(conflict));
        }

        // 2. Detección de polaridad invertida / negación directa sobre el mismo tema
        if let Some(conflict) = Self::detect_polarity_inversion(mem_a, mem_b, &text_a, &text_b)? {
            return Ok(Some(conflict));
        }

        Ok(None)
    }

    /// Detecta colisiones de preferencias tipo: "X es preferido para [contexto]" vs "Y es preferido para [contexto]"
    /// o "Prefiero X para [contexto]" vs "Prefiero Y para [contexto]"
    fn detect_preference_conflict(
        mem_a: &ClusterableMemory,
        mem_b: &ClusterableMemory,
        text_a: &str,
        text_b: &str,
    ) -> Result<Option<Contradiction>, ConsolidationError> {
        // Manejo de prefijo "prefiero X para Y" o "se prefiere X para Y"
        let is_prefix_pref_a =
            text_a.starts_with("prefiero ") || text_a.starts_with("se prefiere ");
        let is_prefix_pref_b =
            text_b.starts_with("prefiero ") || text_b.starts_with("se prefiere ");

        if is_prefix_pref_a && is_prefix_pref_b {
            let trim_a = text_a
                .strip_prefix("prefiero ")
                .or_else(|| text_a.strip_prefix("se prefiere "))
                .unwrap_or(text_a);
            let trim_b = text_b
                .strip_prefix("prefiero ")
                .or_else(|| text_b.strip_prefix("se prefiere "))
                .unwrap_or(text_b);

            if let (Some(pos_a), Some(pos_b)) = (trim_a.find(" para "), trim_b.find(" para ")) {
                let context_a = trim_a[pos_a + 6..].trim();
                let context_b = trim_b[pos_b + 6..].trim();
                let subject_a = trim_a[..pos_a].trim();
                let subject_b = trim_b[..pos_b].trim();

                if !context_a.is_empty() && context_a == context_b && subject_a != subject_b {
                    let reason = format!(
                        "Preferencia en conflicto para '{}': '{}' vs '{}'",
                        context_a, subject_a, subject_b
                    );
                    let suggestion = format!(
                        "Definir contexto específico: ej. '{}' para prototipos vs '{}' para producción",
                        subject_a, subject_b
                    );

                    let mut contradiction = Contradiction::new(
                        mem_a.id,
                        mem_b.id,
                        ConflictType::MutuallyExclusivePreference,
                        reason,
                    )?;
                    contradiction = contradiction.with_suggested_resolution(suggestion);
                    return Ok(Some(contradiction));
                }
            }
        }

        let preference_markers = [
            "es preferido para",
            "es preferida para",
            "se prefiere para",
            "recomendado para",
            "recomendada para",
            "ideal para",
            "usar siempre para",
        ];

        for marker in preference_markers {
            if let (Some(pos_a), Some(pos_b)) = (text_a.find(marker), text_b.find(marker)) {
                let suffix_a = text_a[pos_a + marker.len()..].trim();
                let suffix_b = text_b[pos_b + marker.len()..].trim();

                // Extraer las primeras palabras del contexto (ej: "proyectos pequeños")
                let context_a: String = suffix_a
                    .split_whitespace()
                    .take(3)
                    .collect::<Vec<_>>()
                    .join(" ");
                let context_b: String = suffix_b
                    .split_whitespace()
                    .take(3)
                    .collect::<Vec<_>>()
                    .join(" ");

                if !context_a.is_empty() && context_a == context_b {
                    let subject_a = text_a[..pos_a].trim();
                    let subject_b = text_b[..pos_b].trim();

                    // Si los sujetos son diferentes, hay contradicción de preferencia
                    if subject_a != subject_b && !subject_a.is_empty() && !subject_b.is_empty() {
                        let reason = format!(
                            "Preferencia en conflicto para '{}': '{}' vs '{}'",
                            context_a, subject_a, subject_b
                        );
                        let suggestion = format!(
                            "Definir contexto específico: ej. '{}' para casos simples vs '{}' para casos complejos",
                            subject_a, subject_b
                        );

                        let mut contradiction = Contradiction::new(
                            mem_a.id,
                            mem_b.id,
                            ConflictType::MutuallyExclusivePreference,
                            reason,
                        )?;
                        contradiction = contradiction.with_suggested_resolution(suggestion);
                        return Ok(Some(contradiction));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Detecta afirmaciones con polaridad invertida ("siempre" vs "nunca", "funciona" vs "falla")
    fn detect_polarity_inversion(
        mem_a: &ClusterableMemory,
        mem_b: &ClusterableMemory,
        text_a: &str,
        text_b: &str,
    ) -> Result<Option<Contradiction>, ConsolidationError> {
        let opposing_pairs = [
            ("siempre", "nunca"),
            ("funciona correctamente", "falla recurrentemente"),
            ("exitoso", "fallido"),
            ("seguro", "inseguro"),
            ("compatible", "incompatible"),
            ("rápido", "lento"),
            ("estable", "inestable"),
        ];

        for (pos, neg) in opposing_pairs {
            let (has_pos_a, has_neg_a) = (text_a.contains(pos), text_a.contains(neg));
            let (has_pos_b, has_neg_b) = (text_b.contains(pos), text_b.contains(neg));

            if (has_pos_a && has_neg_b) || (has_neg_a && has_pos_b) {
                // Verificar que compartan al menos 2 palabras clave significativas para asegurar que hablan de lo mismo
                let words_a: Vec<&str> = text_a
                    .split_whitespace()
                    .filter(|w| w.len() > 4 && *w != pos && *w != neg)
                    .collect();
                let words_b: Vec<&str> = text_b
                    .split_whitespace()
                    .filter(|w| w.len() > 4 && *w != pos && *w != neg)
                    .collect();

                let shared = words_a.iter().any(|w| words_b.contains(w));
                if shared {
                    let reason =
                        format!("Oposición lógica detectada entre afirmaciones ({pos} vs {neg})");
                    let contradiction = Contradiction::new(
                        mem_a.id,
                        mem_b.id,
                        ConflictType::DirectOpposite,
                        reason,
                    )?;
                    return Ok(Some(contradiction));
                }
            }
        }

        Ok(None)
    }
}
