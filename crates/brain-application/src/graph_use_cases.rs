//! Casos de uso para operaciones en el Grafo de Conocimiento (SRS §11, §13.2, F5-03).

use std::sync::Arc;

use tracing::{info, instrument};
use uuid::Uuid;

use brain_domain::model::MemoryId;
use brain_domain::ports::MemoryRepository;
use brain_graph::errors::GraphError;
use brain_graph::model::{
    GraphEdge, GraphNode, GraphSubgraph, GraphTraversalOptions, NodeId, NodeType, RelationType,
    TraversalDirection,
};
use brain_graph::ports::GraphRepository;

use crate::ApplicationError;

/// Comando para conectar dos nodos o conceptos en el grafo de conocimiento (SRS §11).
#[derive(Debug, Clone)]
pub struct RelateCommand {
    /// Identificador UUID de memoria o nombre de concepto origen.
    pub source: String,
    /// Identificador UUID de memoria o nombre de concepto destino.
    pub target: String,
    /// Tipo de relación semántica tipada.
    pub relation: RelationType,
    /// Peso o fuerza de la relación en [0.0, 1.0] (por defecto: 1.0).
    pub weight: Option<f32>,
    /// Contexto situacional o justificación explicativa de la relación.
    pub context: Option<String>,
}

impl RelateCommand {
    pub fn new(
        source: impl Into<String>,
        target: impl Into<String>,
        relation: RelationType,
    ) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            relation,
            weight: None,
            context: None,
        }
    }

    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = Some(weight);
        self
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
}

/// Caso de Uso: Conectar recuerdos o conceptos en el Grafo de Conocimiento (SRS §11, §21.1, F5-03).
#[derive(Clone)]
pub struct RelateUseCase {
    graph_repository: Arc<dyn GraphRepository>,
    memory_repository: Option<Arc<dyn MemoryRepository>>,
}

impl RelateUseCase {
    pub fn new(graph_repository: Arc<dyn GraphRepository>) -> Self {
        Self {
            graph_repository,
            memory_repository: None,
        }
    }

    pub fn with_memory_repository(mut self, memory_repository: Arc<dyn MemoryRepository>) -> Self {
        self.memory_repository = Some(memory_repository);
        self
    }

    /// Resuelve inteligentemente un identificador a un nodo de grafo:
    /// - Si es un UUID y existe en memoria: asegura un nodo de tipo `Memory`.
    /// - Si es una cadena textual libre: asegura un nodo de tipo `Concept`.
    async fn resolve_or_ensure_node(
        &self,
        identifier: &str,
    ) -> Result<GraphNode, ApplicationError> {
        let clean = identifier.trim();
        if clean.is_empty() {
            return Err(GraphError::EmptyLabel.into());
        }

        // Si es un UUID válido
        if let Ok(uuid) = Uuid::parse_str(clean) {
            let mem_id = MemoryId::from_uuid(uuid);

            // 1. Verificar si ya existe en el grafo
            if let Some(existing) = self
                .graph_repository
                .find_node_by_memory_id(&mem_id)
                .await?
            {
                return Ok(existing);
            }

            // 2. Si hay repositorio de memoria, consultar la memoria para obtener el resumen/título
            if let Some(ref mem_repo) = self.memory_repository {
                if let Some(mem) = mem_repo.find_by_id(&mem_id).await? {
                    let label = mem.summary.clone().unwrap_or_else(|| {
                        let text = mem.content.text();
                        if text.len() > 60 {
                            format!("{}...", &text[..57])
                        } else {
                            text.to_string()
                        }
                    });
                    return Ok(self
                        .graph_repository
                        .ensure_memory_node(&mem_id, &label)
                        .await?);
                }
            }

            // Si es UUID pero no está en el repo de memorias o no hay repo inyectado
            let node_id = NodeId::from_uuid(uuid);
            if let Some(node) = self.graph_repository.find_node_by_id(&node_id).await? {
                return Ok(node);
            }

            // Tratar como memoria no registrada con label UUID
            return Ok(self
                .graph_repository
                .ensure_memory_node(&mem_id, clean)
                .await?);
        }

        // Si no es UUID, es un concepto textual
        Ok(self.graph_repository.ensure_concept_node(clean).await?)
    }

