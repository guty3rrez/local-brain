//! Implementación pura en memoria de `GraphRepository` para pruebas unitarias de milisegundos (SRS RNF-006).

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use brain_domain::model::MemoryId;

use crate::errors::GraphError;
use crate::invariants::GraphInvariants;
use crate::model::{
    EdgeId, GraphEdge, GraphNode, GraphPathStep, GraphSubgraph, GraphTraversalOptions, NodeId,
    NodeType, RelationType, TraversalDirection,
};
use crate::ports::GraphRepository;

#[derive(Debug, Default)]
struct InMemoryStorage {
    nodes: HashMap<NodeId, GraphNode>,
    edges: HashMap<EdgeId, GraphEdge>,
}

/// Repositorio de grafo 100% en memoria, thread-safe y determinista.
#[derive(Debug, Default, Clone)]
pub struct InMemoryGraphRepository {
    storage: Arc<RwLock<InMemoryStorage>>,
}

impl InMemoryGraphRepository {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(InMemoryStorage::default())),
        }
    }

    pub fn node_count(&self) -> usize {
        let lock = self.storage.read().expect("Lock poisoned");
        lock.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        let lock = self.storage.read().expect("Lock poisoned");
        lock.edges.len()
    }
}

#[async_trait]
impl GraphRepository for InMemoryGraphRepository {
    async fn save_node(&self, node: &GraphNode) -> Result<(), GraphError> {
        let mut lock = self
            .storage
            .write()
            .map_err(|e| GraphError::StorageError(e.to_string()))?;
        lock.nodes.insert(node.id, node.clone());
        Ok(())
    }

    async fn find_node_by_id(&self, id: &NodeId) -> Result<Option<GraphNode>, GraphError> {
        let lock = self
            .storage
            .read()
            .map_err(|e| GraphError::StorageError(e.to_string()))?;
        Ok(lock.nodes.get(id).cloned())
    }

    async fn find_node_by_memory_id(
        &self,
        memory_id: &MemoryId,
    ) -> Result<Option<GraphNode>, GraphError> {
        let lock = self
            .storage
            .read()
            .map_err(|e| GraphError::StorageError(e.to_string()))?;
        let found = lock
            .nodes
            .values()
            .find(|n| n.memory_id.as_ref() == Some(memory_id))
            .cloned();
        Ok(found)
    }

    async fn find_node_by_label(
        &self,
        label: &str,
        node_type: NodeType,
    ) -> Result<Option<GraphNode>, GraphError> {
        let lock = self
            .storage
            .read()
            .map_err(|e| GraphError::StorageError(e.to_string()))?;
        let lower = label.trim().to_lowercase();
        let found = lock
            .nodes
            .values()
            .find(|n| n.node_type == node_type && n.label.trim().to_lowercase() == lower)
            .cloned();
        Ok(found)
    }

    async fn ensure_concept_node(&self, label: &str) -> Result<GraphNode, GraphError> {
        if let Some(existing) = self.find_node_by_label(label, NodeType::Concept).await? {
            return Ok(existing);
        }
        let node = GraphNode::new_concept(label)?;
        self.save_node(&node).await?;
        Ok(node)
    }

    async fn ensure_memory_node(
        &self,
        memory_id: &MemoryId,
        label: &str,
    ) -> Result<GraphNode, GraphError> {
        if let Some(existing) = self.find_node_by_memory_id(memory_id).await? {
            return Ok(existing);
        }
        let node = GraphNode::new_memory(*memory_id, label)?;
        self.save_node(&node).await?;
        Ok(node)
    }

