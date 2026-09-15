//! Puertos de Arquitectura Hexagonal y Repositorio en Memoria para pruebas unitarias.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use crate::model::{DomainError, Memory, MemoryId, MemoryStatus, MemoryType};

/// Puerto secundario para persistencia de unidades de memoria cognitiva (SRS §8, §9).
#[async_trait]
pub trait MemoryRepository: Send + Sync {
    /// Persiste un nuevo recuerdo en el almacén.
    async fn save(&self, memory: &Memory) -> Result<(), DomainError>;

    /// Recupera un recuerdo por su identificador único.
    async fn find_by_id(&self, id: &MemoryId) -> Result<Option<Memory>, DomainError>;

    /// Recupera recuerdos activos asociados a un proyecto específico.
    async fn find_by_project(
        &self,
        project: &str,
        limit: usize,
    ) -> Result<Vec<Memory>, DomainError>;

    /// Recupera recuerdos activos filtrados por tipo cognitivo.
    async fn find_by_type(
        &self,
        memory_type: MemoryType,
        limit: usize,
    ) -> Result<Vec<Memory>, DomainError>;

    /// Recupera recuerdos filtrados por su estado en el ciclo de vida (SRS §9.1, §12.3).
    async fn find_by_status(
        &self,
        status: MemoryStatus,
        limit: usize,
    ) -> Result<Vec<Memory>, DomainError>;

    /// Actualiza un recuerdo existente (concurrencia optimista y versión).
    async fn update(&self, memory: &Memory) -> Result<(), DomainError>;

    /// Marca un recuerdo como eliminado lógicamente (soft-delete).
    async fn soft_delete(&self, id: &MemoryId) -> Result<(), DomainError>;
}

/// Dimensión estándar adoptada para embeddings vectoriales de alta fidelidad (SRS §12, §25.2).
pub const DEFAULT_EMBEDDING_DIMENSION: usize = 768;

/// Puerto secundario para indexación y búsqueda semántica vectorial (SRS §12, §13).
#[async_trait]
pub trait VectorRepository: Send + Sync {
    /// Almacena o actualiza el vector de embedding asociado a una memoria.
    async fn store_embedding(&self, id: &MemoryId, embedding: &[f32]) -> Result<(), DomainError>;

    /// Busca los N recuerdos más similares a un vector de consulta, retornando pares (MemoryId, Score).
    async fn search_similar(
        &self,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(MemoryId, f32)>, DomainError>;
}

/// Puerto secundario para generación local de embeddings vectoriales (SRS §12, §38).
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Genera un vector denso de embedding a partir del texto provisto.
    async fn embed(&self, text: &str) -> Result<Vec<f32>, DomainError>;

    /// Dimensión vectorial provista por el modelo de embeddings.
    fn dimension(&self) -> usize {
        DEFAULT_EMBEDDING_DIMENSION
    }
}

/// Implementación en memoria thread-safe de `VectorRepository` con cálculo de similitud coseno pura (SRS RNF-006).
#[derive(Debug, Default, Clone)]
pub struct InMemoryVectorRepository {
    storage: Arc<RwLock<HashMap<MemoryId, Vec<f32>>>>,
}

impl InMemoryVectorRepository {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn count(&self) -> usize {
        let lock = self.storage.read().expect("Lock poisoned");
        lock.len()
    }
}

#[async_trait]
impl VectorRepository for InMemoryVectorRepository {
    async fn store_embedding(&self, id: &MemoryId, embedding: &[f32]) -> Result<(), DomainError> {
        let mut lock = self
            .storage
            .write()
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
        lock.insert(*id, embedding.to_vec());
        Ok(())
    }

