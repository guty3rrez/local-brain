//! Pruebas de integración para PostgresGraphRepository contra PostgreSQL real (SRS §11, §25, F5-02).

use std::time::{Duration, Instant};

use sqlx::postgres::PgPoolOptions;

use brain_graph::errors::GraphError;
use brain_graph::model::{
    GraphEdge, GraphTraversalOptions, NodeType, RelationType, TraversalDirection,
};
use brain_graph::ports::GraphRepository;
use brain_infrastructure::persistence::PostgresGraphRepository;

async fn get_test_graph_repository() -> Option<PostgresGraphRepository> {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://localbrain:localbrain_secret@localhost:5433/local_brain".to_string()
    });

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&database_url)
        .await
        .ok()?;

    let repo = PostgresGraphRepository::new(pool);
    repo.run_migrations()
        .await
        .expect("Las migraciones de grafo deben ejecutarse exitosamente");
    Some(repo)
}

#[tokio::test]
async fn postgres_graph_repository_crud_lifecycle() {
    let Some(repo) = get_test_graph_repository().await else {
        eprintln!("PostgreSQL no disponible, omitiendo test.");
        return;
    };

    let tag = uuid::Uuid::new_v4().to_string();
    let concept_a_label = format!("Concept A {tag}");
    let concept_b_label = format!("Concept B {tag}");

    // 1. Asegurar nodos de conceptos
    let node_a = repo.ensure_concept_node(&concept_a_label).await.unwrap();
    let node_b = repo.ensure_concept_node(&concept_b_label).await.unwrap();

    assert_eq!(node_a.label, concept_a_label);
    assert_eq!(node_a.node_type, NodeType::Concept);

    // 2. Crear arista A --[RELATED_TO]--> B
    let edge = GraphEdge::new(node_a.id, node_b.id, RelationType::RelatedTo, 0.85).unwrap();
    repo.save_edge(&edge).await.unwrap();

    // 3. Buscar arista por ID
    let fetched_edge = repo.find_edge_by_id(&edge.id).await.unwrap();
    assert!(fetched_edge.is_some());
    let fetched = fetched_edge.unwrap();
    assert_eq!(fetched.relation_type, RelationType::RelatedTo);
    assert_eq!(fetched.weight, 0.85);

    // 4. Buscar aristas para el nodo A
    let edges_a = repo
        .find_edges_for_node(&node_a.id, TraversalDirection::Outbound, None)
        .await
        .unwrap();
    assert_eq!(edges_a.len(), 1);
    assert_eq!(edges_a[0].target_id, node_b.id);

    // 5. Eliminar arista
    repo.delete_edge(&edge.id).await.unwrap();
    assert!(repo.find_edge_by_id(&edge.id).await.unwrap().is_none());
}

#[tokio::test]
async fn postgres_graph_cycle_detection_on_dag_relations() {
    let Some(repo) = get_test_graph_repository().await else {
        eprintln!("PostgreSQL no disponible, omitiendo test.");
        return;
    };

    let tag = uuid::Uuid::new_v4().to_string();
    let dec_1_lbl = format!("Decision 1 {tag}");
    let dec_2_lbl = format!("Decision 2 {tag}");

    let n1 = repo.ensure_concept_node(&dec_1_lbl).await.unwrap();
    let n2 = repo.ensure_concept_node(&dec_2_lbl).await.unwrap();

    // Decision 2 supersedes Decision 1
    let e1 = GraphEdge::new(n2.id, n1.id, RelationType::Supersedes, 1.0).unwrap();
    repo.save_edge(&e1).await.unwrap();

    // Intentar Decision 1 supersedes Decision 2 debe fallar con CycleDetected
    let e2 = GraphEdge::new(n1.id, n2.id, RelationType::Supersedes, 1.0).unwrap();
    let err = repo.save_edge(&e2).await.unwrap_err();

    assert!(matches!(err, GraphError::CycleDetected { .. }));
}

