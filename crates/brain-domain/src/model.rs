//! Modelos de Dominio de Memoria según SRS §9 y §10.

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

/// Límite máximo para el contenido textual de una memoria (64 KB).
pub const MAX_MEMORY_CONTENT_BYTES: usize = 64 * 1024;

/// Errores de invariantes y lógica del dominio puro.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("El valor numérico debe estar en el rango [0.0, 1.0]. Valor recibido: {0}")]
    OutOfRange(String),

    #[error("El contenido de la memoria no puede estar vacío ni contener solo espacios")]
    EmptyContent,

    #[error(
        "El contenido de la memoria excede el límite de {max} bytes (recibidos: {actual} bytes)"
    )]
    ContentTooLarge { actual: usize, max: usize },

    #[error("Identificador de memoria inválido: {0}")]
    InvalidId(String),

    #[error("Versión de memoria inválida: {0}. La versión mínima permitida es 1")]
    InvalidVersion(u32),

    #[error("Transición de estado de memoria no permitida de {from:?} a {to:?}")]
    InvalidStateTransition {
        from: MemoryStatus,
        to: MemoryStatus,
    },

    #[error("Error de persistencia en repositorio: {0}")]
    RepositoryError(String),
}

/// Identificador único fuertemente tipado de una memoria (SRS §9.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MemoryId(Uuid);

impl MemoryId {
    /// Genera un nuevo identificador UUID v4 aleatorio.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Crea un `MemoryId` a partir de un `Uuid` existente.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Retorna una referencia al UUID interno.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for MemoryId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for MemoryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for MemoryId {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s)
            .map(Self)
            .map_err(|e| DomainError::InvalidId(e.to_string()))
    }
}

/// Tipos de memoria cognitiva soportados (SRS §10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    /// Memoria de trabajo volátil asociada a la sesión activa (SRS §10.1).
    Working,
    /// Experiencias y episodios concretos: contexto, acción, resultado (SRS §10.2).
    Episodic,
    /// Conocimiento generalizado, hechos y directrices comprobadas (SRS §10.3).
    Semantic,
    /// Procedimientos y métodos paso a paso (SRS §10.4).
    Procedural,
    /// Vínculos y asociaciones conceptuales (SRS §10.5).
    Associative,
}

impl MemoryType {
    /// Indica si este tipo de memoria es volátil (corta duración ligada a sesión).
    pub fn is_volatile(&self) -> bool {
        matches!(self, Self::Working)
    }

    /// Indica si este tipo de memoria requiere persistencia de largo plazo.
    pub fn is_persistent(&self) -> bool {
        !self.is_volatile()
    }
}

/// Estado del ciclo de vida de una memoria (SRS §9.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryStatus {
    /// Memoria disponible y activa para recuperación.
    Active,
    /// Memoria archivada o consolidada en abstracciones de mayor nivel.
    Archived,
    /// Memoria guardada pendiente de computar su vector de embedding.
    PendingEmbedding,
    /// Memoria en conflicto o contradicción con otra evidencia.
    Conflict,
    /// Memoria eliminada lógicamente (auditable y recuperable si aplica).
    SoftDeleted,
}

/// Grado de importancia o relevancia intrínseca (0.0 a 1.0) (SRS §9.1).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Importance(f32);

impl Importance {
    pub fn new(val: f32) -> Result<Self, DomainError> {
        if (0.0..=1.0).contains(&val) {
            Ok(Self(val))
        } else {
            Err(DomainError::OutOfRange(val.to_string()))
        }
    }

    pub fn neutral() -> Self {
        Self(0.5)
    }

    pub fn high() -> Self {
        Self(0.8)
    }

    pub fn critical() -> Self {
        Self(1.0)
    }

    pub fn value(&self) -> f32 {
        self.0
    }
}

impl Default for Importance {
    fn default() -> Self {
        Self::neutral()
    }
}

/// Nivel de confianza o evidencia empírica (0.0 a 1.0) (SRS §60).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Confidence(f32);

