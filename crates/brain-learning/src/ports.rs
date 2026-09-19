//! Puertos de arquitectura hexagonal para persistencia de conocimiento candidato y aprendizaje (SRS §8, §15).

use async_trait::async_trait;

use crate::errors::LearningError;
use crate::model::{CandidateId, CandidateKnowledge, Evidence, LearningStage};

/// Puerto secundario para persistencia y recuperación de conocimiento candidato y evidencias empíricas.
#[async_trait]
pub trait LearningRepository: Send + Sync {
    /// Guarda un nuevo conocimiento candidato u observación en el almacén.
    async fn save_candidate(&self, candidate: &CandidateKnowledge) -> Result<(), LearningError>;

    /// Recupera un conocimiento candidato por su identificador único junto con sus evidencias.
    async fn find_candidate_by_id(
        &self,
        id: &CandidateId,
    ) -> Result<Option<CandidateKnowledge>, LearningError>;

    /// Recupera candidatos filtrados por su etapa actual de aprendizaje.
    async fn find_candidates_by_stage(
        &self,
        stage: LearningStage,
        limit: usize,
    ) -> Result<Vec<CandidateKnowledge>, LearningError>;

    /// Recupera candidatos asociados a un proyecto o dominio específico.
    async fn find_candidates_by_domain(
        &self,
        domain: &str,
        limit: usize,
    ) -> Result<Vec<CandidateKnowledge>, LearningError>;

    /// Busca candidatos por coincidencia de texto en la afirmación o dominio.
    async fn search_candidates(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<CandidateKnowledge>, LearningError>;

    /// Añade una evidencia empírica a un candidato existente.
    async fn add_evidence(
        &self,
        candidate_id: &CandidateId,
        evidence: &Evidence,
    ) -> Result<(), LearningError>;

    /// Recupera todas las evidencias vinculadas a un candidato.
    async fn get_evidences(
        &self,
        candidate_id: &CandidateId,
    ) -> Result<Vec<Evidence>, LearningError>;

    /// Actualiza el estado, confianza y metadatos de un candidato existente.
    async fn update_candidate(&self, candidate: &CandidateKnowledge) -> Result<(), LearningError>;

    /// Elimina un candidato y sus evidencias asociadas.
    async fn delete_candidate(&self, id: &CandidateId) -> Result<(), LearningError>;
}
