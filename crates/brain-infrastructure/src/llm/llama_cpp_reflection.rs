//! Adaptador HTTP para síntesis de reflexión y evaluación semántica vía llama.cpp (SRS §16, §17, §18).

use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

use brain_consolidation::contradiction::ContradictionDetector;
use brain_consolidation::errors::ConsolidationError;
use brain_consolidation::invariants::TENTATIVE_MAX_CONFIDENCE;
use brain_consolidation::model::{ClusterableMemory, Contradiction, Hypothesis, Pattern};
use brain_consolidation::ports::ConsolidationLlmPort;
use brain_domain::model::MemoryId;

/// Configuración para el cliente de reflexión con llama.cpp.
#[derive(Debug, Clone)]
pub struct LlamaCppReflectionConfig {
    pub endpoint: String,
    pub timeout: Duration,
    pub fallback_to_heuristic: bool,
}

impl Default for LlamaCppReflectionConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://127.0.0.1:8081/completion".to_string(),
            timeout: Duration::from_secs(4),
            fallback_to_heuristic: true,
        }
    }
}

impl LlamaCppReflectionConfig {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            ..Default::default()
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

/// Adaptador HTTP que implementa `ConsolidationLlmPort` comunicándose con `llama.cpp`
/// con tolerancia a fallos y fallback heurístico determinista (SRS §16, §17, §18).
#[derive(Debug, Clone)]
pub struct LlamaCppReflectionClient {
    config: LlamaCppReflectionConfig,
    client: Client,
}

impl LlamaCppReflectionClient {
    pub fn new(config: LlamaCppReflectionConfig) -> Result<Self, ConsolidationError> {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| {
                ConsolidationError::InferenceError(format!("Error inicializando cliente HTTP: {e}"))
            })?;

        Ok(Self { config, client })
    }

    pub fn from_endpoint(endpoint: impl Into<String>) -> Result<Self, ConsolidationError> {
        Self::new(LlamaCppReflectionConfig::new(endpoint))
    }
}

#[derive(Debug, Serialize)]
struct CompletionRequest<'a> {
    prompt: &'a str,
    n_predict: usize,
    temperature: f32,
    stop: Vec<&'a str>,
}

#[derive(Debug, Deserialize)]
struct CompletionResponse {
    content: Option<String>,
}

#[async_trait]
impl ConsolidationLlmPort for LlamaCppReflectionClient {
    async fn synthesize_hypothesis(
        &self,
        pattern: &Pattern,
        memories: &[ClusterableMemory],
    ) -> Result<Hypothesis, ConsolidationError> {
        let source_ids: Vec<MemoryId> = if !pattern.supporting_memory_ids.is_empty() {
            pattern.supporting_memory_ids.clone()
        } else {
            memories.iter().map(|m| m.id).collect()
        };

        let domain = memories.first().and_then(|m| m.project.clone());

        // Intentar llamar a llama.cpp si está configurado
        let prompt = format!(
            "Dado el siguiente patrón observado en {count} experiencias ({desc}):\n\
            Sintetiza una hipótesis técnica concisa y verificable (máximo 1 oración) como candidate knowledge:\nHipótesis:",
            count = pattern.frequency,
            desc = pattern.description
        );

        let req = CompletionRequest {
            prompt: &prompt,
            n_predict: 64,
            temperature: 0.2,
            stop: vec!["\n", "Experience", "Patrón"],
        };

        match self
            .client
            .post(&self.config.endpoint)
            .json(&req)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(data) = resp.json::<CompletionResponse>().await {
                    if let Some(text) = data.content {
                        let trimmed = text.trim();
                        if !trimmed.is_empty() {
                            return Ok(Hypothesis {
                                statement: trimmed.to_string(),
                                domain,
                                suggested_confidence: 0.35,
                                source_memory_ids: source_ids,
                            });
                        }
                    }
                }
            }
            Err(e) => {
                debug!(error = %e, "llama.cpp no respondió a solicitud de reflexión");
            }
            _ => {}
        }

        if !self.config.fallback_to_heuristic {
            return Err(ConsolidationError::InferenceError(
                "Servidor LLM local inaccesible y fallback heurístico deshabilitado".to_string(),
            ));
        }

        // Fallback heurístico determinista garantizado offline
        let fallback_statement = format!(
            "Patrón identificado: {} (observado en {} experiencias)",
            pattern.description, pattern.frequency
        );

        Ok(Hypothesis {
            statement: fallback_statement,
            domain,
            suggested_confidence: 0.30_f32.min(TENTATIVE_MAX_CONFIDENCE),
            source_memory_ids: source_ids,
        })
    }

    async fn evaluate_contradiction(
        &self,
        mem_a: &ClusterableMemory,
        mem_b: &ClusterableMemory,
    ) -> Result<Option<Contradiction>, ConsolidationError> {
        // Ejecución prioritaria del análisis heurístico y determinista (SRS §18)
        ContradictionDetector::check_contradiction(mem_a, mem_b)
    }
}