    async fn search_similar(
        &self,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(MemoryId, f32)>, DomainError> {
        let lock = self
            .storage
            .read()
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        let query_norm_sq: f32 = embedding.iter().map(|x| x * x).sum();
        let query_norm = query_norm_sq.sqrt();
        if query_norm == 0.0 {
            return Ok(Vec::new());
        }

        let mut scored: Vec<(MemoryId, f32)> = lock
            .iter()
            .map(|(id, vec)| {
                let dot_prod: f32 = embedding.iter().zip(vec.iter()).map(|(a, b)| a * b).sum();
                let v_norm_sq: f32 = vec.iter().map(|x| x * x).sum();
                let v_norm = v_norm_sq.sqrt();
                let sim = if v_norm > 0.0 {
                    dot_prod / (query_norm * v_norm)
                } else {
                    0.0
                };
                (*id, sim)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        Ok(scored)
    }
}

/// Implementación en memoria determinista de `EmbeddingProvider` para testing unitario puro (SRS RNF-006).
#[derive(Debug, Clone)]
pub struct InMemoryEmbeddingProvider {
    dimension: usize,
}

impl InMemoryEmbeddingProvider {
    pub fn new() -> Self {
        Self {
            dimension: DEFAULT_EMBEDDING_DIMENSION,
        }
    }

    pub fn with_dimension(dimension: usize) -> Self {
        Self { dimension }
    }
}

impl Default for InMemoryEmbeddingProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EmbeddingProvider for InMemoryEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, DomainError> {
        if text.trim().is_empty() {
            return Err(DomainError::EmptyContent);
        }
        let mut vector = vec![0.0f32; self.dimension];
        for word in text.split_whitespace() {
            let clean = word.to_lowercase();
            let mut h: usize = 0;
            for &b in clean.as_bytes() {
                h = h.wrapping_mul(31).wrapping_add(b as usize);
            }
            vector[h % self.dimension] += 2.0;

            // n-grams de 3 caracteres para capturar raíces y similitud morfológica
            if clean.len() >= 3 {
                for window in clean.as_bytes().windows(3) {
                    let mut gh: usize = 0;
                    for &b in window {
                        gh = gh.wrapping_mul(37).wrapping_add(b as usize);
                    }
                    vector[gh % self.dimension] += 1.0;
                }
            }
        }
        let norm_sq: f32 = vector.iter().map(|x| x * x).sum();
        let norm = norm_sq.sqrt();
        if norm > 0.0 {
            for v in vector.iter_mut() {
                *v /= norm;
            }
        }
        Ok(vector)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

/// Implementación en memoria thread-safe de `MemoryRepository` para testing puro y determinista (SRS RNF-006).
#[derive(Debug, Default, Clone)]
pub struct InMemoryMemoryRepository {
    storage: Arc<RwLock<HashMap<MemoryId, Memory>>>,
}

impl InMemoryMemoryRepository {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Retorna la cantidad de memorias almacenadas en el repositorio (incluyendo archivadas).
    pub fn count(&self) -> usize {
        let lock = self.storage.read().expect("Lock poisoned");
        lock.len()
    }
}

#[async_trait]
impl MemoryRepository for InMemoryMemoryRepository {
    async fn save(&self, memory: &Memory) -> Result<(), DomainError> {
        let mut lock = self
            .storage
            .write()
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
        if lock.contains_key(&memory.id) {
            return Err(DomainError::RepositoryError(format!(
                "La memoria con id {} ya existe",
                memory.id
            )));
        }
        lock.insert(memory.id, memory.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: &MemoryId) -> Result<Option<Memory>, DomainError> {
        let lock = self
            .storage
            .read()
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
        Ok(lock.get(id).cloned())
    }

    async fn find_by_project(
        &self,
        project: &str,
        limit: usize,
    ) -> Result<Vec<Memory>, DomainError> {
        let lock = self
            .storage
            .read()
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
        let results: Vec<Memory> = lock
            .values()
            .filter(|m| m.is_active() && m.project.as_deref() == Some(project))
            .take(limit)
            .cloned()
            .collect();
        Ok(results)
    }

    async fn find_by_type(
        &self,
        memory_type: MemoryType,
        limit: usize,
    ) -> Result<Vec<Memory>, DomainError> {
        let lock = self
            .storage
            .read()
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
        let results: Vec<Memory> = lock
            .values()
            .filter(|m| m.is_active() && m.memory_type == memory_type)
            .take(limit)
            .cloned()
            .collect();
        Ok(results)
    }

    async fn find_by_status(
        &self,
        status: MemoryStatus,
        limit: usize,
    ) -> Result<Vec<Memory>, DomainError> {
        let lock = self
            .storage
            .read()
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
        let results: Vec<Memory> = lock
            .values()
            .filter(|m| m.status == status)
            .take(limit)
            .cloned()
            .collect();
        Ok(results)
    }

    async fn update(&self, memory: &Memory) -> Result<(), DomainError> {
        let mut lock = self
            .storage
            .write()
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
        if !lock.contains_key(&memory.id) {
            return Err(DomainError::RepositoryError(format!(
                "No se puede actualizar: memoria {} no encontrada",
                memory.id
            )));
        }
        lock.insert(memory.id, memory.clone());
        Ok(())
    }

    async fn soft_delete(&self, id: &MemoryId) -> Result<(), DomainError> {
        let mut lock = self
            .storage
            .write()
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
        if let Some(mem) = lock.get_mut(id) {
            mem.soft_delete()?;
            Ok(())
        } else {
            Err(DomainError::RepositoryError(format!(
                "No se puede eliminar: memoria {} no encontrada",
                id
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{MemoryContent, MemoryOrigin, MemoryStatus, Provenance};

    #[tokio::test]
    async fn in_memory_repository_crud_operations() {
        let repo = InMemoryMemoryRepository::new();
        assert_eq!(repo.count(), 0);

        let content = MemoryContent::new("Regla de arquitectura: dominio puro").unwrap();
        let prov = Provenance::new(MemoryOrigin::Observation).with_agent("agent-007");
        let memory = Memory::new_episodic(content, prov, Some("local-brain".to_string()));
        let id = memory.id;

        // Save
        repo.save(&memory).await.unwrap();
        assert_eq!(repo.count(), 1);

        // Find by ID
        let fetched = repo.find_by_id(&id).await.unwrap().expect("Debe existir");
        assert_eq!(
            fetched.content.text(),
            "Regla de arquitectura: dominio puro"
        );

        // Find by project
        let project_memories = repo.find_by_project("local-brain", 10).await.unwrap();
        assert_eq!(project_memories.len(), 1);

        // Find by type
        let episodic_memories = repo.find_by_type(MemoryType::Episodic, 10).await.unwrap();
        assert_eq!(episodic_memories.len(), 1);

        // Update
        let mut updated = fetched;
        updated.update_content(MemoryContent::new("Regla actualizada").unwrap());
        repo.update(&updated).await.unwrap();

        let refetched = repo.find_by_id(&id).await.unwrap().unwrap();
        assert_eq!(refetched.content.text(), "Regla actualizada");
        assert_eq!(refetched.version.value(), 2);

        // Soft delete
        repo.soft_delete(&id).await.unwrap();
        let deleted = repo.find_by_id(&id).await.unwrap().unwrap();
        assert_eq!(deleted.status, MemoryStatus::SoftDeleted);

        // Query by project should no longer return soft deleted memory
        let active_memories = repo.find_by_project("local-brain", 10).await.unwrap();
        assert!(active_memories.is_empty());
    }

    #[tokio::test]
    async fn in_memory_embedding_and_vector_repository_search() {
        let provider = InMemoryEmbeddingProvider::new();
        assert_eq!(provider.dimension(), DEFAULT_EMBEDDING_DIMENSION);

        let vec1 = provider
            .embed("Arquitectura Hexagonal en Rust")
            .await
            .unwrap();
        assert_eq!(vec1.len(), 768);

        let vec2 = provider
            .embed("Arquitectura Hexagonal y puertos")
            .await
            .unwrap();
        let vec3 = provider.embed("Receta de cocina para pizza").await.unwrap();

        let repo = InMemoryVectorRepository::new();
        let id1 = MemoryId::new();
        let id2 = MemoryId::new();
        let id3 = MemoryId::new();

        repo.store_embedding(&id1, &vec1).await.unwrap();
        repo.store_embedding(&id2, &vec2).await.unwrap();
        repo.store_embedding(&id3, &vec3).await.unwrap();
        assert_eq!(repo.count(), 3);

        // Búsqueda con vector similar a Arquitectura
        let query_vec = provider.embed("Hexagonal architecture").await.unwrap();
        let results = repo.search_similar(&query_vec, 3).await.unwrap();

        assert_eq!(results.len(), 3);
        // Cada resultado tiene un score entre -1.0 y 1.0
        for (_, score) in &results {
            assert!(*score >= -1.0 && *score <= 1.0);
        }
    }

    #[tokio::test]
    async fn in_memory_embedding_empty_content_error() {
        let provider = InMemoryEmbeddingProvider::new();
        let result = provider.embed("   ").await;
        assert_eq!(result, Err(DomainError::EmptyContent));
    }
}