    #[instrument(skip(self, cmd), fields(source = %cmd.source, target = %cmd.target, relation = %cmd.relation))]
    pub async fn execute(&self, cmd: RelateCommand) -> Result<GraphEdge, ApplicationError> {
        let source_node = self.resolve_or_ensure_node(&cmd.source).await?;
        let target_node = self.resolve_or_ensure_node(&cmd.target).await?;

        let weight = cmd.weight.unwrap_or(1.0);
        let mut edge = GraphEdge::new(source_node.id, target_node.id, cmd.relation, weight)?;

        if let Some(ctx) = cmd.context {
            edge = edge.with_metadata(serde_json::json!({ "context": ctx }));
        }

        self.graph_repository.save_edge(&edge).await?;

        info!(
            edge_id = %edge.id,
            source = %source_node.label,
            target = %target_node.label,
            relation = %cmd.relation,
            "Relación semántica registrada en el grafo de conocimiento"
        );

        Ok(edge)
    }
}

/// Parámetros de consulta para traversal recursivo en el Grafo de Conocimiento (SRS §11, §13.2).
#[derive(Debug, Clone)]
pub struct TraverseGraphQuery {
    /// Identificador UUID de nodo o nombre de concepto a explorar.
    pub node: String,
    /// Profundidad máxima de traversal (1..=5).
    pub depth: Option<u32>,
    /// Dirección de las aristas a seguir (`Outbound`, `Inbound`, `Both`).
    pub direction: Option<TraversalDirection>,
    /// Filtro por tipos de relación específicos.
    pub relation_types: Option<Vec<RelationType>>,
    /// Umbral mínimo de peso de arista.
    pub min_weight: Option<f32>,
    /// Límite máximo de nodos a retornar.
    pub limit: Option<usize>,
}

impl TraverseGraphQuery {
    pub fn new(node: impl Into<String>) -> Self {
        Self {
            node: node.into(),
            depth: None,
            direction: None,
            relation_types: None,
            min_weight: None,
            limit: None,
        }
    }

    pub fn with_depth(mut self, depth: u32) -> Self {
        self.depth = Some(depth);
        self
    }

    pub fn with_direction(mut self, direction: TraversalDirection) -> Self {
        self.direction = Some(direction);
        self
    }

    pub fn with_relations(mut self, relations: Vec<RelationType>) -> Self {
        self.relation_types = Some(relations);
        self
    }
}

/// Caso de Uso: Navegar y consultar vecindades en el Grafo de Conocimiento (SRS §11, §13.2).
#[derive(Clone)]
pub struct TraverseGraphUseCase {
    graph_repository: Arc<dyn GraphRepository>,
}

impl TraverseGraphUseCase {
    pub fn new(graph_repository: Arc<dyn GraphRepository>) -> Self {
        Self { graph_repository }
    }

