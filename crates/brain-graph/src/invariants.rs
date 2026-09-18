//! Invariantes de dominio y reglas de integridad para el grafo de conocimiento (SRS §11).

use std::collections::{HashSet, VecDeque};

use crate::errors::GraphError;
use crate::model::{GraphEdge, NodeId, RelationType};

/// Validador de invariantes estructurales y reglas de consistencia de grafos.
pub struct GraphInvariants;

impl GraphInvariants {
    /// Valida que la arista propuesta no viole invariantes locales:
    /// 1. No auto-bucles (source != target).
    /// 2. Peso en rango [0.0, 1.0].
    pub fn validate_edge(edge: &GraphEdge) -> Result<(), GraphError> {
        if edge.source_id == edge.target_id {
            return Err(GraphError::SelfLoopForbidden {
                node_id: edge.source_id.to_string(),
                relation: edge.relation_type.to_string(),
            });
        }
        if !(0.0..=1.0).contains(&edge.weight) {
            return Err(GraphError::InvalidWeight(edge.weight));
        }
        Ok(())
    }

    /// Comprueba deterministamente si agregar una arista dirigida `source -> target`
    /// crearía un ciclo en una relación que exige aciclicidad (DAG).
    ///
    /// Utiliza búsqueda en amplitud (BFS) sobre las aristas existentes:
    /// Si existe un camino desde `target` hasta `source` con la relación dada,
    /// agregar `source -> target` cerraría un ciclo prohibido.
    pub fn would_create_cycle<F>(
        source: &NodeId,
        target: &NodeId,
        relation: RelationType,
        get_outbound_neighbors: F,
    ) -> Result<bool, GraphError>
    where
        F: Fn(&NodeId, RelationType) -> Vec<NodeId>,
    {
        if !relation.is_acyclic() {
            // Relaciones no acíclicas (como RELATED_TO o CONTRADICTS) permiten ciclos
            return Ok(false);
        }

        // Si source == target es un auto-bucle, ciclo inmediato
        if source == target {
            return Ok(true);
        }

        // BFS desde `target` buscando alcanzar `source`
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();

        queue.push_back(*target);
        visited.insert(*target);

        while let Some(current) = queue.pop_front() {
            let neighbors = get_outbound_neighbors(&current, relation);
            for neighbor in neighbors {
                if neighbor == *source {
                    // Se encontró un camino de regreso a source: agregarlo formaría un ciclo
                    return Ok(true);
                }
                if visited.insert(neighbor) {
                    queue.push_back(neighbor);
                }
            }
        }

        Ok(false)
    }
}
