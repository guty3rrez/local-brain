//! Adaptador HTTP para generación de embeddings locales vía llama.cpp (SRS §12.2, §26).

use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde::Serialize;
use tracing::{debug, error, instrument};

use brain_domain::model::DomainError;
use brain_domain::ports::{EmbeddingProvider, DEFAULT_EMBEDDING_DIMENSION};

/// Configuración para el cliente local de embeddings llama.cpp (SRS §12.2, §26).
#[derive(Debug, Clone)]
pub struct LlamaCppConfig {
    /// URL del endpoint de embeddings (e.g. "http://127.0.0.1:8081/embedding" o "/v1/embeddings").
    pub endpoint: String,
    /// Timeout máximo para la solicitud HTTP.
    pub timeout: Duration,
    /// Dimensión esperada del vector (por defecto 768).
    pub expected_dimension: usize,
}

impl Default for LlamaCppConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://127.0.0.1:8081/embedding".to_string(),
            timeout: Duration::from_secs(3),
            expected_dimension: DEFAULT_EMBEDDING_DIMENSION,
        }
    }
}

impl LlamaCppConfig {
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

    pub fn with_dimension(mut self, dimension: usize) -> Self {
        self.expected_dimension = dimension;
        self
    }
}

/// Adaptador HTTP que implementa `EmbeddingProvider` comunicándose con `llama.cpp`.
#[derive(Debug, Clone)]
pub struct LlamaCppEmbeddingProvider {
    config: LlamaCppConfig,
    client: Client,
}

impl LlamaCppEmbeddingProvider {
    pub fn new(config: LlamaCppConfig) -> Result<Self, DomainError> {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| {
                DomainError::RepositoryError(format!("Error inicializando cliente HTTP: {e}"))
            })?;

        Ok(Self { config, client })
    }

    pub fn from_endpoint(endpoint: impl Into<String>) -> Result<Self, DomainError> {
        Self::new(LlamaCppConfig::new(endpoint))
    }
}

#[derive(Debug, Serialize)]
struct LlamaRequest<'a> {
    content: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    input: Option<&'a str>,
}

fn parse_float_vec(val: &serde_json::Value) -> Option<Vec<f32>> {
    val.as_array().map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_f64().map(|f| f as f32))
            .collect()
    })
}

#[async_trait]
impl EmbeddingProvider for LlamaCppEmbeddingProvider {
    #[instrument(skip(self, text), fields(text_len = text.len(), endpoint = %self.config.endpoint))]
    async fn embed(&self, text: &str) -> Result<Vec<f32>, DomainError> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(DomainError::EmptyContent);
        }

        let body = LlamaRequest {
            content: trimmed,
            input: Some(trimmed),
        };

        debug!("Enviando solicitud de embedding a llama.cpp");
        let response = self
            .client
            .post(&self.config.endpoint)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                DomainError::RepositoryError(format!(
                    "Fallo al contactar el runtime local de llama.cpp en '{}': {e}",
                    self.config.endpoint
                ))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let err_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Sin detalle de error".to_string());
            error!(%status, %err_text, "El runtime llama.cpp devolvió un error HTTP");
            return Err(DomainError::RepositoryError(format!(
                "Runtime llama.cpp respondió con código {status}: {err_text}"
            )));
        }

        let raw_json: serde_json::Value = response.json().await.map_err(|e| {
            DomainError::RepositoryError(format!(
                "Respuesta inválida del servidor llama.cpp al deserializar JSON: {e}"
            ))
        })?;

        // 1. Formato OpenAI compatible: {"data": [{"embedding": [...]}]}
        let vector_opt = raw_json
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("embedding"))
            .and_then(parse_float_vec)
            // 2. Formato clásico llama.cpp: {"embedding": [...]}
            .or_else(|| raw_json.get("embedding").and_then(parse_float_vec))
            // 3. Formato array reciente de llama-server: [{"embedding": [[...]]}] o [{"embedding": [...]}]
            .or_else(|| {
                raw_json
                    .as_array()
                    .and_then(|arr| arr.first())
                    .and_then(|item| {
                        item.get("embedding").and_then(|emb| {
                            if let Some(nested) = emb
                                .as_array()
                                .and_then(|a| a.first())
                                .and_then(parse_float_vec)
                            {
                                Some(nested)
                            } else {
                                parse_float_vec(emb)
                            }
                        })
                    })
            });

        let vector = vector_opt.ok_or_else(|| {
            DomainError::RepositoryError(
                "El servidor llama.cpp devolvió una respuesta con estructura no reconocida (sin 'embedding' ni 'data')".to_string(),
            )
        })?;

        if vector.len() != self.config.expected_dimension {
            return Err(DomainError::RepositoryError(format!(
                "Dimensión inesperada del embedding: se esperaban {} dimensiones pero se recibieron {}",
                self.config.expected_dimension,
                vector.len()
            )));
        }

        Ok(vector)
    }

    fn dimension(&self) -> usize {
        self.config.expected_dimension
    }
}