impl Confidence {
    pub fn new(val: f32) -> Result<Self, DomainError> {
        if (0.0..=1.0).contains(&val) {
            Ok(Self(val))
        } else {
            Err(DomainError::OutOfRange(val.to_string()))
        }
    }

    pub fn tentative() -> Self {
        Self(0.3)
    }

    pub fn verified() -> Self {
        Self(0.85)
    }

    pub fn absolute() -> Self {
        Self(1.0)
    }

    pub fn value(&self) -> f32 {
        self.0
    }
}

impl Default for Confidence {
    fn default() -> Self {
        Self::tentative()
    }
}

/// Utilidad observada de la memoria a lo largo del tiempo (0.0 a 1.0) (SRS §9.1).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Utility(f32);

impl Utility {
    pub fn new(val: f32) -> Result<Self, DomainError> {
        if (0.0..=1.0).contains(&val) {
            Ok(Self(val))
        } else {
            Err(DomainError::OutOfRange(val.to_string()))
        }
    }

    pub fn value(&self) -> f32 {
        self.0
    }

    /// Ajusta la utilidad sumando un delta positivo o negativo, acotando el resultado a [0.0, 1.0].
    pub fn adjust_by(&mut self, delta: f32) {
        let new_val = (self.0 + delta).clamp(0.0, 1.0);
        self.0 = new_val;
    }
}

impl Default for Utility {
    fn default() -> Self {
        Self(0.5)
    }
}

/// Contador de versión monotónico para concurrencia optimista y auditoría (SRS §9.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Version(u32);

impl Version {
    /// Versión inicial para una memoria recién creada (1).
    pub fn initial() -> Self {
        Self(1)
    }

    /// Crea una versión explícita. Falla si es 0.
    pub fn new(val: u32) -> Result<Self, DomainError> {
        if val >= 1 {
            Ok(Self(val))
        } else {
            Err(DomainError::InvalidVersion(val))
        }
    }

    /// Retorna la siguiente versión incrementada en 1.
    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

impl Default for Version {
    fn default() -> Self {
        Self::initial()
    }
}

/// Origen de procedencia de la captura de memoria (SRS §4.4, §9.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryOrigin {
    UserPrompt,
    ToolOutput,
    Observation,
    Reflection,
    Manual,
    FileContext,
    System,
}

/// Metadata de procedencia y trazabilidad inmutable (SRS §4.4, §9.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// Origen específico de la información.
    pub origin: MemoryOrigin,
    /// Identificador del agente autor si aplica.
    pub agent: Option<String>,
    /// Identificador de la sesión de trabajo si aplica.
    pub session_id: Option<String>,
    /// Referencia al contexto original (archivo, comando, URL, etc.).
    pub context_reference: Option<String>,
    /// Timestamp exacto de la captura.
    pub captured_at: DateTime<Utc>,
}

impl Provenance {
    pub fn new(origin: MemoryOrigin) -> Self {
        Self {
            origin,
            agent: None,
            session_id: None,
            context_reference: None,
            captured_at: Utc::now(),
        }
    }

    pub fn with_agent(mut self, agent: impl Into<String>) -> Self {
        self.agent = Some(agent.into());
        self
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context_reference = Some(context.into());
        self
    }
}

/// Contenido textual validado de la memoria con hash SHA-256 inmutable (SRS §9.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryContent {
    text: String,
    hash: String,
}

impl MemoryContent {
    /// Crea un nuevo `MemoryContent` validando que no esté vacío y no exceda 64 KB.
    /// Calcula automáticamente el hash SHA-256 para comprobación de integridad y deduplicación.
    pub fn new(raw: impl Into<String>) -> Result<Self, DomainError> {
        let text = raw.into();
        let trimmed = text.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptyContent);
        }

        let bytes_len = text.len();
        if bytes_len > MAX_MEMORY_CONTENT_BYTES {
            return Err(DomainError::ContentTooLarge {
                actual: bytes_len,
                max: MAX_MEMORY_CONTENT_BYTES,
            });
        }

        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        Ok(Self { text, hash })
    }

    /// Retorna el contenido textual.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Retorna el hash criptográfico SHA-256 en formato hexadecimal.
    pub fn hash(&self) -> &str {
        &self.hash
    }

    /// Retorna el tamaño en bytes del texto.
    pub fn byte_len(&self) -> usize {
        self.text.len()
    }
}

