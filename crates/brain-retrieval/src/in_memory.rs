//! Adaptador en memoria de búsqueda léxica para testing unitario puro y CI aislado (SRS RNF-006).

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use brain_domain::model::{DomainError, MemoryId};

use crate::ports::FullTextSearchRepository;

/// Repositorio en memoria thread-safe que implementa `FullTextSearchRepository`
/// mediante tokenización léxica y conteo de frecuencias normalizado.
#[derive(Debug, Default, Clone)]
pub struct InMemoryFullTextSearchRepository {
    documents: Arc<RwLock<HashMap<MemoryId, String>>>,
}

impl InMemoryFullTextSearchRepository {
    /// Inicializa un nuevo almacén de documentos en memoria.
    pub fn new() -> Self {
        Self {
            documents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Indexa o actualiza el texto de un recuerdo en el almacén léxico en memoria.
    pub fn index_document(&self, id: MemoryId, text: &str) {
        let mut lock = self.documents.write().expect("Lock poisoned");
        lock.insert(id, text.to_lowercase());
    }

    /// Limpia todos los documentos almacenados.
    pub fn clear(&self) {
        let mut lock = self.documents.write().expect("Lock poisoned");
        lock.clear();
    }
}

#[async_trait]
impl FullTextSearchRepository for InMemoryFullTextSearchRepository {
    async fn search_fulltext(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<(MemoryId, f32)>, DomainError> {
        let lock = self
            .documents
            .read()
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        let query_tokens: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .filter(|s| s.len() > 1)
            .map(|s| s.to_string())
            .collect();

        if query_tokens.is_empty() {
            return Ok(Vec::new());
        }

        let mut scored_results = Vec::new();

        for (id, doc_text) in lock.iter() {
            let doc_tokens: Vec<&str> = doc_text.split_whitespace().collect();
            let total_doc_words = doc_tokens.len().max(1) as f32;

            let mut matched_tokens = 0;
            let mut match_frequency = 0;

            for q_tok in &query_tokens {
                let occurrences = doc_tokens.iter().filter(|&&w| w.contains(q_tok)).count();
                if occurrences > 0 {
                    matched_tokens += 1;
                    match_frequency += occurrences;
                }
            }

            if matched_tokens > 0 {
                // Cálculo simple de cobertura de tokens y densidad de términos
                let coverage = matched_tokens as f32 / query_tokens.len() as f32;
                let density = (match_frequency as f32 / total_doc_words).min(1.0);
                let score = (coverage * 0.7 + density * 0.3).clamp(0.01, 1.0);

                scored_results.push((*id, score));
            }
        }

        scored_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored_results.truncate(limit);

        Ok(scored_results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_fts_matching() {
        let fts = InMemoryFullTextSearchRepository::new();
        let id1 = MemoryId::new();
        let id2 = MemoryId::new();

        fts.index_document(id1, "Arquitectura Hexagonal en Rust para Local Brain");
        fts.index_document(id2, "Búsqueda vectorial con pgvector y embeddings");

        let results = fts.search_fulltext("hexagonal rust", 5).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, id1);
        assert!(results[0].1 > 0.5);

        let results_common = fts.search_fulltext("pgvector", 5).await.unwrap();
        assert_eq!(results_common.len(), 1);
        assert_eq!(results_common[0].0, id2);
    }
}
