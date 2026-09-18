//! Modelos de Dominio de Grafo de Conocimiento y Relaciones según SRS §11 y §25.

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use brain_domain::model::MemoryId;

use crate::errors::GraphError;

/// Tipos de relación semántica iniciales soportados en el grafo de conocimiento (SRS §11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RelationType {
    /// Relación conceptual asociativa o temática general.
    RelatedTo,
    /// Concepto, herramienta o tecnología utilizada en un proyecto o componente.
    UsedIn,
    /// Causalidad directa (un suceso o decisión provocó otro).
    CausedBy,
    /// Solución a un error, desafío o requerimiento específico.
    Solves,
    /// Contradicción o incompatibilidad lógica con otro conocimiento o creencia.
    Contradicts,
    /// Reemplazo o superación de una decisión/memoria por una más reciente o mejorada.
    Supersedes,
    /// Origen o derivación conceptual a partir de una experiencia previa.
    DerivedFrom,
    /// Dependencia técnica, arquitectónica o procedural estricta.
    DependsOn,
    /// Preferencia explícita sobre una alternativa tecnológica o diseño.
    Prefers,
    /// Antipatrón, práctica o decisión expresamente desaconsejada.
    Avoid,
}

impl RelationType {
    /// Lista estática de todos los tipos de relación disponibles.
    pub const ALL: [Self; 10] = [
        Self::RelatedTo,
        Self::UsedIn,
        Self::CausedBy,
        Self::Solves,
        Self::Contradicts,
        Self::Supersedes,
        Self::DerivedFrom,
        Self::DependsOn,
        Self::Prefers,
        Self::Avoid,
    ];

    /// Representación canónica oficial en cadena (SRS §11).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RelatedTo => "RELATED_TO",
            Self::UsedIn => "USED_IN",
            Self::CausedBy => "CAUSED_BY",
            Self::Solves => "SOLVES",
            Self::Contradicts => "CONTRADICTS",
            Self::Supersedes => "SUPERSEDES",
            Self::DerivedFrom => "DERIVED_FROM",
            Self::DependsOn => "DEPENDS_ON",
            Self::Prefers => "PREFERS",
            Self::Avoid => "AVOID",
        }
    }

    /// Determina si la relación forma un grafo dirigido acíclico (DAG) y prohíbe ciclos.
    ///
    /// Por ejemplo, una decisión A no puede ser superseded_by B si B ya superseded_by A.
    /// Tampoco se permiten dependencias o derivaciones circulares.
    pub fn is_acyclic(&self) -> bool {
        matches!(
            self,
            Self::Supersedes | Self::DependsOn | Self::DerivedFrom | Self::CausedBy
        )
    }

    /// Determina si la relación es conceptualmente simétrica (bidireccional).
    pub fn is_symmetric(&self) -> bool {
        matches!(self, Self::RelatedTo | Self::Contradicts)
    }
}

impl fmt::Display for RelationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for RelationType {
    type Err = GraphError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.trim().to_uppercase().replace('-', "_");
        match normalized.as_str() {
            "RELATED_TO" | "RELATEDTO" => Ok(Self::RelatedTo),
            "USED_IN" | "USEDIN" => Ok(Self::UsedIn),
            "CAUSED_BY" | "CAUSEDBY" => Ok(Self::CausedBy),
            "SOLVES" => Ok(Self::Solves),
            "CONTRADICTS" => Ok(Self::Contradicts),
            "SUPERSEDES" | "SUPERSEDED_BY" => Ok(Self::Supersedes),
            "DERIVED_FROM" | "DERIVEDFROM" => Ok(Self::DerivedFrom),
            "DEPENDS_ON" | "DEPENDSON" => Ok(Self::DependsOn),
            "PREFERS" => Ok(Self::Prefers),
            "AVOID" => Ok(Self::Avoid),
            _ => Err(GraphError::InvalidRelation(s.to_string())),
        }
    }
}

/// Categoría o tipo de nodo en el grafo de conocimiento.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    /// Nodo respaldado por una unidad atómica de memoria cognitiva (`Memory`).
    Memory,
    /// Concepto abstracto, tema o patrón de diseño.
    Concept,
    /// Entidad del mundo real (tecnología, framework, proyecto, autor).
    Entity,
}

impl NodeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::Concept => "concept",
            Self::Entity => "entity",
        }
    }
}

impl fmt::Display for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for NodeType {
    type Err = GraphError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "memory" => Ok(Self::Memory),
            "concept" => Ok(Self::Concept),
            "entity" => Ok(Self::Entity),
            _ => Ok(Self::Concept),
        }
    }
}

/// Identificador fuertemente tipado de un nodo en el grafo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(Uuid);

impl NodeId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn from_memory_id(mem_id: &MemoryId) -> Self {
        Self(*mem_id.as_uuid())
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for NodeId {
    type Err = GraphError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s)
            .map(Self)
            .map_err(|_| GraphError::NodeNotFound(s.to_string()))
    }
}

/// Identificador fuertemente tipado de una arista en el grafo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EdgeId(Uuid);

impl EdgeId {
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

impl Default for EdgeId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EdgeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for EdgeId {
    type Err = GraphError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s)
            .map(Self)
            .map_err(|_| GraphError::EdgeNotFound(s.to_string()))
    }
}