/// Aggregate Root que representa una unidad atómica de memoria cognitiva (SRS §9.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Memory {
    pub id: MemoryId,
    pub memory_type: MemoryType,
    pub content: MemoryContent,
    pub summary: Option<String>,
    pub source: Provenance,
    pub project: Option<String>,
    pub agent: Option<String>,
    pub importance: Importance,
    pub confidence: Confidence,
    pub utility: Utility,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_retrieved_at: Option<DateTime<Utc>>,
    pub status: MemoryStatus,
    pub version: Version,
    pub embedding: Option<Vec<f32>>,
}

impl Memory {
    /// Constructor principal que inicializa una memoria activa.
    pub fn new(content: MemoryContent, memory_type: MemoryType, source: Provenance) -> Self {
        let now = Utc::now();
        let agent = source.agent.clone();

        Self {
            id: MemoryId::new(),
            memory_type,
            content,
            summary: None,
            source,
            project: None,
            agent,
            importance: Importance::default(),
            confidence: Confidence::default(),
            utility: Utility::default(),
            created_at: now,
            updated_at: now,
            last_retrieved_at: None,
            status: MemoryStatus::Active,
            version: Version::initial(),
            embedding: None,
        }
    }

    /// Crea un recuerdo episódico (experiencia ligada a un proyecto).
    pub fn new_episodic(
        content: MemoryContent,
        source: Provenance,
        project: Option<String>,
    ) -> Self {
        let mut memory = Self::new(content, MemoryType::Episodic, source);
        memory.project = project;
        memory.importance = Importance::high();
        memory
    }

    /// Crea un recuerdo semántico (hecho o directriz generalizada comprobada).
    pub fn new_semantic(
        content: MemoryContent,
        source: Provenance,
        confidence: Confidence,
    ) -> Self {
        let mut memory = Self::new(content, MemoryType::Semantic, source);
        memory.confidence = confidence;
        memory
    }

    /// Crea una memoria de trabajo volátil asociada a una sesión.
    pub fn new_working(
        content: MemoryContent,
        source: Provenance,
        session_id: impl Into<String>,
    ) -> Self {
        let sid = session_id.into();
        let prov = source.with_session(sid.clone());
        let mut memory = Self::new(content, MemoryType::Working, prov);
        memory.importance = Importance::neutral();
        memory
    }

    /// Actualiza el contenido textual, recalculando el hash, actualizando el timestamp y elevando la versión.
    pub fn update_content(&mut self, new_content: MemoryContent) {
        self.content = new_content;
        self.updated_at = Utc::now();
        self.version = self.version.next();
    }

    /// Registra un acceso o recuperación de la memoria, actualizando su timestamp y aumentando su utilidad.
    pub fn mark_retrieved(&mut self) {
        self.last_retrieved_at = Some(Utc::now());
        self.utility.adjust_by(0.05);
    }

    /// Asigna un resumen sintetizado de la memoria.
    pub fn set_summary(&mut self, summary: impl Into<String>) {
        self.summary = Some(summary.into());
        self.updated_at = Utc::now();
    }

    /// Asigna el vector de embeddings generado para búsqueda semántica (pgvector).
    pub fn assign_embedding(&mut self, embedding: Vec<f32>) {
        self.embedding = Some(embedding);
        self.updated_at = Utc::now();
        if self.status == MemoryStatus::PendingEmbedding {
            self.status = MemoryStatus::Active;
        }
    }