    async fn save_edge(&self, edge: &GraphEdge) -> Result<(), GraphError> {
        GraphInvariants::validate_edge(edge)?;

        let mut lock = self
            .storage
            .write()
            .map_err(|e| GraphError::StorageError(e.to_string()))?;

        // Validar existencia de source y target
        if !lock.nodes.contains_key(&edge.source_id) {
            return Err(GraphError::NodeNotFound(edge.source_id.to_string()));
        }
        if !lock.nodes.contains_key(&edge.target_id) {
            return Err(GraphError::NodeNotFound(edge.target_id.to_string()));
        }

        // Validación de ciclos en relaciones acíclicas
        if edge.relation_type.is_acyclic() {
            let edges_ref = &lock.edges;
            let cycle = GraphInvariants::would_create_cycle(
                &edge.source_id,
                &edge.target_id,
                edge.relation_type,
                |curr, rel| {
                    edges_ref
                        .values()
                        .filter(|e| e.source_id == *curr && e.relation_type == rel)
                        .map(|e| e.target_id)
                        .collect()
                },
            )?;

            if cycle {
                return Err(GraphError::CycleDetected {
                    from: edge.source_id.to_string(),
                    to: edge.target_id.to_string(),
                    relation: edge.relation_type.to_string(),
                });
            }
        }

        // Reemplazar si ya existe la misma tripleta (source, target, relation)
        let existing_id = lock.edges.values().find_map(|e| {
            if e.source_id == edge.source_id
                && e.target_id == edge.target_id
                && e.relation_type == edge.relation_type
            {
                Some(e.id)
            } else {
                None
            }
        });

        if let Some(id) = existing_id {
            lock.edges.remove(&id);
        }

        lock.edges.insert(edge.id, edge.clone());
        Ok(())
    }

    async fn find_edge_by_id(&self, id: &EdgeId) -> Result<Option<GraphEdge>, GraphError> {
        let lock = self
            .storage
            .read()
            .map_err(|e| GraphError::StorageError(e.to_string()))?;
        Ok(lock.edges.get(id).cloned())
    }

    async fn find_edge(
        &self,
        source_id: &NodeId,
        target_id: &NodeId,
        relation: RelationType,
    ) -> Result<Option<GraphEdge>, GraphError> {
        let lock = self
            .storage
            .read()
            .map_err(|e| GraphError::StorageError(e.to_string()))?;
        let found = lock
            .edges
            .values()
            .find(|e| {
                e.source_id == *source_id
                    && e.target_id == *target_id
                    && e.relation_type == relation
            })
            .cloned();
        Ok(found)
    }

    async fn find_edges_for_node(
        &self,
        node_id: &NodeId,
        direction: TraversalDirection,
        relation: Option<RelationType>,
    ) -> Result<Vec<GraphEdge>, GraphError> {
        let lock = self
            .storage
            .read()
            .map_err(|e| GraphError::StorageError(e.to_string()))?;

        let filtered = lock
            .edges
            .values()
            .filter(|e| {
                let dir_match = match direction {
                    TraversalDirection::Outbound => e.source_id == *node_id,
                    TraversalDirection::Inbound => e.target_id == *node_id,
                    TraversalDirection::Both => e.source_id == *node_id || e.target_id == *node_id,
                };
                let rel_match = match relation {
                    Some(r) => e.relation_type == r,
                    None => true,
                };
                dir_match && rel_match
            })
            .cloned()
            .collect();

        Ok(filtered)
    }

    async fn delete_edge(&self, id: &EdgeId) -> Result<(), GraphError> {
        let mut lock = self
            .storage
            .write()
            .map_err(|e| GraphError::StorageError(e.to_string()))?;
        lock.edges.remove(id);
        Ok(())
    }

    async fn has_path(
        &self,
        from: &NodeId,
        to: &NodeId,
        relation: Option<RelationType>,
    ) -> Result<bool, GraphError> {
        let lock = self
            .storage
            .read()
            .map_err(|e| GraphError::StorageError(e.to_string()))?;

        if from == to {
            return Ok(true);
        }

        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();

        queue.push_back(*from);
        visited.insert(*from);

        while let Some(curr) = queue.pop_front() {
            let neighbors: Vec<NodeId> = lock
                .edges
                .values()
                .filter(|e| e.source_id == curr && relation.is_none_or(|r| e.relation_type == r))
                .map(|e| e.target_id)
                .collect();

            for next in neighbors {
                if next == *to {
                    return Ok(true);
                }
                if visited.insert(next) {
                    queue.push_back(next);
                }
            }
        }

        Ok(false)
    }

