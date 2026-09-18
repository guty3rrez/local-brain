//! Modelos de datos especializados para los 5 tipos cognitivos de memoria (SRS §10, §68).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::model::{Confidence, DomainError, MemoryId};

/// Datos específicos de una memoria de trabajo volátil asociada a sesión (SRS §10.1, F4-01).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkingMemoryData {
    /// Identificador de la sesión de trabajo activa a la que está ligada la memoria.
    pub session_id: String,
    /// Objetivo o meta que el agente intenta resolver durante la sesión.
    pub goal: Option<String>,
    /// Hipótesis activas de depuración o trabajo formuladas por el agente.
    pub hypotheses: Vec<String>,
    /// Tiempo de vida en segundos antes de expirar (opcional).
    pub ttl_seconds: Option<u64>,
}

impl WorkingMemoryData {
    pub fn new(session_id: impl Into<String>) -> Result<Self, DomainError> {
        let sid = session_id.into();
        if sid.trim().is_empty() {
            return Err(DomainError::InvalidSessionId);
        }
        Ok(Self {
            session_id: sid,
            goal: None,
            hypotheses: Vec::new(),
            ttl_seconds: None,
        })
    }

    pub fn with_ttl(mut self, ttl_seconds: u64) -> Result<Self, DomainError> {
        if ttl_seconds == 0 {
            return Err(DomainError::InvalidTtl(0));
        }
        self.ttl_seconds = Some(ttl_seconds);
        Ok(self)
    }

    pub fn with_goal(mut self, goal: impl Into<String>) -> Self {
        self.goal = Some(goal.into());
        self
    }

    pub fn with_hypotheses(mut self, hypotheses: Vec<String>) -> Self {
        self.hypotheses = hypotheses;
        self
    }

    pub fn add_hypothesis(&mut self, hypothesis: impl Into<String>) {
        self.hypotheses.push(hypothesis.into());
    }
}

/// Datos estructurados de una experiencia episódica concreta (SRS §10.2, F4-02).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpisodicMemoryData {
    /// Contexto o situación que motivó la acción.
    pub context: String,
    /// Acción o decisión concreta adoptada por el agente.
    pub action: String,
    /// Resultado u observación empírica obtenida tras la acción.
    pub outcome: String,
    /// Proyecto asociado a la experiencia.
    pub project: String,
    /// Agente autor de la experiencia.
    pub agent: String,
    /// Momento temporal en el que ocurrió el episodio.
    pub occurred_at: DateTime<Utc>,
}

impl EpisodicMemoryData {
    pub fn new(
        project: impl Into<String>,
        agent: impl Into<String>,
        context: impl Into<String>,
        action: impl Into<String>,
        outcome: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let proj = project.into();
        let ag = agent.into();
        let ctx = context.into();
        let act = action.into();
        let out = outcome.into();

        if proj.trim().is_empty() {
            return Err(DomainError::EmptyField("project".to_string()));
        }
        if ag.trim().is_empty() {
            return Err(DomainError::EmptyField("agent".to_string()));
        }
        if ctx.trim().is_empty() {
            return Err(DomainError::EmptyField("context".to_string()));
        }
        if act.trim().is_empty() {
            return Err(DomainError::EmptyField("action".to_string()));
        }
        if out.trim().is_empty() {
            return Err(DomainError::EmptyField("outcome".to_string()));
        }

        Ok(Self {
            project: proj,
            agent: ag,
            context: ctx,
            action: act,
            outcome: out,
            occurred_at: Utc::now(),
        })
    }

    /// Genera una síntesis textual descriptiva y coherente para indexación y búsqueda semántica.
    pub fn to_content_text(&self) -> String {
        format!(
            "Contexto: {}\nAcción: {}\nResultado: {}\nProyecto: {}\nAgente: {}",
            self.context, self.action, self.outcome, self.project, self.agent
        )
    }
}