    /// Archiva la memoria. Solo memorias activas pueden archivarse.
    pub fn archive(&mut self) -> Result<(), DomainError> {
        if self.status == MemoryStatus::SoftDeleted {
            return Err(DomainError::InvalidStateTransition {
                from: self.status,
                to: MemoryStatus::Archived,
            });
        }
        self.status = MemoryStatus::Archived;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Marca la memoria como eliminada lógicamente (soft-delete).
    pub fn soft_delete(&mut self) -> Result<(), DomainError> {
        self.status = MemoryStatus::SoftDeleted;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Restaura una memoria archivada o eliminada lógicamente a estado Activo.
    pub fn restore(&mut self) -> Result<(), DomainError> {
        self.status = MemoryStatus::Active;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Comprueba si la memoria está activa para consultas operativas normales.
    pub fn is_active(&self) -> bool {
        self.status == MemoryStatus::Active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn importance_valid_range() {
        assert!(Importance::new(0.0).is_ok());
        assert!(Importance::new(0.5).is_ok());
        assert!(Importance::new(1.0).is_ok());
        assert!(Importance::new(-0.1).is_err());
        assert!(Importance::new(1.01).is_err());
    }

    #[test]
    fn confidence_valid_range() {
        assert!(Confidence::new(0.85).is_ok());
        assert!(Confidence::new(-0.01).is_err());
        assert!(Confidence::new(1.5).is_err());
    }

    #[test]
    fn utility_adjust_clamping() {
        let mut u = Utility::new(0.9).unwrap();
        u.adjust_by(0.2);
        assert_eq!(u.value(), 1.0);

        u.adjust_by(-1.5);
        assert_eq!(u.value(), 0.0);
    }

    #[test]
    fn memory_content_invariants() {
        assert_eq!(MemoryContent::new(""), Err(DomainError::EmptyContent));
        assert_eq!(
            MemoryContent::new("   \n\t  "),
            Err(DomainError::EmptyContent)
        );

        let valid =
            MemoryContent::new("Decidimos usar Rust por cero costo de abstracción").unwrap();
        assert_eq!(
            valid.text(),
            "Decidimos usar Rust por cero costo de abstracción"
        );
        assert_eq!(valid.hash().len(), 64);

        // Content too large
        let oversized = "a".repeat(MAX_MEMORY_CONTENT_BYTES + 1);
        assert!(matches!(
            MemoryContent::new(oversized),
            Err(DomainError::ContentTooLarge { .. })
        ));
    }

    #[test]
    fn memory_id_from_str_and_display() {
        let id = MemoryId::new();
        let s = id.to_string();
        let parsed: MemoryId = s.parse().unwrap();
        assert_eq!(id, parsed);

        assert!(MemoryId::from_str("invalido").is_err());
    }

    #[test]
    fn memory_lifecycle_and_versioning() {
        let content = MemoryContent::new("Arquitectura Hexagonal en Rust").unwrap();
        let prov = Provenance::new(MemoryOrigin::Observation).with_agent("antigravity");
        let mut memory = Memory::new(content, MemoryType::Semantic, prov);

        assert_eq!(memory.version.value(), 1);
        assert_eq!(memory.status, MemoryStatus::Active);
        assert!(memory.is_active());
        assert!(memory.last_retrieved_at.is_none());

        // Mark retrieved
        memory.mark_retrieved();
        assert!(memory.last_retrieved_at.is_some());
        assert!(memory.utility.value() > 0.5);

        // Update content
        let new_content = MemoryContent::new("Hexagonal y Clean Architecture en Rust").unwrap();
        memory.update_content(new_content.clone());
        assert_eq!(memory.version.value(), 2);
        assert_eq!(memory.content, new_content);

        // Archive & soft delete
        assert!(memory.archive().is_ok());
        assert_eq!(memory.status, MemoryStatus::Archived);
        assert!(!memory.is_active());

        assert!(memory.soft_delete().is_ok());
        assert_eq!(memory.status, MemoryStatus::SoftDeleted);

        assert!(memory.restore().is_ok());
        assert_eq!(memory.status, MemoryStatus::Active);
    }
}