/// Nodo en el grafo de conocimiento (SRS §11).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: NodeId,
    pub node_type: NodeType,
    pub label: String,
    pub memory_id: Option<MemoryId>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl GraphNode {
    /// Crea un nodo de concepto general.
    pub fn new_concept(label: impl Into<String>) -> Result<Self, GraphError> {
        let lbl = label.into().trim().to_string();
        if lbl.is_empty() {
            return Err(GraphError::EmptyLabel);
        }
        let now = Utc::now();
        Ok(Self {
            id: NodeId::new(),
            node_type: NodeType::Concept,
            label: lbl,
            memory_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
        })
    }

    /// Crea un nodo respaldado por un recuerdo (`Memory`). Su ID coincide con el del recuerdo.
    pub fn new_memory(memory_id: MemoryId, label: impl Into<String>) -> Result<Self, GraphError> {
        let lbl = label.into().trim().to_string();
        if lbl.is_empty() {
            return Err(GraphError::EmptyLabel);
        }
        let now = Utc::now();
        Ok(Self {
            id: NodeId::from_memory_id(&memory_id),
            node_type: NodeType::Memory,
            label: lbl,
            memory_id: Some(memory_id),
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
        })
    }

    /// Crea un nodo de entidad nombrada (tecnología, framework, proyecto).
    pub fn new_entity(label: impl Into<String>) -> Result<Self, GraphError> {
        let lbl = label.into().trim().to_string();
        if lbl.is_empty() {
            return Err(GraphError::EmptyLabel);
        }
        let now = Utc::now();
        Ok(Self {
            id: NodeId::new(),
            node_type: NodeType::Entity,
            label: lbl,
            memory_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
        })
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Arista tipada y ponderada que conecta dos nodos en el grafo (SRS §11).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: EdgeId,
    pub source_id: NodeId,
    pub target_id: NodeId,
    pub relation_type: RelationType,
    pub weight: f32,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl GraphEdge {
    /// Crea una nueva arista validando invariantes fundamentales:
    /// 1. Prohíbe auto-bucles (source_id != target_id).
    /// 2. Valida que el peso esté en [0.0, 1.0].
    pub fn new(
        source_id: NodeId,
        target_id: NodeId,
        relation_type: RelationType,
        weight: f32,
    ) -> Result<Self, GraphError> {
        if source_id == target_id {
            return Err(GraphError::SelfLoopForbidden {
                node_id: source_id.to_string(),
                relation: relation_type.to_string(),
            });
        }

        if !(0.0..=1.0).contains(&weight) {
            return Err(GraphError::InvalidWeight(weight));
        }

        let now = Utc::now();
        Ok(Self {
            id: EdgeId::new(),
            source_id,
            target_id,
            relation_type,
            weight,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
        })
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Dirección de exploración para traversal de grafos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TraversalDirection {
    /// Solo aristas salientes (source -> target).
    Outbound,
    /// Solo aristas entrantes (target <- source).
    Inbound,
    /// Ambas direcciones (grafo no dirigido).
    #[default]
    Both,
}

impl TraversalDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Outbound => "outbound",
            Self::Inbound => "inbound",
            Self::Both => "both",
        }
    }
}

impl fmt::Display for TraversalDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for TraversalDirection {
    type Err = GraphError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "outbound" | "out" => Ok(Self::Outbound),
            "inbound" | "in" => Ok(Self::Inbound),
            "both" | "all" => Ok(Self::Both),
            _ => Ok(Self::Both),
        }
    }
}

/// Opciones de configuración para consultas recursivas de traversal (SRS §11, §13.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphTraversalOptions {
    /// Profundidad máxima en saltos de arista (1..=5). Por defecto: 1.
    pub max_depth: u32,
    /// Dirección de las aristas a recorrer. Por defecto: `Both`.
    pub direction: TraversalDirection,
    /// Filtro opcional por tipos específicos de relación.
    pub relation_types: Option<Vec<RelationType>>,
    /// Umbral mínimo de peso de arista para filtrar conexiones débiles.
    pub min_weight: Option<f32>,
    /// Cantidad máxima de nodos a retornar. Por defecto: 50.
    pub limit: usize,
}

impl GraphTraversalOptions {
    pub fn new() -> Self {
        Self {
            max_depth: 1,
            direction: TraversalDirection::Both,
            relation_types: None,
            min_weight: None,
            limit: 50,
        }
    }

    pub fn with_depth(mut self, depth: u32) -> Result<Self, GraphError> {
        if depth == 0 || depth > 5 {
            return Err(GraphError::InvalidDepth {
                actual: depth,
                max: 5,
            });
        }
        self.max_depth = depth;
        Ok(self)
    }

    pub fn with_direction(mut self, direction: TraversalDirection) -> Self {
        self.direction = direction;
        self
    }

    pub fn with_relations(mut self, relations: Vec<RelationType>) -> Self {
        self.relation_types = Some(relations);
        self
    }

    pub fn with_min_weight(mut self, min_weight: f32) -> Result<Self, GraphError> {
        if !(0.0..=1.0).contains(&min_weight) {
            return Err(GraphError::InvalidWeight(min_weight));
        }
        self.min_weight = Some(min_weight);
        Ok(self)
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit.max(1);
        self
    }
}

impl Default for GraphTraversalOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Registro de un paso de traversal en un camino recursivo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphPathStep {
    pub node_id: NodeId,
    pub label: String,
    pub node_type: NodeType,
    pub edge_id: Option<EdgeId>,
    pub relation_type: Option<RelationType>,
    pub predecessor_id: Option<NodeId>,
    pub depth: u32,
    pub accumulated_weight: f32,
}

/// Subgrafo resultante de un traversal a partir de un nodo raíz.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphSubgraph {
    pub root: GraphNode,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub steps: Vec<GraphPathStep>,
}
