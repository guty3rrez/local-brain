//! Implementación en memoria thread-safe de `LearningRepository` para pruebas rápidas y modo volátil.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use crate::errors::LearningError;
use crate::model::{CandidateId, CandidateKnowledge, Evidence, LearningStage};
use crate::ports::LearningRepository;

/// Repositorio en memoria concurrente y thread-safe para conocimiento candidato.
#[derive(Debug, Default, Clone)]
pub struct InMemoryLearningRepository {
    candidates: Arc<RwLock<HashMap<CandidateId, CandidateKnowledge>>>,
    evidences: Arc<RwLock<HashMap<CandidateId, Vec<Evidence>>>>,
}

impl InMemoryLearningRepository {
    /// Inicializa un nuevo almacén en memoria vacío.
    pub fn new() -> Self {
        Self {
            candidates: Arc::new(RwLock::new(HashMap::new())),
            evidences: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Retorna la cantidad total de candidatos registrados.
    pub fn candidate_count(&self) -> usize {
        self.candidates.read().unwrap().len()
    }

    /// Retorna la cantidad total de evidencias registradas.
    pub fn evidence_count(&self) -> usize {
        self.evidences
            .read()
            .unwrap()
            .values()
            .map(|v| v.len())
            .sum()
    }

    /// Limpia todos los registros almacenados.
    pub fn clear(&self) {
        self.candidates.write().unwrap().clear();
        self.evidences.write().unwrap().clear();
    }
}

#[async_trait]
impl LearningRepository for InMemoryLearningRepository {
    async fn save_candidate(&self, candidate: &CandidateKnowledge) -> Result<(), LearningError> {
        let mut candidates_guard = self.candidates.write().unwrap();
        let mut evidences_guard = self.evidences.write().unwrap();

        candidates_guard.insert(candidate.id, candidate.clone());
        evidences_guard.insert(candidate.id, candidate.evidences.clone());

        Ok(())
    }

    async fn find_candidate_by_id(
        &self,
        id: &CandidateId,
    ) -> Result<Option<CandidateKnowledge>, LearningError> {
        let candidates_guard = self.candidates.read().unwrap();
        let evidences_guard = self.evidences.read().unwrap();

        if let Some(candidate) = candidates_guard.get(id) {
            let mut full = candidate.clone();
            if let Some(evs) = evidences_guard.get(id) {
                full.evidences = evs.clone();
            }
            Ok(Some(full))
        } else {
            Ok(None)
        }
    }

    async fn find_candidates_by_stage(
        &self,
        stage: LearningStage,
        limit: usize,
    ) -> Result<Vec<CandidateKnowledge>, LearningError> {
        let candidates_guard = self.candidates.read().unwrap();
        let evidences_guard = self.evidences.read().unwrap();

        let mut results: Vec<CandidateKnowledge> = candidates_guard
            .values()
            .filter(|c| c.stage == stage)
            .take(limit)
            .cloned()
            .collect();

        for candidate in &mut results {
            if let Some(evs) = evidences_guard.get(&candidate.id) {
                candidate.evidences = evs.clone();
            }
        }

        Ok(results)
    }

    async fn find_candidates_by_domain(
        &self,
        domain: &str,
        limit: usize,
    ) -> Result<Vec<CandidateKnowledge>, LearningError> {
        let candidates_guard = self.candidates.read().unwrap();
        let evidences_guard = self.evidences.read().unwrap();

        let mut results: Vec<CandidateKnowledge> = candidates_guard
            .values()
            .filter(|c| {
                c.domain
                    .as_ref()
                    .map(|d| d.eq_ignore_ascii_case(domain))
                    .unwrap_or(false)
            })
            .take(limit)
            .cloned()
            .collect();

        for candidate in &mut results {
            if let Some(evs) = evidences_guard.get(&candidate.id) {
                candidate.evidences = evs.clone();
            }
        }

        Ok(results)
    }

    async fn search_candidates(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<CandidateKnowledge>, LearningError> {
        let q_lower = query.to_lowercase();
        let candidates_guard = self.candidates.read().unwrap();
        let evidences_guard = self.evidences.read().unwrap();

        let mut results: Vec<CandidateKnowledge> = candidates_guard
            .values()
            .filter(|c| {
                c.statement.to_lowercase().contains(&q_lower)
                    || c.domain
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&q_lower))
                        .unwrap_or(false)
            })
            .take(limit)
            .cloned()
            .collect();

        for candidate in &mut results {
            if let Some(evs) = evidences_guard.get(&candidate.id) {
                candidate.evidences = evs.clone();
            }
        }

        Ok(results)
    }

    async fn add_evidence(
        &self,
        candidate_id: &CandidateId,
        evidence: &Evidence,
    ) -> Result<(), LearningError> {
        let mut candidates_guard = self.candidates.write().unwrap();
        let mut evidences_guard = self.evidences.write().unwrap();

        if let Some(candidate) = candidates_guard.get_mut(candidate_id) {
            candidate.evidences.push(evidence.clone());
            let list = evidences_guard.entry(*candidate_id).or_default();
            list.push(evidence.clone());
            Ok(())
        } else {
            Err(LearningError::NotFound(candidate_id.to_string()))
        }
    }

    async fn get_evidences(
        &self,
        candidate_id: &CandidateId,
    ) -> Result<Vec<Evidence>, LearningError> {
        let evidences_guard = self.evidences.read().unwrap();
        Ok(evidences_guard
            .get(candidate_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn update_candidate(&self, candidate: &CandidateKnowledge) -> Result<(), LearningError> {
        let mut candidates_guard = self.candidates.write().unwrap();
        let mut evidences_guard = self.evidences.write().unwrap();

        if let std::collections::hash_map::Entry::Occupied(mut entry) =
            candidates_guard.entry(candidate.id)
        {
            entry.insert(candidate.clone());
            evidences_guard.insert(candidate.id, candidate.evidences.clone());
            Ok(())
        } else {
            Err(LearningError::NotFound(candidate.id.to_string()))
        }
    }

    async fn delete_candidate(&self, id: &CandidateId) -> Result<(), LearningError> {
        let mut candidates_guard = self.candidates.write().unwrap();
        let mut evidences_guard = self.evidences.write().unwrap();

        candidates_guard.remove(id);
        evidences_guard.remove(id);
        Ok(())
    }
}