    async fn traverse(
        &self,
        root_id: &NodeId,
        options: &GraphTraversalOptions,
    ) -> Result<GraphSubgraph, GraphError> {
        let lock = self
            .storage
            .read()
            .map_err(|e| GraphError::StorageError(e.to_string()))?;

        let root_node = lock
            .nodes
            .get(root_id)
            .cloned()
            .ok_or_else(|| GraphError::NodeNotFound(root_id.to_string()))?;

        let mut discovered_nodes: HashMap<NodeId, GraphNode> = HashMap::new();
        let mut discovered_edges: HashMap<EdgeId, GraphEdge> = HashMap::new();
        let mut path_steps: Vec<GraphPathStep> = Vec::new();

        discovered_nodes.insert(*root_id, root_node.clone());
        path_steps.push(GraphPathStep {
            node_id: *root_id,
            label: root_node.label.clone(),
            node_type: root_node.node_type,
            edge_id: None,
            relation_type: None,
            predecessor_id: None,
            depth: 0,
            accumulated_weight: 1.0,
        });

        // Cola de BFS: (current_node_id, current_depth, accumulated_weight, path_history)
        let mut queue: VecDeque<(NodeId, u32, f32, Vec<NodeId>)> = VecDeque::new();
        queue.push_back((*root_id, 0, 1.0, vec![*root_id]));

        while let Some((curr_id, curr_depth, acc_weight, path)) = queue.pop_front() {
            if curr_depth >= options.max_depth {
                continue;
            }

            for edge in lock.edges.values() {
                // Filtrar por umbral de peso si aplica
                if let Some(min_w) = options.min_weight {
                    if edge.weight < min_w {
                        continue;
                    }
                }

                // Filtrar por tipos de relación si aplica
                if let Some(ref allowed_rels) = options.relation_types {
                    if !allowed_rels.contains(&edge.relation_type) {
                        continue;
                    }
                }

                let maybe_next = match options.direction {
                    TraversalDirection::Outbound if edge.source_id == curr_id => {
                        Some(edge.target_id)
                    }
                    TraversalDirection::Inbound if edge.target_id == curr_id => {
                        Some(edge.source_id)
                    }
                    TraversalDirection::Both => {
                        if edge.source_id == curr_id {
                            Some(edge.target_id)
                        } else if edge.target_id == curr_id {
                            Some(edge.source_id)
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                if let Some(next_id) = maybe_next {
                    // Evitar ciclos infinitos en el camino actual
                    if path.contains(&next_id) {
                        continue;
                    }

                    if let Some(next_node) = lock.nodes.get(&next_id) {
                        let new_acc_weight = acc_weight * edge.weight;
                        let next_depth = curr_depth + 1;

                        discovered_nodes.insert(next_id, next_node.clone());
                        discovered_edges.insert(edge.id, edge.clone());

                        path_steps.push(GraphPathStep {
                            node_id: next_id,
                            label: next_node.label.clone(),
                            node_type: next_node.node_type,
                            edge_id: Some(edge.id),
                            relation_type: Some(edge.relation_type),
                            predecessor_id: Some(curr_id),
                            depth: next_depth,
                            accumulated_weight: new_acc_weight,
                        });

                        let mut next_path = path.clone();
                        next_path.push(next_id);
                        queue.push_back((next_id, next_depth, new_acc_weight, next_path));

                        if discovered_nodes.len() >= options.limit {
                            break;
                        }
                    }
                }
            }

            if discovered_nodes.len() >= options.limit {
                break;
            }
        }

        Ok(GraphSubgraph {
            root: root_node,
            nodes: discovered_nodes.into_values().collect(),
            edges: discovered_edges.into_values().collect(),
            steps: path_steps,
        })
    }
}
