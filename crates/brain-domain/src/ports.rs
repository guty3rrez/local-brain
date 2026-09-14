//! Puertos de Arquitectura Hexagonal y Repositorio en Memoria para pruebas unitarias.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use crate::model::{DomainError, Memory, MemoryId, MemoryType};

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

    /// Actualiza un recuerdo existente (concurrencia optimista y versión).
    async fn update(&self, memory: &Memory) -> Result<(), DomainError>;

    /// Marca un recuerdo como eliminado lógicamente (soft-delete).
    async fn soft_delete(&self, id: &MemoryId) -> Result<(), DomainError>;
}

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
}
