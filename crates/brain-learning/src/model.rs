//! Modelos y Value Objects para el aprendizaje y conocimiento candidato (SRS §15, §59, §60).

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use brain_domain::model::{Confidence, MemoryId};

use crate::errors::LearningError;

/// Límite máximo para la declaración o afirmación de conocimiento (16 KB).
pub const MAX_STATEMENT_BYTES: usize = 16 * 1024;

/// Identificador fuertemente tipado de un conocimiento candidato u observación (UUID v7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CandidateId(Uuid);

impl CandidateId {
    /// Genera un nuevo identificador UUID v7 ordenado cronológicamente.
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Crea un `CandidateId` a partir de un `Uuid` existente.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Retorna una referencia al UUID interno.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for CandidateId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CandidateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for CandidateId {
    type Err = LearningError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s)
            .map(Self)
            .map_err(|e| LearningError::InvalidId(e.to_string()))
    }
}

/// Identificador fuertemente tipado de una evidencia empírica (UUID v7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EvidenceId(Uuid);

impl EvidenceId {
    /// Genera un nuevo identificador UUID v7 ordenado cronológicamente.
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Crea un `EvidenceId` a partir de un `Uuid` existente.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Retorna una referencia al UUID interno.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for EvidenceId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EvidenceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for EvidenceId {
    type Err = LearningError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s)
            .map(Self)
            .map_err(|e| LearningError::InvalidId(e.to_string()))
    }
}

/// Etapas del ciclo de vida del conocimiento en el motor de aprendizaje (SRS §15, §59).
///
/// Flujo:
/// Experience → Observation → Candidate Knowledge → Validation → Consolidated Knowledge
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LearningStage {
    /// Observación factual de un evento puntual sin generalización ("El deployment tardó 4m").
    Observation,
    /// Creencia o hipótesis candidata que requiere más evidencia empírica ("Este stack tiene deployments lentos").
    Candidate,
    /// Conocimiento validado mediante múltiples evidencias empíricas o confirmación humana explícita.
    Validated,
    /// Conocimiento consolidado integrado en la memoria semántica de largo plazo.
    Consolidated,
    /// Hipótesis o creencia descartada o refutada por contraevidencias concluyentes.
    Rejected,
}

impl LearningStage {
    /// Retorna la representación canónica en cadena de texto.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Observation => "observation",
            Self::Candidate => "candidate",
            Self::Validated => "validated",
            Self::Consolidated => "consolidated",
            Self::Rejected => "rejected",
        }
    }

    /// Valida si una transición de etapa es permitida estructuralmente.
    pub fn can_transition_to(&self, target: Self) -> bool {
        match (self, target) {
            // Mismo estado siempre es válido
            (a, b) if *a == b => true,
            // Observation puede avanzar a Candidate o ser rechazada
            (Self::Observation, Self::Candidate) => true,
            (Self::Observation, Self::Rejected) => true,
            // Candidate puede avanzar a Validated o ser rechazada
            (Self::Candidate, Self::Validated) => true,
            (Self::Candidate, Self::Rejected) => true,
            // Validated puede consolidarse o degradarse si surge contraevidencia
            (Self::Validated, Self::Consolidated) => true,
            (Self::Validated, Self::Candidate) => true,
            (Self::Validated, Self::Rejected) => true,
            // Consolidated puede degradarse ante contradicciones
            (Self::Consolidated, Self::Candidate) => true,
            (Self::Consolidated, Self::Rejected) => true,
            // Rejected puede reactivarse como Candidate si se presenta nueva evidencia
            (Self::Rejected, Self::Candidate) => true,
            _ => false,
        }
    }
}

impl fmt::Display for LearningStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for LearningStage {
    type Err = LearningError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            "observation" => Ok(Self::Observation),
            "candidate" => Ok(Self::Candidate),
            "validated" => Ok(Self::Validated),
            "consolidated" => Ok(Self::Consolidated),
            "rejected" => Ok(Self::Rejected),
            other => Err(LearningError::InvalidStageTransition {
                from: other.to_string(),
                to: "unknown".to_string(),
                reason: format!("Etapa de aprendizaje desconocida: '{other}'"),
            }),
        }
    }
}

/// Tipos de fuente de una evidencia empírica (SRS §60).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSourceType {
    /// Validación explícita provista o verificada por un humano (máxima confiabilidad).
    Human,
    /// Resultado directo de compilador, linter, test suite o ejecución de herramientas.
    ToolExecution,
    /// Observación empírica directa reportada por un agente durante su ciclo de trabajo.
    DirectObservation,
    /// Inferencia o hipótesis generada por razonamiento de un agente (confiabilidad preliminar).
    AgentHypothesis,
}

impl EvidenceSourceType {
    /// Retorna el nombre canónico en cadena de texto.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::ToolExecution => "tool_execution",
            Self::DirectObservation => "direct_observation",
            Self::AgentHypothesis => "agent_hypothesis",
        }
    }

    /// Ponderación de calidad intrínseca de la fuente (SRS §60).
    pub fn default_weight(&self) -> f32 {
        match self {
            Self::Human => 1.0,
            Self::ToolExecution => 0.8,
            Self::DirectObservation => 0.5,
            Self::AgentHypothesis => 0.3,
        }
    }
}

impl fmt::Display for EvidenceSourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for EvidenceSourceType {
    type Err = LearningError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            "human" => Ok(Self::Human),
            "tool_execution" | "tool" | "execution" => Ok(Self::ToolExecution),
            "direct_observation" | "observation" => Ok(Self::DirectObservation),
            "agent_hypothesis" | "hypothesis" | "agent" => Ok(Self::AgentHypothesis),
            other => Err(LearningError::InvalidSourceType(other.to_string())),
        }
    }
}

