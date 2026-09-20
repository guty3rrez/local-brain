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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;

    use super::*;
    use crate::model::EdgeId;

    fn raw_edge(source_id: NodeId, target_id: NodeId, weight: f32) -> GraphEdge {
        // Construido con un literal de struct (no `GraphEdge::new`) a propósito:
        // `GraphEdge::new` ya rechaza auto-bucles y pesos inválidos por su cuenta,
        // así que un edge inválido nunca llegaría aquí en producción. Estos tests
        // verifican el invariante de `GraphInvariants::validate_edge` en aislamiento,
        // como defensa en profundidad independiente del constructor.
        let now = Utc::now();
        GraphEdge {
            id: EdgeId::new(),
            source_id,
            target_id,
            relation_type: RelationType::RelatedTo,
            weight,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn test_validate_edge_accepts_valid_edge() {
        let edge = raw_edge(NodeId::new(), NodeId::new(), 0.5);
        assert!(GraphInvariants::validate_edge(&edge).is_ok());
    }

    #[test]
    fn test_validate_edge_rejects_self_loop() {
        let node_id = NodeId::new();
        let edge = raw_edge(node_id, node_id, 0.5);
        match GraphInvariants::validate_edge(&edge) {
            Err(GraphError::SelfLoopForbidden { node_id: id, .. }) => {
                assert_eq!(id, node_id.to_string());
            }
            other => panic!("Se esperaba SelfLoopForbidden, recibido: {:?}", other),
        }
    }

    #[test]
    fn test_validate_edge_rejects_weight_out_of_range() {
        let below = raw_edge(NodeId::new(), NodeId::new(), -0.1);
        assert!(matches!(
            GraphInvariants::validate_edge(&below),
            Err(GraphError::InvalidWeight(_))
        ));

        let above = raw_edge(NodeId::new(), NodeId::new(), 1.1);
        assert!(matches!(
            GraphInvariants::validate_edge(&above),
            Err(GraphError::InvalidWeight(_))
        ));
    }

    #[test]
    fn test_would_create_cycle_false_for_non_acyclic_relation() {
        let source = NodeId::new();
        let target = NodeId::new();
        // RELATED_TO no es acíclica: nunca reporta ciclo, sin importar la topología.
        let result = GraphInvariants::would_create_cycle(
            &source,
            &target,
            RelationType::RelatedTo,
            |_, _| vec![source],
        );
        assert!(!result.unwrap());
    }

    #[test]
    fn test_would_create_cycle_true_for_self_reference() {
        let node = NodeId::new();
        let result = GraphInvariants::would_create_cycle(
            &node,
            &node,
            RelationType::DependsOn,
            |_, _| vec![],
        );
        assert!(result.unwrap());
    }

    #[test]
    fn test_would_create_cycle_detects_transitive_cycle() {
        // a -> b -> c ya existe. Proponer c -> a cerraría el ciclo.
        let a = NodeId::new();
        let b = NodeId::new();
        let c = NodeId::new();

        let mut edges: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        edges.insert(a, vec![b]);
        edges.insert(b, vec![c]);

        let result =
            GraphInvariants::would_create_cycle(&c, &a, RelationType::DependsOn, |node, _| {
                edges.get(node).cloned().unwrap_or_default()
            });
        assert!(result.unwrap());
    }

    #[test]
    fn test_would_create_cycle_false_for_linear_graph() {
        // a -> b -> c ya existe. Proponer a -> c NO cierra ningún ciclo (sigue siendo un DAG).
        let a = NodeId::new();
        let b = NodeId::new();
        let c = NodeId::new();

        let mut edges: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        edges.insert(a, vec![b]);
        edges.insert(b, vec![c]);

        let result =
            GraphInvariants::would_create_cycle(&a, &c, RelationType::DependsOn, |node, _| {
                edges.get(node).cloned().unwrap_or_default()
            });
        assert!(!result.unwrap());
    }
}
