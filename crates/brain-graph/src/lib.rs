//! # Brain Graph
//!
//! Crate de Dominio y Teoría de Grafos de Conocimiento de Local Brain (SRS §11, §68, Fase 5).
//!
//! Este módulo contiene los tipos fuertemente tipados para los 10 tipos de relación,
//! detección determinista de ciclos en relaciones acíclicas (DAG), prevención de auto-bucles,
//! y el puerto secundario `GraphRepository` con su implementación en memoria para tests instantáneos.

pub mod errors;
pub mod in_memory;
pub mod invariants;
pub mod model;
pub mod ports;

pub use errors::GraphError;
pub use in_memory::InMemoryGraphRepository;
pub use invariants::GraphInvariants;
pub use model::{
    EdgeId, GraphEdge, GraphNode, GraphPathStep, GraphSubgraph, GraphTraversalOptions, NodeId,
    NodeType, RelationType, TraversalDirection,
};
pub use ports::GraphRepository;

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use brain_domain::model::MemoryId;

    use super::*;

    #[test]
    fn test_relation_type_parse_and_properties() {
        for rel in RelationType::ALL {
            let s = rel.as_str();
            let parsed = RelationType::from_str(s).expect("Debe parsear tipo canónico");
            assert_eq!(rel, parsed);

            // Prueba case-insensitivity
            let lower = s.to_lowercase();
            let parsed_lower =
                RelationType::from_str(&lower).expect("Debe parsear tipo en minúsculas");
            assert_eq!(rel, parsed_lower);
        }

        assert!(RelationType::Supersedes.is_acyclic());
        assert!(RelationType::DependsOn.is_acyclic());
        assert!(RelationType::DerivedFrom.is_acyclic());
        assert!(RelationType::CausedBy.is_acyclic());

        assert!(!RelationType::RelatedTo.is_acyclic());
        assert!(!RelationType::Solves.is_acyclic());
        assert!(!RelationType::Contradicts.is_acyclic());

        assert!(RelationType::RelatedTo.is_symmetric());
        assert!(RelationType::Contradicts.is_symmetric());
        assert!(!RelationType::Supersedes.is_symmetric());
    }

    #[test]
    fn test_self_loop_is_forbidden() {
        let node_id = NodeId::new();
        let err = GraphEdge::new(node_id, node_id, RelationType::RelatedTo, 1.0).unwrap_err();
        match err {
            GraphError::SelfLoopForbidden { node_id: id, .. } => {
                assert_eq!(id, node_id.to_string());
            }
            other => panic!("Se esperaba SelfLoopForbidden, recibido: {:?}", other),
        }
    }

    #[test]
    fn test_invalid_weight_is_rejected() {
        let n1 = NodeId::new();
        let n2 = NodeId::new();

        assert!(matches!(
            GraphEdge::new(n1, n2, RelationType::UsedIn, -0.1).unwrap_err(),
            GraphError::InvalidWeight(_)
        ));

        assert!(matches!(
            GraphEdge::new(n1, n2, RelationType::UsedIn, 1.01).unwrap_err(),
            GraphError::InvalidWeight(_)
        ));

        assert!(GraphEdge::new(n1, n2, RelationType::UsedIn, 0.0).is_ok());
        assert!(GraphEdge::new(n1, n2, RelationType::UsedIn, 1.0).is_ok());
        assert!(GraphEdge::new(n1, n2, RelationType::UsedIn, 0.75).is_ok());
    }

    #[tokio::test]
    async fn test_dag_cycle_prevention_direct_two_nodes() {
        let repo = InMemoryGraphRepository::new();

        let n1 = repo.ensure_concept_node("Decision A").await.unwrap();
        let n2 = repo.ensure_concept_node("Decision B").await.unwrap();

        // Decision B supersedes Decision A
        let edge1 =
            GraphEdge::new(n2.id, n1.id, RelationType::Supersedes, 1.0).expect("Arista válida");
        repo.save_edge(&edge1).await.expect("Debe guardar arista");

        // Intentar Decision A supersedes Decision B debe fallar por ciclo
        let edge2 =
            GraphEdge::new(n1.id, n2.id, RelationType::Supersedes, 1.0).expect("Arista válida");
        let result = repo.save_edge(&edge2).await;

        assert!(matches!(result, Err(GraphError::CycleDetected { .. })));
    }

    #[tokio::test]
    async fn test_dag_cycle_prevention_transitive_three_nodes() {
        let repo = InMemoryGraphRepository::new();

        let n1 = repo.ensure_concept_node("A").await.unwrap();
        let n2 = repo.ensure_concept_node("B").await.unwrap();
        let n3 = repo.ensure_concept_node("C").await.unwrap();

        // A DEPENDS_ON B
        let e1 = GraphEdge::new(n1.id, n2.id, RelationType::DependsOn, 1.0).unwrap();
        repo.save_edge(&e1).await.unwrap();

        // B DEPENDS_ON C
        let e2 = GraphEdge::new(n2.id, n3.id, RelationType::DependsOn, 1.0).unwrap();
        repo.save_edge(&e2).await.unwrap();

        // Intentar C DEPENDS_ON A formaría ciclo A -> B -> C -> A
        let e3 = GraphEdge::new(n3.id, n1.id, RelationType::DependsOn, 1.0).unwrap();
        let err = repo.save_edge(&e3).await.unwrap_err();

        assert!(matches!(err, GraphError::CycleDetected { .. }));
    }

    #[tokio::test]
    async fn test_cyclic_relations_allowed_for_non_dag() {
        let repo = InMemoryGraphRepository::new();

        let n1 = repo.ensure_concept_node("Concept 1").await.unwrap();
        let n2 = repo.ensure_concept_node("Concept 2").await.unwrap();

        // RELATED_TO no es acíclica, por lo que A -> B y B -> A son válidas
        let e1 = GraphEdge::new(n1.id, n2.id, RelationType::RelatedTo, 0.8).unwrap();
        repo.save_edge(&e1).await.unwrap();

        let e2 = GraphEdge::new(n2.id, n1.id, RelationType::RelatedTo, 0.8).unwrap();
        assert!(repo.save_edge(&e2).await.is_ok());
    }

    #[tokio::test]
    async fn test_memory_node_id_matches_memory_id() {
        let repo = InMemoryGraphRepository::new();
        let mem_id = MemoryId::new();

        let node = repo
            .ensure_memory_node(&mem_id, "Arquitectura Hexagonal")
            .await
            .unwrap();

        assert_eq!(*node.id.as_uuid(), *mem_id.as_uuid());
        assert_eq!(node.memory_id, Some(mem_id));
        assert_eq!(node.node_type, NodeType::Memory);

        // Llamar de nuevo debe retornar el mismo nodo sin duplicar
        let node_dup = repo
            .ensure_memory_node(&mem_id, "Otro nombre")
            .await
            .unwrap();
        assert_eq!(node.id, node_dup.id);
        assert_eq!(repo.node_count(), 1);
    }

    #[tokio::test]
    async fn test_in_memory_traversal_multi_hop() {
        let repo = InMemoryGraphRepository::new();

        let rust = repo.ensure_concept_node("Rust").await.unwrap();
        let actix = repo.ensure_concept_node("Actix").await.unwrap();
        let tokio = repo.ensure_concept_node("Tokio").await.unwrap();

        // Rust --[USED_IN]--> Actix
        let e1 = GraphEdge::new(rust.id, actix.id, RelationType::UsedIn, 0.9).unwrap();
        repo.save_edge(&e1).await.unwrap();

        // Actix --[DEPENDS_ON]--> Tokio
        let e2 = GraphEdge::new(actix.id, tokio.id, RelationType::DependsOn, 1.0).unwrap();
        repo.save_edge(&e2).await.unwrap();

        // Traversal a profundidad 1 desde Rust
        let opts_d1 = GraphTraversalOptions::new()
            .with_depth(1)
            .unwrap()
            .with_direction(TraversalDirection::Outbound);
        let sub1 = repo.traverse(&rust.id, &opts_d1).await.unwrap();
        assert_eq!(sub1.nodes.len(), 2); // Rust y Actix

        // Traversal a profundidad 2 desde Rust
        let opts_d2 = GraphTraversalOptions::new()
            .with_depth(2)
            .unwrap()
            .with_direction(TraversalDirection::Outbound);
        let sub2 = repo.traverse(&rust.id, &opts_d2).await.unwrap();
        assert_eq!(sub2.nodes.len(), 3); // Rust, Actix y Tokio
        assert_eq!(sub2.edges.len(), 2);
    }
}
