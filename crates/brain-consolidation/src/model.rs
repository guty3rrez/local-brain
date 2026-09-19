//! Modelos de Dominio para Consolidación, Reflexión y Contradicciones (SRS §16, §17, §18).

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use brain_domain::model::MemoryId;
use brain_learning::model::CandidateId;

use crate::errors::ConsolidationError;

/// Identificador único fuertemente tipado de un conflicto de conocimiento (SRS §18).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ConflictId(Uuid);

impl ConflictId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for ConflictId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ConflictId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for ConflictId {
    type Err = ConsolidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s)
            .map(ConflictId)
            .map_err(|_| ConsolidationError::InvalidConflictId(s.to_string()))
    }
}

/// Tipo o naturaleza del conflicto cognitivo (SRS §18).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictType {
    /// Oposición lógica directa o polaridad invertida ("siempre" vs "nunca", "funciona" vs "falla").
    DirectOpposite,
    /// Preferencias tecnológicas o metodológicas mutuamente excluyentes (SRS §18: Supabase vs .NET).
    MutuallyExclusivePreference,
    /// Resultados empíricos contradictorios ante condiciones o contextos idénticos.
    ConditionalContradiction,
}

impl ConflictType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DirectOpposite => "direct_opposite",
            Self::MutuallyExclusivePreference => "mutually_exclusive_preference",
            Self::ConditionalContradiction => "conditional_contradiction",
        }
    }
}

impl FromStr for ConflictType {
    type Err = ConsolidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            "direct_opposite" => Ok(Self::DirectOpposite),
            "mutually_exclusive_preference" => Ok(Self::MutuallyExclusivePreference),
            "conditional_contradiction" => Ok(Self::ConditionalContradiction),
            other => Err(ConsolidationError::RepositoryError(format!(
                "Tipo de conflicto desconocido: {other}"
            ))),
        }
    }
}

/// Estado del ciclo de vida de un conflicto de conocimiento (SRS §18).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictStatus {
    /// Conflicto activo pendiente de resolución humana o clarificación contextual.
    Pending,
    /// Conflicto resuelto formalmente (se aportó contexto o se actualizó a SUPERSEDES).
    Resolved,
    /// Conflicto desestimado o considerado no bloqueante tras análisis.
    Dismissed,
}

impl ConflictStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Resolved => "resolved",
            Self::Dismissed => "dismissed",
        }
    }
}

impl FromStr for ConflictStatus {
    type Err = ConsolidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            "pending" => Ok(Self::Pending),
            "resolved" => Ok(Self::Resolved),
            "dismissed" => Ok(Self::Dismissed),
            other => Err(ConsolidationError::RepositoryError(format!(
                "Estado de conflicto desconocido: {other}"
            ))),
        }
    }
}

/// Representación de una contradicción detectada entre dos piezas de conocimiento (SRS §18).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contradiction {
    pub id: ConflictId,
    pub source_memory_id: MemoryId,
    pub conflicting_memory_id: MemoryId,
    pub conflict_type: ConflictType,
    pub reason: String,
    pub suggested_resolution: Option<String>,
    pub status: ConflictStatus,
    pub resolution_context: Option<String>,
    pub detected_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
}

impl Contradiction {
    pub fn new(
        source_memory_id: MemoryId,
        conflicting_memory_id: MemoryId,
        conflict_type: ConflictType,
        reason: impl Into<String>,
    ) -> Result<Self, ConsolidationError> {
        if source_memory_id == conflicting_memory_id {
            return Err(ConsolidationError::SelfContradiction(
                source_memory_id.to_string(),
            ));
        }

        Ok(Self {
            id: ConflictId::new(),
            source_memory_id,
            conflicting_memory_id,
            conflict_type,
            reason: reason.into(),
            suggested_resolution: None,
            status: ConflictStatus::Pending,
            resolution_context: None,
            detected_at: Utc::now(),
            resolved_at: None,
            metadata: serde_json::json!({}),
        })
    }

    pub fn with_suggested_resolution(mut self, suggestion: impl Into<String>) -> Self {
        self.suggested_resolution = Some(suggestion.into());
        self
    }

    pub fn resolve(&mut self, context: impl Into<String>) -> Result<(), ConsolidationError> {
        if self.status == ConflictStatus::Resolved {
            return Err(ConsolidationError::ConflictAlreadyResolved(self.id));
        }
        self.status = ConflictStatus::Resolved;
        self.resolution_context = Some(context.into());
        self.resolved_at = Some(Utc::now());
        Ok(())
    }
}

/// Identificador de un grupo/cluster de memorias afines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ClusterId(Uuid);

impl ClusterId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for ClusterId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ClusterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Resumen de memoria apta para clustering y análisis de reflexión.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClusterableMemory {
    pub id: MemoryId,
    pub content: String,
    pub project: Option<String>,
    pub memory_type: String,
    pub embedding: Option<Vec<f32>>,
    pub created_at: DateTime<Utc>,
}

/// Grupo de memorias afines resultante del proceso de clustering (SRS §16, §17).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryCluster {
    pub id: ClusterId,
    pub project: Option<String>,
    pub memory_ids: Vec<MemoryId>,
    pub summary: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl MemoryCluster {
    pub fn new(project: Option<String>, memory_ids: Vec<MemoryId>) -> Self {
        Self {
            id: ClusterId::new(),
            project,
            memory_ids,
            summary: None,
            created_at: Utc::now(),
        }
    }
}

/// Tipología de patrones reconocidos a través de experiencias agrupadas (SRS §17).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternType {
    /// Fallo o error sistemático que se repite bajo condiciones similares.
    RecurringFailure,
    /// Estrategia o acción que produce resultados exitosos reiteradamente.
    RecurringSuccess,
    /// Regla o directriz condicionada al entorno o tecnología.
    ConditionalRule,
    /// Inclinación o preferencia metodológica derivada de experiencias.
    PreferencePattern,
    /// Correlación o coincidencia factual recurrente.
    CorrelatedObservation,
}

impl PatternType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RecurringFailure => "recurring_failure",
            Self::RecurringSuccess => "recurring_success",
            Self::ConditionalRule => "conditional_rule",
            Self::PreferencePattern => "preference_pattern",
            Self::CorrelatedObservation => "correlated_observation",
        }
    }
}

/// Patrón detectado durante la reflexión (SRS §16, §17).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pattern {
    pub pattern_type: PatternType,
    pub description: String,
    pub frequency: usize,
    pub supporting_memory_ids: Vec<MemoryId>,
}

/// Hipótesis candidata generada a partir de un patrón (SRS §17).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hypothesis {
    pub statement: String,
    pub domain: Option<String>,
    pub suggested_confidence: f32,
    pub source_memory_ids: Vec<MemoryId>,
}

/// Informe completo del resultado de un proceso de reflexión o consolidación (SRS §17).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReflectionReport {
    pub run_id: Uuid,
    pub project: Option<String>,
    pub memories_analyzed: usize,
    pub clusters_formed: usize,
    pub patterns_detected: Vec<Pattern>,
    pub hypotheses: Vec<Hypothesis>,
    pub conflicts_detected: Vec<Contradiction>,
    pub created_candidates: Vec<CandidateId>,
    pub summary: String,
    pub executed_at: DateTime<Utc>,
}
