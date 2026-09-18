//! Puertos secundarios de arquitectura hexagonal para persistencia y navegación del grafo (SRS §8, §11, §38).

use async_trait::async_trait;
use brain_domain::model::MemoryId;

use crate::errors::GraphError;
use crate::model::{
    EdgeId, GraphEdge, GraphNode, GraphSubgraph, GraphTraversalOptions, NodeId, NodeType,
    RelationType, TraversalDirection,
};

/// Puerto secundario para persistencia y consultas sobre el grafo de conocimiento (SRS §11, §38).
#[async_trait]
pub trait GraphRepository: Send + Sync {
    /// Guarda o actualiza un nodo en el grafo.
    async fn save_node(&self, node: &GraphNode) -> Result<(), GraphError>;

    /// Busca un nodo por su identificador único.
    async fn find_node_by_id(&self, id: &NodeId) -> Result<Option<GraphNode>, GraphError>;

    /// Busca un nodo vinculado a una memoria específica por su `MemoryId`.
    async fn find_node_by_memory_id(
        &self,
        memory_id: &MemoryId,
    ) -> Result<Option<GraphNode>, GraphError>;

    /// Busca un nodo por su etiqueta y categoría (case-insensitive).
    async fn find_node_by_label(
        &self,
        label: &str,
        node_type: NodeType,
    ) -> Result<Option<GraphNode>, GraphError>;

    /// Garantiza la existencia de un nodo de concepto: si existe lo retorna; si no, lo crea.
    async fn ensure_concept_node(&self, label: &str) -> Result<GraphNode, GraphError>;

    /// Garantiza la existencia de un nodo para una memoria: si existe lo retorna; si no, lo crea.
    async fn ensure_memory_node(
        &self,
        memory_id: &MemoryId,
        label: &str,
    ) -> Result<GraphNode, GraphError>;

    /// Guarda o actualiza una arista en el grafo.
    async fn save_edge(&self, edge: &GraphEdge) -> Result<(), GraphError>;

    /// Busca una arista por su identificador único.
    async fn find_edge_by_id(&self, id: &EdgeId) -> Result<Option<GraphEdge>, GraphError>;

    /// Busca una arista específica entre dos nodos por tipo de relación.
    async fn find_edge(
        &self,
        source_id: &NodeId,
        target_id: &NodeId,
        relation: RelationType,
    ) -> Result<Option<GraphEdge>, GraphError>;

    /// Obtiene las aristas conectadas a un nodo según la dirección y filtro de relación opcional.
    async fn find_edges_for_node(
        &self,
        node_id: &NodeId,
        direction: TraversalDirection,
        relation: Option<RelationType>,
    ) -> Result<Vec<GraphEdge>, GraphError>;

    /// Elimina una arista del grafo por su ID.
    async fn delete_edge(&self, id: &EdgeId) -> Result<(), GraphError>;

    /// Verifica si existe un camino dirigido desde `from` hasta `to` opcionalmente filtrado por tipo de relación.
    async fn has_path(
        &self,
        from: &NodeId,
        to: &NodeId,
        relation: Option<RelationType>,
    ) -> Result<bool, GraphError>;

    /// Ejecuta una consulta de traversal recursivo a partir de un nodo raíz.
    async fn traverse(
        &self,
        root_id: &NodeId,
        options: &GraphTraversalOptions,
    ) -> Result<GraphSubgraph, GraphError>;
}