    #[instrument(skip(self, query), fields(node = %query.node))]
    pub async fn execute(
        &self,
        query: TraverseGraphQuery,
    ) -> Result<GraphSubgraph, ApplicationError> {
        let clean = query.node.trim();
        if clean.is_empty() {
            return Err(GraphError::EmptyLabel.into());
        }

        // 1. Intentar resolver el nodo raíz por UUID
        let root_node = if let Ok(uuid) = Uuid::parse_str(clean) {
            let node_id = NodeId::from_uuid(uuid);
            if let Some(n) = self.graph_repository.find_node_by_id(&node_id).await? {
                Some(n)
            } else {
                let mem_id = MemoryId::from_uuid(uuid);
                self.graph_repository
                    .find_node_by_memory_id(&mem_id)
                    .await?
            }
        } else {
            None
        };

        // 2. Si no es UUID o no se encontró por ID, buscar por etiqueta de concepto o entidad
        let root_node = match root_node {
            Some(n) => n,
            None => {
                if let Some(c) = self
                    .graph_repository
                    .find_node_by_label(clean, NodeType::Concept)
                    .await?
                {
                    c
                } else if let Some(e) = self
                    .graph_repository
                    .find_node_by_label(clean, NodeType::Entity)
                    .await?
                {
                    e
                } else if let Some(m) = self
                    .graph_repository
                    .find_node_by_label(clean, NodeType::Memory)
                    .await?
                {
                    m
                } else {
                    return Err(GraphError::NodeNotFound(clean.to_string()).into());
                }
            }
        };

        // 3. Configurar opciones de traversal
        let mut opts = GraphTraversalOptions::new();
        if let Some(d) = query.depth {
            opts = opts.with_depth(d)?;
        }
        if let Some(dir) = query.direction {
            opts = opts.with_direction(dir);
        }
        if let Some(rels) = query.relation_types {
            opts = opts.with_relations(rels);
        }
        if let Some(w) = query.min_weight {
            opts = opts.with_min_weight(w)?;
        }
        if let Some(l) = query.limit {
            opts = opts.with_limit(l);
        }

        // 4. Ejecutar consulta de traversal
        let subgraph = self.graph_repository.traverse(&root_node.id, &opts).await?;

        info!(
            root = %subgraph.root.label,
            total_nodes = subgraph.nodes.len(),
            total_edges = subgraph.edges.len(),
            "Traversal de grafo completado exitosamente"
        );

        Ok(subgraph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brain_domain::model::{Memory, MemoryContent, MemoryOrigin, MemoryType, Provenance};
    use brain_domain::ports::InMemoryMemoryRepository;
    use brain_graph::InMemoryGraphRepository;

    #[tokio::test]
    async fn test_relate_two_concepts() {
        let graph_repo = Arc::new(InMemoryGraphRepository::new());
        let use_case = RelateUseCase::new(graph_repo.clone());

        let cmd = RelateCommand::new("Rust", "PostgreSQL", RelationType::UsedIn)
            .with_weight(0.95)
            .with_context("Backend persistente");

        let edge = use_case.execute(cmd).await.unwrap();
        assert_eq!(edge.relation_type, RelationType::UsedIn);
        assert_eq!(edge.weight, 0.95);

        assert_eq!(graph_repo.node_count(), 2);
        assert_eq!(graph_repo.edge_count(), 1);
    }

    #[tokio::test]
    async fn test_relate_memory_to_concept() {
        let graph_repo = Arc::new(InMemoryGraphRepository::new());
        let mem_repo = Arc::new(InMemoryMemoryRepository::new());

        let content = MemoryContent::new("Decisión de arquitectura: Hexagonal").unwrap();
        let prov = Provenance::new(MemoryOrigin::UserPrompt);
        let memory = Memory::new(content, MemoryType::Episodic, prov);
        mem_repo.save(&memory).await.unwrap();

        let use_case = RelateUseCase::new(graph_repo.clone()).with_memory_repository(mem_repo);

        let cmd = RelateCommand::new(
            memory.id.to_string(),
            "Hexagonal Architecture",
            RelationType::UsedIn,
        );
        let edge = use_case.execute(cmd).await.unwrap();

        assert_eq!(edge.relation_type, RelationType::UsedIn);

        let mem_node = graph_repo.find_node_by_memory_id(&memory.id).await.unwrap();
        assert!(mem_node.is_some());
        assert_eq!(mem_node.unwrap().node_type, NodeType::Memory);
    }

    #[tokio::test]
    async fn test_relate_supersedes_cycle_prevention() {
        let graph_repo = Arc::new(InMemoryGraphRepository::new());
        let use_case = RelateUseCase::new(graph_repo);

        // B supersedes A
        let cmd1 = RelateCommand::new("Decisión B", "Decisión A", RelationType::Supersedes);
        use_case.execute(cmd1).await.unwrap();

        // A supersedes B (debe fallar por ciclo)
        let cmd2 = RelateCommand::new("Decisión A", "Decisión B", RelationType::Supersedes);
        let err = use_case.execute(cmd2).await.unwrap_err();

        match err {
            ApplicationError::Graph(GraphError::CycleDetected { .. }) => (),
            other => panic!("Se esperaba CycleDetected, recibido: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_traverse_graph_use_case() {
        let graph_repo = Arc::new(InMemoryGraphRepository::new());
        let relate_use_case = RelateUseCase::new(graph_repo.clone());
        let traverse_use_case = TraverseGraphUseCase::new(graph_repo);

        relate_use_case
            .execute(RelateCommand::new("Rust", "Actix", RelationType::UsedIn))
            .await
            .unwrap();
        relate_use_case
            .execute(RelateCommand::new(
                "Actix",
                "Tokio",
                RelationType::DependsOn,
            ))
            .await
            .unwrap();

        let query = TraverseGraphQuery::new("Rust").with_depth(2);
        let subgraph = traverse_use_case.execute(query).await.unwrap();

        assert_eq!(subgraph.root.label, "Rust");
        assert_eq!(subgraph.nodes.len(), 3);
        assert_eq!(subgraph.edges.len(), 2);
    }
}