#[tokio::test]
async fn postgres_graph_traversal_recursive_cte() {
    let Some(repo) = get_test_graph_repository().await else {
        eprintln!("PostgreSQL no disponible, omitiendo test.");
        return;
    };

    let tag = uuid::Uuid::new_v4().to_string();
    let root_lbl = format!("Root {tag}");
    let child1_lbl = format!("Child 1 {tag}");
    let child2_lbl = format!("Child 2 {tag}");
    let grandchild_lbl = format!("Grandchild {tag}");

    let root = repo.ensure_concept_node(&root_lbl).await.unwrap();
    let child1 = repo.ensure_concept_node(&child1_lbl).await.unwrap();
    let child2 = repo.ensure_concept_node(&child2_lbl).await.unwrap();
    let grandchild = repo.ensure_concept_node(&grandchild_lbl).await.unwrap();

    // Root -> Child 1 (USED_IN, 0.9)
    let e1 = GraphEdge::new(root.id, child1.id, RelationType::UsedIn, 0.9).unwrap();
    repo.save_edge(&e1).await.unwrap();

    // Root -> Child 2 (SOLVES, 0.8)
    let e2 = GraphEdge::new(root.id, child2.id, RelationType::Solves, 0.8).unwrap();
    repo.save_edge(&e2).await.unwrap();

    // Child 1 -> Grandchild (DEPENDS_ON, 0.7)
    let e3 = GraphEdge::new(child1.id, grandchild.id, RelationType::DependsOn, 0.7).unwrap();
    repo.save_edge(&e3).await.unwrap();

    // Traversal profundidad 1 desde Root
    let opts_d1 = GraphTraversalOptions::new()
        .with_depth(1)
        .unwrap()
        .with_direction(TraversalDirection::Outbound);
    let sub_d1 = repo.traverse(&root.id, &opts_d1).await.unwrap();
    assert_eq!(sub_d1.nodes.len(), 3); // Root, Child 1, Child 2

    // Traversal profundidad 2 desde Root
    let opts_d2 = GraphTraversalOptions::new()
        .with_depth(2)
        .unwrap()
        .with_direction(TraversalDirection::Outbound);
    let sub_d2 = repo.traverse(&root.id, &opts_d2).await.unwrap();
    assert_eq!(sub_d2.nodes.len(), 4); // Root, Child 1, Child 2, Grandchild

    // Traversal filtrando únicamente relación SOLVES
    let opts_solves = GraphTraversalOptions::new()
        .with_depth(2)
        .unwrap()
        .with_direction(TraversalDirection::Outbound)
        .with_relations(vec![RelationType::Solves]);
    let sub_solves = repo.traverse(&root.id, &opts_solves).await.unwrap();
    assert_eq!(sub_solves.nodes.len(), 2); // Root y Child 2
}

#[tokio::test]
async fn postgres_graph_1000_nodes_traversal_stress_test() {
    let Some(repo) = get_test_graph_repository().await else {
        eprintln!("PostgreSQL no disponible, omitiendo test de 1.000 nodos.");
        return;
    };

    let run_id = uuid::Uuid::new_v4().to_string();
    println!("Iniciando test de escalabilidad y traversal con 1.000 nodos [{run_id}]...");

    let start_insert = Instant::now();

    // Crear 1.000 nodos en estructura de árbol balanceado
    // Nivel 0: 1 raíz
    // Nivel 1: 10 hijos
    // Nivel 2: 10 * 10 = 100 nietos
    // Nivel 3: 889 bisnietos
    let root_label = format!("Stress-Root-{run_id}");
    let root = repo.ensure_concept_node(&root_label).await.unwrap();

    let mut level1_nodes = Vec::with_capacity(10);
    for i in 0..10 {
        let node = repo
            .ensure_concept_node(&format!("N1-{i}-{run_id}"))
            .await
            .unwrap();
        let edge = GraphEdge::new(root.id, node.id, RelationType::RelatedTo, 0.95).unwrap();
        repo.save_edge(&edge).await.unwrap();
        level1_nodes.push(node);
    }

    let mut level2_nodes = Vec::with_capacity(100);
    for (i, p) in level1_nodes.iter().enumerate() {
        for j in 0..10 {
            let node = repo
                .ensure_concept_node(&format!("N2-{i}-{j}-{run_id}"))
                .await
                .unwrap();
            let edge = GraphEdge::new(p.id, node.id, RelationType::UsedIn, 0.9).unwrap();
            repo.save_edge(&edge).await.unwrap();
            level2_nodes.push(node);
        }
    }

    // Resto hasta completar 1.000 nodos (889 nodos en Nivel 3)
    let remaining = 889;
    for k in 0..remaining {
        let parent = &level2_nodes[k % level2_nodes.len()];
        let node = repo
            .ensure_concept_node(&format!("N3-{k}-{run_id}"))
            .await
            .unwrap();
        let edge = GraphEdge::new(parent.id, node.id, RelationType::DependsOn, 0.85).unwrap();
        repo.save_edge(&edge).await.unwrap();
    }

    let insert_duration = start_insert.elapsed();
    println!(
        "1.000 nodos y 999 aristas creados exitosamente en {:?}",
        insert_duration
    );

    // Ejecutar traversal recursivo a profundidad 3 desde la raíz
    let start_traversal = Instant::now();
    let opts = GraphTraversalOptions::new()
        .with_depth(3)
        .unwrap()
        .with_direction(TraversalDirection::Outbound)
        .with_limit(150);

    let subgraph = repo.traverse(&root.id, &opts).await.unwrap();
    let traversal_duration = start_traversal.elapsed();

    println!(
        "Traversal recursivo multi-salto sobre 1.000 nodos ejecutado en {:?} (retornó {} nodos y {} aristas)",
        traversal_duration,
        subgraph.nodes.len(),
        subgraph.edges.len()
    );

    assert!(
        subgraph.nodes.len() >= 111,
        "Debe haber recuperado al menos la raíz, 10 hijos y 100 nietos"
    );
    assert!(
        traversal_duration < Duration::from_millis(200),
        "El traversal recursivo en PostgreSQL debe ser sub-200ms incluso sobre grafos grandes (fue {:?})",
        traversal_duration
    );
}