/// Unidad atómica de evidencia empírica que apoya o refuta una creencia o hipótesis (SRS §15, §60).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Evidence {
    pub id: EvidenceId,
    pub candidate_id: CandidateId,
    pub source_type: EvidenceSourceType,
    pub content: String,
    pub memory_id: Option<MemoryId>,
    pub agent: Option<String>,
    pub is_supporting: bool,
    pub confidence_weight: f32,
    pub recorded_at: DateTime<Utc>,
}

impl Evidence {
    /// Crea una nueva evidencia asociada a un conocimiento candidato.
    pub fn new(
        candidate_id: CandidateId,
        source_type: EvidenceSourceType,
        content: impl Into<String>,
        is_supporting: bool,
    ) -> Result<Self, LearningError> {
        let content_str = content.into();
        let trimmed = content_str.trim();

        if trimmed.is_empty() {
            return Err(LearningError::EmptyEvidence);
        }

        let weight = source_type.default_weight();

        Ok(Self {
            id: EvidenceId::new(),
            candidate_id,
            source_type,
            content: content_str,
            memory_id: None,
            agent: None,
            is_supporting,
            confidence_weight: weight,
            recorded_at: Utc::now(),
        })
    }

    /// Asocia un identificador de memoria previo a la evidencia.
    pub fn with_memory_id(mut self, memory_id: MemoryId) -> Self {
        self.memory_id = Some(memory_id);
        self
    }

    /// Asocia el identificador del agente que recopiló la evidencia.
    pub fn with_agent(mut self, agent: impl Into<String>) -> Self {
        self.agent = Some(agent.into());
        self
    }

    /// Configura una ponderación de confianza personalizada (acotada a [0.0, 1.0]).
    pub fn with_weight(mut self, weight: f32) -> Result<Self, LearningError> {
        if !(0.0..=1.0).contains(&weight) {
            return Err(LearningError::OutOfRange(weight.to_string()));
        }
        self.confidence_weight = weight;
        Ok(self)
    }
}

/// Aggregate Root que representa una creencia candidata u observación en aprendizaje (SRS §15, §59, §60).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateKnowledge {
    pub id: CandidateId,
    pub statement: String,
    pub stage: LearningStage,
    pub confidence: Confidence,
    pub domain: Option<String>,
    pub human_validated: bool,
    pub memory_id: Option<MemoryId>,
    pub evidences: Vec<Evidence>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CandidateKnowledge {
    /// Valida que el texto de la afirmación no esté vacío y cumpla los límites de tamaño.
    pub fn validate_statement(raw: &str) -> Result<String, LearningError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(LearningError::EmptyStatement);
        }
        let bytes_len = raw.len();
        if bytes_len > MAX_STATEMENT_BYTES {
            return Err(LearningError::StatementTooLarge {
                actual: bytes_len,
                max: MAX_STATEMENT_BYTES,
            });
        }
        Ok(raw.to_string())
    }

    /// Crea un nuevo registro de observación puntual (SRS §59).
    pub fn new_observation(
        statement: impl Into<String>,
        domain: Option<String>,
        agent: Option<String>,
    ) -> Result<Self, LearningError> {
        let statement = Self::validate_statement(&statement.into())?;
        let now = Utc::now();
        let id = CandidateId::new();

        // Una observación aislada tiene confianza preliminar baja (0.3)
        let initial_confidence = Confidence::tentative();

        let mut candidate = Self {
            id,
            statement: statement.clone(),
            stage: LearningStage::Observation,
            confidence: initial_confidence,
            domain,
            human_validated: false,
            memory_id: None,
            evidences: Vec::new(),
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
        };

        // Genera la evidencia factual inicial a partir de la observación
        let mut initial_evidence =
            Evidence::new(id, EvidenceSourceType::DirectObservation, statement, true)?;
        if let Some(a) = agent {
            initial_evidence = initial_evidence.with_agent(a);
        }
        candidate.evidences.push(initial_evidence);

        Ok(candidate)
    }

    /// Crea un nuevo conocimiento candidato con una hipótesis o regla general (SRS §15, §59).
    pub fn new_candidate(
        statement: impl Into<String>,
        domain: Option<String>,
        initial_evidence: Option<Evidence>,
    ) -> Result<Self, LearningError> {
        let statement = Self::validate_statement(&statement.into())?;
        let now = Utc::now();
        let id = CandidateId::new();

        let mut evidences = Vec::new();
        if let Some(mut ev) = initial_evidence {
            ev.candidate_id = id;
            evidences.push(ev);
        }

        Ok(Self {
            id,
            statement,
            stage: LearningStage::Candidate,
            confidence: Confidence::tentative(),
            domain,
            human_validated: false,
            memory_id: None,
            evidences,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
        })
    }

    /// Retorna las evidencias que apoyan la creencia.
    pub fn supporting_evidences(&self) -> Vec<&Evidence> {
        self.evidences.iter().filter(|e| e.is_supporting).collect()
    }

    /// Retorna las evidencias que contradicen o refutan la creencia.
    pub fn contradicting_evidences(&self) -> Vec<&Evidence> {
        self.evidences.iter().filter(|e| !e.is_supporting).collect()
    }

    /// Agrega una evidencia empírica a la lista.
    pub fn add_evidence(&mut self, mut evidence: Evidence) {
        evidence.candidate_id = self.id;
        self.evidences.push(evidence);
        self.updated_at = Utc::now();
    }
}