/// Datos estructurados de conocimiento generalizado y comprobado (SRS §10.3, F4-03).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticMemoryData {
    /// Afirmación, directriz, principio o hecho generalizado.
    pub statement: String,
    /// Identificadores de memorias o evidencias de origen que respaldan esta afirmación (SRS §60).
    pub evidence_ids: Vec<MemoryId>,
    /// Área o dominio de conocimiento (ej. "arquitectura", "base_de_datos", "seguridad").
    pub domain_area: Option<String>,
    /// Fecha de última validación o auditoría empírica.
    pub last_validated_at: Option<DateTime<Utc>>,
}

impl SemanticMemoryData {
    pub fn new(
        statement: impl Into<String>,
        confidence: Confidence,
        evidence_ids: Vec<MemoryId>,
    ) -> Result<Self, DomainError> {
        let stmt = statement.into();
        if stmt.trim().is_empty() {
            return Err(DomainError::EmptyContent);
        }

        // Invariante de evidencia mínima para alta confianza (SRS §60, DoD F4-03):
        // Confianza >= 0.8 exige obligatoriamente al menos una evidencia de respaldo.
        if confidence.value() >= 0.8 && evidence_ids.is_empty() {
            return Err(DomainError::InsufficientEvidenceForHighConfidence {
                confidence: confidence.value().to_string(),
                minimum_required: 1,
            });
        }

        Ok(Self {
            statement: stmt,
            evidence_ids,
            domain_area: None,
            last_validated_at: Some(Utc::now()),
        })
    }

    pub fn with_domain_area(mut self, domain_area: impl Into<String>) -> Self {
        self.domain_area = Some(domain_area.into());
        self
    }

    pub fn add_evidence(&mut self, evidence_id: MemoryId) {
        if !self.evidence_ids.contains(&evidence_id) {
            self.evidence_ids.push(evidence_id);
            self.last_validated_at = Some(Utc::now());
        }
    }
}

/// Paso atómico de un procedimiento técnico estructurado (SRS §10.4, F4-04).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcedureStep {
    /// Posición ordinal secuencial (1, 2, 3...).
    pub step_number: u32,
    /// Descripción clara de la tarea a ejecutar en este paso.
    pub description: String,
    /// Comando de terminal o script asociado (opcional).
    pub command: Option<String>,
    /// Tipo de acción (ej. "command", "edit", "verify", "manual").
    pub action_type: Option<String>,
}

impl ProcedureStep {
    pub fn new(step_number: u32, description: impl Into<String>) -> Result<Self, DomainError> {
        let desc = description.into();
        if desc.trim().is_empty() {
            return Err(DomainError::EmptyField(format!(
                "step {step_number} description"
            )));
        }
        if step_number == 0 {
            return Err(DomainError::InvalidStepSequence {
                expected: 1,
                actual: 0,
            });
        }
        Ok(Self {
            step_number,
            description: desc,
            command: None,
            action_type: None,
        })
    }

    pub fn with_command(mut self, cmd: impl Into<String>) -> Self {
        self.command = Some(cmd.into());
        self.action_type = Some("command".to_string());
        self
    }

    pub fn with_action_type(mut self, action_type: impl Into<String>) -> Self {
        self.action_type = Some(action_type.into());
        self
    }
}

/// Procedimiento técnico estructurado paso a paso (SRS §10.4, F4-04).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProceduralMemoryData {
    /// Nombre descriptivo del procedimiento o receta.
    pub name: String,
    /// Meta u objetivo perseguido con este procedimiento.
    pub goal: String,
    /// Secuencia ordenada y validada de pasos (1..N sin huecos).
    pub steps: Vec<ProcedureStep>,
    /// Versión monotónica de la secuencia de pasos.
    pub step_version: u32,
}

impl ProceduralMemoryData {
    pub fn new(
        name: impl Into<String>,
        goal: impl Into<String>,
        steps: Vec<ProcedureStep>,
    ) -> Result<Self, DomainError> {
        let nm = name.into();
        let gl = goal.into();

        if nm.trim().is_empty() {
            return Err(DomainError::EmptyField("procedure name".to_string()));
        }
        if gl.trim().is_empty() {
            return Err(DomainError::EmptyField("procedure goal".to_string()));
        }
        if steps.is_empty() {
            return Err(DomainError::EmptyProcedureSteps);
        }

        Self::validate_steps_sequence(&steps)?;

        Ok(Self {
            name: nm,
            goal: gl,
            steps,
            step_version: 1,
        })
    }

