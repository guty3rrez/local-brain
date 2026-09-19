//! Implementaciones puras en memoria para tests unitarios y ejecución sin persistencia (SRS §16, §17, §18, RNF-006).

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use brain_domain::model::MemoryId;

use crate::contradiction::ContradictionDetector;
use crate::errors::ConsolidationError;
use crate::invariants::TENTATIVE_MAX_CONFIDENCE;
use crate::model::{
    ClusterableMemory, ConflictId, ConflictStatus, Contradiction, Hypothesis, Pattern,
    ReflectionReport,
};
use crate::ports::{ConflictRepository, ConsolidationLlmPort};

/// Repositorio de conflictos en memoria thread-safe.
#[derive(Debug, Clone, Default)]
pub struct InMemoryConflictRepository {
    conflicts: Arc<RwLock<HashMap<ConflictId, Contradiction>>>,
    runs: Arc<RwLock<Vec<ReflectionReport>>>,
}

impl InMemoryConflictRepository {
    pub fn new() -> Self {
        Self {
            conflicts: Arc::new(RwLock::new(HashMap::new())),
            runs: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn count(&self) -> usize {
        self.conflicts.read().unwrap().len()
    }
}

#[async_trait]
impl ConflictRepository for InMemoryConflictRepository {
    async fn save_conflict(&self, conflict: &Contradiction) -> Result<(), ConsolidationError> {
        let mut store = self.conflicts.write().unwrap();
        store.insert(conflict.id, conflict.clone());
        Ok(())
    }

    async fn find_conflict_by_id(
        &self,
        id: &ConflictId,
    ) -> Result<Option<Contradiction>, ConsolidationError> {
        let store = self.conflicts.read().unwrap();
        Ok(store.get(id).cloned())
    }

    async fn find_by_memory_pair(
        &self,
        mem_a: &MemoryId,
        mem_b: &MemoryId,
    ) -> Result<Option<Contradiction>, ConsolidationError> {
        let store = self.conflicts.read().unwrap();
        let found = store.values().find(|c| {
            (c.source_memory_id == *mem_a && c.conflicting_memory_id == *mem_b)
                || (c.source_memory_id == *mem_b && c.conflicting_memory_id == *mem_a)
        });
        Ok(found.cloned())
    }

    async fn list_pending_conflicts(
        &self,
        _project: Option<&str>,
    ) -> Result<Vec<Contradiction>, ConsolidationError> {
        let store = self.conflicts.read().unwrap();
        let list: Vec<Contradiction> = store
            .values()
            .filter(|c| c.status == ConflictStatus::Pending)
            .cloned()
            .collect();
        Ok(list)
    }

    async fn update_conflict(&self, conflict: &Contradiction) -> Result<(), ConsolidationError> {
        let mut store = self.conflicts.write().unwrap();
        if !store.contains_key(&conflict.id) {
            return Err(ConsolidationError::ConflictNotFound(conflict.id));
        }
        store.insert(conflict.id, conflict.clone());
        Ok(())
    }

    async fn record_consolidation_run(
        &self,
        report: &ReflectionReport,
    ) -> Result<(), ConsolidationError> {
        let mut runs = self.runs.write().unwrap();
        runs.push(report.clone());
        Ok(())
    }
}

/// Simulación determinista del puerto LLM para consolidación y contradicciones.
#[derive(Debug, Clone, Default)]
pub struct MockConsolidationLlm {
    pub default_confidence: f32,
}

impl MockConsolidationLlm {
    pub fn new() -> Self {
        Self {
            default_confidence: 0.35,
        }
    }
}

#[async_trait]
impl ConsolidationLlmPort for MockConsolidationLlm {
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
        let statement = format!(
            "Hipótesis consolidada: {} (frecuencia: {})",
            pattern.description, pattern.frequency
        );

        let confidence = self.default_confidence.min(TENTATIVE_MAX_CONFIDENCE);

        Ok(Hypothesis {
            statement,
            domain,
            suggested_confidence: confidence,
            source_memory_ids: source_ids,
        })
    }

    async fn evaluate_contradiction(
        &self,
        mem_a: &ClusterableMemory,
        mem_b: &ClusterableMemory,
    ) -> Result<Option<Contradiction>, ConsolidationError> {
        ContradictionDetector::check_contradiction(mem_a, mem_b)
    }
}