    /// Valida que la secuencia de pasos comience en 1 y sea estrictamente consecutiva.
    pub fn validate_steps_sequence(steps: &[ProcedureStep]) -> Result<(), DomainError> {
        for (idx, step) in steps.iter().enumerate() {
            let expected = (idx + 1) as u32;
            if step.step_number != expected {
                return Err(DomainError::InvalidStepSequence {
                    expected,
                    actual: step.step_number,
                });
            }
        }
        Ok(())
    }

    /// Agrega un nuevo paso al final de la secuencia incrementando la versión.
    pub fn append_step(&mut self, description: impl Into<String>) -> Result<(), DomainError> {
        let next_number = (self.steps.len() + 1) as u32;
        let step = ProcedureStep::new(next_number, description)?;
        self.steps.push(step);
        self.step_version += 1;
        Ok(())
    }

    /// Genera la representación textual ordenada del procedimiento.
    pub fn to_content_text(&self) -> String {
        let mut out = format!(
            "Procedimiento: {}\nObjetivo: {}\nPasos:\n",
            self.name, self.goal
        );
        for step in &self.steps {
            out.push_str(&format!("{}. {}\n", step.step_number, step.description));
            if let Some(cmd) = &step.command {
                out.push_str(&format!("   Comando: {}\n", cmd));
            }
        }
        out
    }
}

/// Vínculo conceptual o relación semántica entre entidades (SRS §10.5, F4-05).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssociativeMemoryData {
    /// Concepto o entidad de origen (ej. "Flutter").
    pub source_concept: String,
    /// Concepto o entidad de destino (ej. "Riverpod").
    pub target_concept: String,
    /// Predicado o tipo de relación (ej. "uses", "compatible_with", "used_in").
    pub predicate: String,
    /// Fuerza o relevancia asociativa normalizada en [0.0, 1.0].
    pub strength: f32,
    /// Contexto o justificación de la asociación (opcional).
    pub context: Option<String>,
}

impl AssociativeMemoryData {
    pub fn new(
        source_concept: impl Into<String>,
        target_concept: impl Into<String>,
        predicate: impl Into<String>,
        strength: f32,
    ) -> Result<Self, DomainError> {
        let src = source_concept.into().trim().to_string();
        let dst = target_concept.into().trim().to_string();
        let pred = predicate.into().trim().to_string();

        if src.is_empty() {
            return Err(DomainError::EmptyField("source_concept".to_string()));
        }
        if dst.is_empty() {
            return Err(DomainError::EmptyField("target_concept".to_string()));
        }
        if pred.is_empty() {
            return Err(DomainError::EmptyField("predicate".to_string()));
        }
        if src.eq_ignore_ascii_case(&dst) {
            return Err(DomainError::InvalidAssociation(format!(
                "No se permiten auto-asociaciones sobre el mismo concepto: {src}"
            )));
        }
        if !(0.0..=1.0).contains(&strength) {
            return Err(DomainError::OutOfRange(strength.to_string()));
        }

        Ok(Self {
            source_concept: src,
            target_concept: dst,
            predicate: pred,
            strength,
            context: None,
        })
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    pub fn to_content_text(&self) -> String {
        let mut out = format!(
            "Asociación: {} --[{}]--> {} (Fuerza: {:.2})",
            self.source_concept, self.predicate, self.target_concept, self.strength
        );
        if let Some(ctx) = &self.context {
            out.push_str(&format!("\nContexto: {ctx}"));
        }
        out
    }
}

/// Contenedor unificado para la metadata especializada de los 5 tipos cognitivos.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MemoryTypeData {
    Working(WorkingMemoryData),
    Episodic(EpisodicMemoryData),
    Semantic(SemanticMemoryData),
    Procedural(ProceduralMemoryData),
    Associative(AssociativeMemoryData),
    #[default]
    Generic,
}
