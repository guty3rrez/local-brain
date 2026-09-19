//! Runner de BDD Cucumber en Rust para pruebas de persistencia de memoria y búsqueda vectorial (SRS §25).

use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use cucumber::{given, then, when, World};
use sqlx::postgres::PgPoolOptions;

use brain_application::{
    AddEvidenceCommand, AddEvidenceUseCase, ExplainQuery, ExplainUseCase, ExplanationReport,
    LearnCommand, LearnResult, LearnUseCase,
};
use brain_domain::model::{
    AssociativeMemoryData, EpisodicMemoryData, Memory, MemoryContent, MemoryId, MemoryOrigin,
    MemoryStatus, MemoryType, MemoryTypeData, ProceduralMemoryData, ProcedureStep, Provenance,
    SemanticMemoryData, WorkingMemoryData,
};
use brain_domain::ports::{MemoryRepository, VectorRepository, DEFAULT_EMBEDDING_DIMENSION};
use brain_graph::errors::GraphError;
use brain_graph::model::{
    GraphEdge, GraphSubgraph, GraphTraversalOptions, NodeType, RelationType, TraversalDirection,
};
use brain_graph::ports::GraphRepository;
use brain_infrastructure::persistence::{
    PostgresGraphRepository, PostgresLearningRepository, PostgresMemoryRepository,
};
use brain_learning::model::EvidenceSourceType;
use brain_learning::ports::LearningRepository;
use std::sync::Arc;

#[derive(Default, World)]
pub struct MemoryWorld {
    repo: Option<PostgresMemoryRepository>,
    graph_repo: Option<PostgresGraphRepository>,
    learning_repo: Option<PostgresLearningRepository>,
    saved_memory: Option<Memory>,
    retrieved_memory: Option<Memory>,
    indexed_vector: Option<Vec<f32>>,
    search_results: Option<Vec<(MemoryId, f32)>>,
    domain_error: Option<String>,
    graph_error: Option<String>,
    decision_memories: HashMap<String, Memory>,
    graph_subgraph: Option<GraphSubgraph>,
    last_learn_result: Option<LearnResult>,
    explanation_report: Option<ExplanationReport>,
    learn_uc: Option<Arc<LearnUseCase>>,
    add_evidence_uc: Option<Arc<AddEvidenceUseCase>>,
    explain_uc: Option<Arc<ExplainUseCase>>,
}

impl std::fmt::Debug for MemoryWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryWorld").finish()
    }
}

#[given(expr = "un repositorio PostgreSQL conectado y con migraciones aplicadas")]
async fn given_connected_repo(world: &mut MemoryWorld) {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://localbrain:localbrain_secret@localhost:5433/local_brain".to_string()
    });

    let pool = PgPoolOptions::new()
        .max_connections(3)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&database_url)
        .await
        .expect("Debe conectar con la base de datos PostgreSQL local");

    let repo = PostgresMemoryRepository::new(pool);
    repo.run_migrations()
        .await
        .expect("Debe ejecutar las migraciones de forma idempotente");

    world.repo = Some(repo);
}

#[when(
    expr = "guardo un nuevo recuerdo episódico con contenido {string} para el proyecto {string}"
)]
async fn when_saving_memory(world: &mut MemoryWorld, content_str: String, project_str: String) {
    let repo = world.repo.as_ref().expect("Repositorio no inicializado");
    let content = MemoryContent::new(content_str).expect("Contenido de memoria válido");
    let prov = Provenance::new(MemoryOrigin::Observation).with_agent("antigravity");
    let memory = Memory::new_episodic(content, prov, Some(project_str));

    repo.save(&memory)
        .await
        .expect("Debe guardar la memoria en PostgreSQL");
    world.saved_memory = Some(memory);
}

#[then(expr = "puedo recuperar el recuerdo utilizando su ID")]
async fn then_can_retrieve_by_id(world: &mut MemoryWorld) {
    let repo = world.repo.as_ref().expect("Repositorio no inicializado");
    let saved = world.saved_memory.as_ref().expect("Memoria no guardada");

    let retrieved = repo
        .find_by_id(&saved.id)
        .await
        .expect("Error al consultar por ID")
        .expect("La memoria guardada debe encontrarse en PostgreSQL");

    world.retrieved_memory = Some(retrieved);
}

#[then(expr = "el contenido recuperado coincide exactamente con {string}")]
async fn then_content_matches(world: &mut MemoryWorld, expected_content: String) {
    let retrieved = world
        .retrieved_memory
        .as_ref()
        .expect("Memoria no recuperada");
    assert_eq!(retrieved.content.text(), expected_content);
}

#[then(expr = "el estado del recuerdo es {string}")]
async fn then_status_matches(world: &mut MemoryWorld, expected_status: String) {
    let retrieved = world
        .retrieved_memory
        .as_ref()
        .expect("Memoria no recuperada");
    let expected = match expected_status.as_str() {
        "active" => MemoryStatus::Active,
        "archived" => MemoryStatus::Archived,
        "soft_deleted" => MemoryStatus::SoftDeleted,
        other => panic!("Estado no reconocido: {other}"),
    };
    assert_eq!(retrieved.status, expected);
}

// Pasos para Búsqueda Semántica Vectorial (semantic_recall.feature)

#[given(expr = "un recuerdo indexado con embedding de 768 dimensiones y contenido {string}")]
async fn given_indexed_memory_with_embedding(world: &mut MemoryWorld, content_str: String) {
    let repo = world.repo.as_ref().expect("Repositorio no inicializado");
    let content = MemoryContent::new(content_str).expect("Contenido válido");
    let prov = Provenance::new(MemoryOrigin::Observation).with_agent("bdd-agent");
    let mut memory = Memory::new_episodic(content, prov, Some("bdd-semantic".to_string()));

    let mut vector = vec![0.0f32; DEFAULT_EMBEDDING_DIMENSION];
    vector[42] = 1.0; // Vector unitario en dimensión 42
    memory.embedding = Some(vector.clone());

    repo.save(&memory)
        .await
        .expect("Debe guardar memoria con embedding");

    world.saved_memory = Some(memory);
    world.indexed_vector = Some(vector);
}

#[when(expr = "realizo una búsqueda vectorial con un vector de consulta similar")]
async fn when_vector_search(world: &mut MemoryWorld) {
    let repo = world.repo.as_ref().expect("Repositorio no inicializado");
    let query_vector = world.indexed_vector.as_ref().expect("Vector no disponible");

    let results = repo
        .search_similar(query_vector, 5)
        .await
        .expect("Debe ejecutar búsqueda similar");

    world.search_results = Some(results);
}

#[then(expr = "el resultado más similar contiene {string}")]
async fn then_top_result_contains(world: &mut MemoryWorld, expected_text: String) {
    let repo = world.repo.as_ref().expect("Repositorio no inicializado");
    let results = world
        .search_results
        .as_ref()
        .expect("Sin resultados de búsqueda");

    assert!(
        !results.is_empty(),
        "La búsqueda vectorial debe retornar al menos un resultado"
    );
    let (top_id, _) = &results[0];

    let memory = repo
        .find_by_id(top_id)
        .await
        .expect("Error al buscar memoria")
        .expect("Memoria top debe existir");

    assert!(
        memory.content.text().contains(&expected_text),
        "El contenido '{}' debe contener '{}'",
        memory.content.text(),
        expected_text
    );
}

#[then(expr = "la similitud coseno calculada es superior a {float}")]
async fn then_similarity_above(world: &mut MemoryWorld, threshold: f32) {
    let results = world
        .search_results
        .as_ref()
        .expect("Sin resultados de búsqueda");
    assert!(!results.is_empty());
    let (_, score) = &results[0];
    assert!(
        *score > threshold,
        "Similitud coseno {} debe ser mayor a {}",
        score,
        threshold
    );
}

// Pasos para Tipos Especializados de Memoria (specialized_memory_types.feature)

use brain_domain::model::Confidence;

#[when(
    expr = "guardo un recuerdo episódico con contexto {string} y acción {string} y resultado {string} para el proyecto {string}"
)]
async fn when_saving_specialized_episodic(
    world: &mut MemoryWorld,
    context_str: String,
    action_str: String,
    outcome_str: String,
    project_str: String,
) {
    let repo = world.repo.as_ref().expect("Repositorio no inicializado");
    let prov = Provenance::new(MemoryOrigin::Observation).with_agent("bdd-agent");
    let episodic = EpisodicMemoryData::new(
        project_str,
        "bdd-agent",
        context_str,
        action_str,
        outcome_str,
    )
    .unwrap();
    let memory = Memory::new_episodic_specialized(episodic, prov, None).unwrap();

    repo.save(&memory)
        .await
        .expect("Debe guardar recuerdo episódico especializado");
    world.saved_memory = Some(memory);
}

#[then(expr = "puedo recuperar el recuerdo episódico por su ID")]
async fn then_retrieve_episodic_by_id(world: &mut MemoryWorld) {
    let repo = world.repo.as_ref().expect("Repositorio no inicializado");
    let saved = world.saved_memory.as_ref().expect("Memoria no guardada");
    let retrieved = repo
        .find_by_id(&saved.id)
        .await
        .expect("Error al consultar ID")
        .expect("Memoria no encontrada");
    world.retrieved_memory = Some(retrieved);
}

#[then(
    expr = "el contexto episódico es {string}, la acción es {string} y el resultado es {string}"
)]
async fn then_episodic_fields_match(
    world: &mut MemoryWorld,
    expected_context: String,
    expected_action: String,
    expected_outcome: String,
) {
    let retrieved = world
        .retrieved_memory
        .as_ref()
        .expect("Memoria no recuperada");
    match &retrieved.type_data {
        MemoryTypeData::Episodic(data) => {
            assert_eq!(data.context, expected_context);
            assert_eq!(data.action, expected_action);
            assert_eq!(data.outcome, expected_outcome);
        }
        other => panic!("Tipo de memoria no esperado: {other:?}"),
    }
}

#[when(
    expr = "guardo un recuerdo de trabajo para la sesión {string} con contenido {string} para el proyecto {string}"
)]
async fn when_saving_working_memory(
    world: &mut MemoryWorld,
    session_id: String,
    content_str: String,
    _project_str: String,
) {
    let repo = world.repo.as_ref().expect("Repositorio no inicializado");
    let prov = Provenance::new(MemoryOrigin::Observation).with_agent("bdd-agent");
    let working = WorkingMemoryData::new(session_id)
        .unwrap()
        .with_goal(content_str)
        .with_ttl(3600)
        .unwrap();
    let memory = Memory::new_working_specialized(working, prov).unwrap();

    repo.save(&memory)
        .await
        .expect("Debe guardar recuerdo de trabajo");
    world.saved_memory = Some(memory);
}

#[then(regex = r#"^encuentro (\d+) recuerdos? activos? en la sesión "([^"]+)"$"#)]
async fn then_find_active_by_session(world: &mut MemoryWorld, count: usize, session_id: String) {
    let repo = world.repo.as_ref().expect("Repositorio no inicializado");
    let active = repo
        .find_active_by_session(&session_id, 100)
        .await
        .expect("Error al buscar recuerdos de sesión");
    assert_eq!(
        active.len(),
        count,
        "Se esperaban {} recuerdos activos pero se encontraron {}",
        count,
        active.len()
    );
}

#[when(expr = "expiro la sesión {string}")]
async fn when_expire_session(world: &mut MemoryWorld, session_id: String) {
    let repo = world.repo.as_ref().expect("Repositorio no inicializado");
    repo.expire_session(&session_id)
        .await
        .expect("Error al expirar sesión");
}

#[when(
    expr = "guardo un recuerdo procedimental para la tarea {string} con {int} pasos para el proyecto {string}"
)]
async fn when_saving_procedural_memory(
    world: &mut MemoryWorld,
    task_name: String,
    step_count: usize,
    _project_str: String,
) {
    let repo = world.repo.as_ref().expect("Repositorio no inicializado");
    let mut steps = Vec::new();
    for i in 1..=step_count {
        steps.push(ProcedureStep::new(i as u32, format!("Paso {i}")).unwrap());
    }
    let procedural = ProceduralMemoryData::new(task_name.clone(), "Meta de prueba", steps).unwrap();
    let prov = Provenance::new(MemoryOrigin::Observation).with_agent("bdd-agent");
    let memory = Memory::new_procedural_specialized(procedural, prov).unwrap();

    repo.save(&memory)
        .await
        .expect("Debe guardar recuerdo procedimental");
    world.saved_memory = Some(memory);
}

#[then(expr = "puedo recuperar el recuerdo procedimental por su ID")]
async fn then_retrieve_procedural_by_id(world: &mut MemoryWorld) {
    let repo = world.repo.as_ref().expect("Repositorio no inicializado");
    let saved = world.saved_memory.as_ref().expect("Memoria no guardada");
    let retrieved = repo
        .find_by_id(&saved.id)
        .await
        .expect("Error al consultar ID")
        .expect("Memoria no encontrada");
    world.retrieved_memory = Some(retrieved);
}

#[then(expr = "el procedimiento tiene {int} pasos y la versión es {int}")]
async fn then_procedural_fields_match(
    world: &mut MemoryWorld,
    expected_steps: usize,
    expected_version: u32,
) {
    let retrieved = world
        .retrieved_memory
        .as_ref()
        .expect("Memoria no recuperada");
    match &retrieved.type_data {
        MemoryTypeData::Procedural(data) => {
            assert_eq!(data.steps.len(), expected_steps);
            assert_eq!(data.step_version, expected_version);
        }
        other => panic!("Tipo de memoria no esperado: {other:?}"),
    }
}

#[when(
    expr = "guardo una asociación desde {string} hacia {string} con predicado {string} para el proyecto {string}"
)]
async fn when_saving_associative_memory(
    world: &mut MemoryWorld,
    source: String,
    target: String,
    predicate: String,
    _project_str: String,
) {
    let repo = world.repo.as_ref().expect("Repositorio no inicializado");
    let associative =
        AssociativeMemoryData::new(source.clone(), target.clone(), predicate.clone(), 0.9).unwrap();
    let prov = Provenance::new(MemoryOrigin::Observation).with_agent("bdd-agent");
    let memory = Memory::new_associative_specialized(associative, prov).unwrap();

    repo.save(&memory)
        .await
        .expect("Debe guardar recuerdo asociativo");
    world.saved_memory = Some(memory);
}

#[then(expr = "al buscar asociaciones para {string} encuentro relación con {string}")]
async fn then_find_associations_match(
    world: &mut MemoryWorld,
    concept: String,
    expected_target: String,
) {
    let repo = world.repo.as_ref().expect("Repositorio no inicializado");
    let assocs = repo
        .find_associations(&concept, 10)
        .await
        .expect("Error buscando asociaciones");

    let found = assocs.iter().any(|m| match &m.type_data {
        MemoryTypeData::Associative(data) => {
            (data.source_concept == concept && data.target_concept == expected_target)
                || (data.target_concept == concept && data.source_concept == expected_target)
        }
        _ => false,
    });
    assert!(
        found,
        "No se encontró asociación entre '{concept}' y '{expected_target}'"
    );
}

#[given(expr = "que intento crear una memoria semántica con confianza {float} y sin evidencias")]
async fn given_invalid_semantic_attempt(world: &mut MemoryWorld, confidence: f32) {
    let conf = Confidence::new(confidence).unwrap();
    match SemanticMemoryData::new("Afirmación sin evidencia", conf, vec![]) {
        Ok(_) => world.domain_error = None,
        Err(err) => world.domain_error = Some(err.to_string()),
    }
}

#[then(expr = "la creación es rechazada por regla de invariante")]
async fn then_rejected_by_invariant(world: &mut MemoryWorld) {
    assert!(
        world.domain_error.is_some(),
        "Se esperaba que la creación fuera rechazada por invariante"
    );
}

// Pasos para Grafo de Conocimiento (knowledge_graph.feature)

#[given(expr = "un repositorio PostgreSQL conectado con tablas de grafo")]
async fn given_connected_graph_repo(world: &mut MemoryWorld) {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://localbrain:localbrain_secret@localhost:5433/local_brain".to_string()
    });

    let pool = PgPoolOptions::new()
        .max_connections(3)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&database_url)
        .await
        .expect("Debe conectar con la base de datos PostgreSQL local");

    let graph_repo = PostgresGraphRepository::new(pool.clone());
    graph_repo
        .run_migrations()
        .await
        .expect("Debe ejecutar las migraciones de grafo");

    let mem_repo = PostgresMemoryRepository::new(pool);
    world.graph_repo = Some(graph_repo);
    world.repo = Some(mem_repo);
}

#[when(expr = "registro un recuerdo con decisión {string}")]
#[when(expr = "registro un nuevo recuerdo con decisión {string}")]
async fn when_register_decision_memory(world: &mut MemoryWorld, decision_text: String) {
    let repo = world.repo.as_ref().expect("Repositorio no inicializado");
    let content = MemoryContent::new(&decision_text).unwrap();
    let prov = Provenance::new(MemoryOrigin::UserPrompt).with_agent("bdd-agent");
    let memory = Memory::new(content, MemoryType::Episodic, prov);

    repo.save(&memory)
        .await
        .expect("Debe guardar recuerdo de decisión");
    world.decision_memories.insert(decision_text, memory);
}

#[when(expr = "vinculo {string} hacia {string} con relación {string}")]
async fn when_link_memories(
    world: &mut MemoryWorld,
    source_label: String,
    target_label: String,
    relation_str: String,
) {
    let graph = world
        .graph_repo
        .as_ref()
        .expect("Graph repo no inicializado");
    let relation = RelationType::from_str(&relation_str).expect("Tipo de relación válido");

    let find_mem = |lbl: &str| -> Option<Memory> {
        if let Some(m) = world.decision_memories.get(lbl) {
            return Some(m.clone());
        }
        world
            .decision_memories
            .iter()
            .find(|(k, _)| k.contains(lbl) || lbl.contains(k.as_str()))
            .map(|(_, v)| v.clone())
    };

    let source_node = if let Some(mem) = find_mem(&source_label) {
        graph
            .ensure_memory_node(&mem.id, &source_label)
            .await
            .unwrap()
    } else {
        graph.ensure_concept_node(&source_label).await.unwrap()
    };

    let target_node = if let Some(mem) = find_mem(&target_label) {
        graph
            .ensure_memory_node(&mem.id, &target_label)
            .await
            .unwrap()
    } else {
        graph.ensure_concept_node(&target_label).await.unwrap()
    };

    let edge = GraphEdge::new(source_node.id, target_node.id, relation, 1.0).unwrap();
    match graph.save_edge(&edge).await {
        Ok(_) => world.graph_error = None,
        Err(e) => world.graph_error = Some(e.to_string()),
    }
}

#[then(expr = "el grafo contiene la arista {string} entre ambas decisiones")]
async fn then_graph_contains_edge(world: &mut MemoryWorld, relation_str: String) {
    let graph = world
        .graph_repo
        .as_ref()
        .expect("Graph repo no inicializado");
    let relation = RelationType::from_str(&relation_str).expect("Tipo de relación válido");

    let dec_b = world
        .decision_memories
        .get("Decisión B: PostgreSQL")
        .expect("Decisión B debe existir");
    let dec_a = world
        .decision_memories
        .get("Decisión A: SQLite")
        .expect("Decisión A debe existir");

    let node_b = graph
        .find_node_by_memory_id(&dec_b.id)
        .await
        .unwrap()
        .expect("Nodo B debe existir");
    let node_a = graph
        .find_node_by_memory_id(&dec_a.id)
        .await
        .unwrap()
        .expect("Nodo A debe existir");

    let edge = graph
        .find_edge(&node_b.id, &node_a.id, relation)
        .await
        .unwrap();
    assert!(
        edge.is_some(),
        "Debe existir la arista {relation_str} entre Decisión B y Decisión A"
    );
}

#[then(
    expr = "al intentar vincular {string} hacia {string} como {string} la operación es rechazada por detección de ciclo"
)]
async fn then_reverse_link_rejected_by_cycle(
    world: &mut MemoryWorld,
    source_label: String,
    target_label: String,
    relation_str: String,
) {
    let graph = world
        .graph_repo
        .as_ref()
        .expect("Graph repo no inicializado");
    let relation = RelationType::from_str(&relation_str).expect("Tipo de relación válido");

    let find_mem = |lbl: &str| -> Memory {
        if let Some(m) = world.decision_memories.get(lbl) {
            return m.clone();
        }
        world
            .decision_memories
            .iter()
            .find(|(k, _)| k.contains(lbl) || lbl.contains(k.as_str()))
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| panic!("Memoria '{lbl}' no encontrada"))
    };

    let mem_src = find_mem(&source_label);
    let mem_tgt = find_mem(&target_label);

    let node_src = graph
        .find_node_by_memory_id(&mem_src.id)
        .await
        .unwrap()
        .unwrap();
    let node_tgt = graph
        .find_node_by_memory_id(&mem_tgt.id)
        .await
        .unwrap()
        .unwrap();

    let edge = GraphEdge::new(node_src.id, node_tgt.id, relation, 1.0).unwrap();
    let result = graph.save_edge(&edge).await;

    assert!(
        matches!(result, Err(GraphError::CycleDetected { .. })),
        "Se esperaba CycleDetected al intentar crear ciclo en relación acíclica"
    );
}

#[when(expr = "vinculo el concepto {string} hacia {string} con relación {string} y peso {float}")]
async fn when_link_concepts_with_weight(
    world: &mut MemoryWorld,
    source_concept: String,
    target_concept: String,
    relation_str: String,
    weight: f32,
) {
    let graph = world
        .graph_repo
        .as_ref()
        .expect("Graph repo no inicializado");
    let relation = RelationType::from_str(&relation_str).expect("Tipo de relación válido");

    let src = graph.ensure_concept_node(&source_concept).await.unwrap();
    let tgt = graph.ensure_concept_node(&target_concept).await.unwrap();

    let edge = GraphEdge::new(src.id, tgt.id, relation, weight).unwrap();
    graph.save_edge(&edge).await.unwrap();
}

#[then(
    expr = "al consultar el grafo para {string} a profundidad {int} obtengo los nodos {string} y {string}"
)]
async fn then_traverse_returns_nodes(
    world: &mut MemoryWorld,
    root_label: String,
    depth: u32,
    expected_n1: String,
    expected_n2: String,
) {
    let graph = world
        .graph_repo
        .as_ref()
        .expect("Graph repo no inicializado");
    let root = graph
        .find_node_by_label(&root_label, NodeType::Concept)
        .await
        .unwrap()
        .expect("Nodo raíz debe existir");

    let opts = GraphTraversalOptions::new()
        .with_depth(depth)
        .unwrap()
        .with_direction(TraversalDirection::Outbound);

    let subgraph = graph.traverse(&root.id, &opts).await.unwrap();
    world.graph_subgraph = Some(subgraph.clone());

    let has_n1 = subgraph.nodes.iter().any(|n| n.label == expected_n1);
    let has_n2 = subgraph.nodes.iter().any(|n| n.label == expected_n2);

    assert!(has_n1, "El subgrafo debe contener el nodo '{expected_n1}'");
    assert!(has_n2, "El subgrafo debe contener el nodo '{expected_n2}'");
}

// =========================================================================
// Pasos BDD para el Motor de Aprendizaje y Conocimiento Candidato (Fase 6)
// =========================================================================

#[given(expr = "un repositorio PostgreSQL conectado con tablas de aprendizaje")]
async fn given_connected_learning_repo(world: &mut MemoryWorld) {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://localbrain:localbrain_secret@localhost:5433/local_brain".to_string()
    });

    let pool = PgPoolOptions::new()
        .max_connections(3)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&database_url)
        .await
        .expect("Debe conectar con PostgreSQL");

    let repo = PostgresLearningRepository::new(pool.clone());
    repo.run_migrations()
        .await
        .expect("Debe ejecutar migraciones de aprendizaje");

    sqlx::query("DELETE FROM learning_evidence")
        .execute(&pool)
        .await
        .expect("Debe limpiar tabla learning_evidence");
    sqlx::query("DELETE FROM learning_candidates")
        .execute(&pool)
        .await
        .expect("Debe limpiar tabla learning_candidates");

    let repo_arc = Arc::new(repo.clone());
    world.learning_repo = Some(repo);
    world.learn_uc = Some(Arc::new(LearnUseCase::new(repo_arc.clone())));
    world.add_evidence_uc = Some(Arc::new(AddEvidenceUseCase::new(repo_arc.clone())));
    world.explain_uc = Some(Arc::new(ExplainUseCase::new(repo_arc)));
}

#[when(expr = "registro una observación factual {string} para el dominio {string}")]
async fn when_register_observation(world: &mut MemoryWorld, statement: String, domain: String) {
    let uc = world
        .learn_uc
        .as_ref()
        .expect("LearnUseCase no inicializado");
    let cmd = LearnCommand::new_observation(statement)
        .with_domain(domain)
        .with_agent("bdd-agent");
    let res = uc.execute(cmd).await.expect("Debe registrar observación");
    world.last_learn_result = Some(res);
}

#[then(expr = "la observación se almacena en etapa {string} con confianza menor a {float}")]
async fn then_observation_stored(
    world: &mut MemoryWorld,
    expected_stage: String,
    max_confidence: f32,
) {
    let res = world
        .last_learn_result
        .as_ref()
        .expect("No hay resultado de aprendizaje");
    assert_eq!(res.stage.as_str(), expected_stage.to_lowercase().as_str());
    assert!(
        res.confidence < max_confidence,
        "La confianza {} debe ser menor a {}",
        res.confidence,
        max_confidence
    );
}

#[when(expr = "propongo una creencia candidata {string} con evidencia {string}")]
async fn when_propose_candidate(world: &mut MemoryWorld, statement: String, evidence: String) {
    let uc = world
        .learn_uc
        .as_ref()
        .expect("LearnUseCase no inicializado");
    let cmd = LearnCommand::new_candidate(statement)
        .with_evidence(evidence, EvidenceSourceType::DirectObservation)
        .with_agent("bdd-agent");
    let res = uc.execute(cmd).await.expect("Debe registrar candidato");
    world.last_learn_result = Some(res);
}

#[then(expr = "la creencia se registra en etapa {string}")]
async fn then_belief_registered(world: &mut MemoryWorld, expected_stage: String) {
    let res = world
        .last_learn_result
        .as_ref()
        .expect("No hay resultado de aprendizaje");
    assert_eq!(res.stage.as_str(), expected_stage.to_lowercase().as_str());
}

#[when(expr = "agrego la evidencia de herramienta {string} de soporte")]
async fn when_add_tool_evidence(world: &mut MemoryWorld, content: String) {
    let uc = world
        .add_evidence_uc
        .as_ref()
        .expect("AddEvidenceUseCase no inicializado");
    let last = world
        .last_learn_result
        .as_ref()
        .expect("No hay candidato activo");
    let cmd = AddEvidenceCommand::new(
        last.candidate_id,
        content,
        EvidenceSourceType::ToolExecution,
        true,
    )
    .with_agent("bdd-tool");
    let res = uc.execute(cmd).await.expect("Debe agregar evidencia");
    world.last_learn_result = Some(res);
}

#[then(expr = "la creencia madura a etapa {string} con confianza superior a {float}")]
async fn then_belief_matures(world: &mut MemoryWorld, expected_stage: String, min_conf: f32) {
    let res = world
        .last_learn_result
        .as_ref()
        .expect("No hay resultado de aprendizaje");
    assert_eq!(res.stage.as_str(), expected_stage.to_lowercase().as_str());
    assert!(
        res.confidence > min_conf,
        "La confianza {} debe ser mayor a {}",
        res.confidence,
        min_conf
    );
}

#[when(expr = "un agente afirma {string} sin evidencias ni validación humana")]
async fn when_agent_hallucinates(world: &mut MemoryWorld, statement: String) {
    let uc = world
        .learn_uc
        .as_ref()
        .expect("LearnUseCase no inicializado");
    let cmd = LearnCommand::new_candidate(statement).with_agent("hallucinating-agent");
    let res = uc.execute(cmd).await.expect("Debe registrar candidato");
    world.last_learn_result = Some(res);
}

#[then(expr = "la afirmación permanece en etapa {string}")]
async fn then_statement_remains(world: &mut MemoryWorld, expected_stage: String) {
    let res = world.last_learn_result.as_ref().expect("No hay resultado");
    assert_eq!(res.stage.as_str(), expected_stage.to_lowercase().as_str());
}

#[then(expr = "la confianza asignada no supera el umbral tentativo de {float}")]
async fn then_confidence_below(world: &mut MemoryWorld, threshold: f32) {
    let res = world.last_learn_result.as_ref().expect("No hay resultado");
    assert!(
        res.confidence <= threshold,
        "La confianza {} debe ser <= {}",
        res.confidence,
        threshold
    );
}

#[then(expr = "no puede ascender a {string} sin respaldo empírico")]
async fn then_cannot_promote(world: &mut MemoryWorld, disallowed_stage: String) {
    let res = world.last_learn_result.as_ref().expect("No hay resultado");
    assert_ne!(res.stage.as_str(), disallowed_stage.to_lowercase().as_str());
}

#[when(expr = "registro el conocimiento candidato {string} para el dominio {string}")]
async fn when_register_candidate_with_domain(
    world: &mut MemoryWorld,
    statement: String,
    domain: String,
) {
    let uc = world
        .learn_uc
        .as_ref()
        .expect("LearnUseCase no inicializado");
    let cmd = LearnCommand::new_candidate(statement)
        .with_domain(domain)
        .with_agent("architect-agent");
    let res = uc.execute(cmd).await.expect("Debe registrar candidato");
    world.last_learn_result = Some(res);
}

#[when(expr = "agrego la evidencia {string} de soporte")]
async fn when_add_evidence_generic(world: &mut MemoryWorld, content: String) {
    let uc = world
        .add_evidence_uc
        .as_ref()
        .expect("AddEvidenceUseCase no inicializado");
    let last = world
        .last_learn_result
        .as_ref()
        .expect("No hay candidato activo");
    let cmd = AddEvidenceCommand::new(
        last.candidate_id,
        content,
        EvidenceSourceType::DirectObservation,
        true,
    )
    .with_agent("bdd-agent");
    let res = uc.execute(cmd).await.expect("Debe agregar evidencia");
    world.last_learn_result = Some(res);
}

#[when(expr = "el conocimiento es validado formalmente por un humano")]
async fn when_human_validates(world: &mut MemoryWorld) {
    let last = world
        .last_learn_result
        .as_ref()
        .expect("No hay candidato activo");
    let repo = world.learning_repo.as_ref().expect("Repo no disponible");
    let mut cand = repo
        .find_candidate_by_id(&last.candidate_id)
        .await
        .unwrap()
        .unwrap();
    cand.human_validated = true;
    brain_learning::invariants::LearningInvariants::evaluate_progression(&mut cand).unwrap();
    repo.update_candidate(&cand).await.unwrap();
    world.last_learn_result = Some(LearnResult {
        candidate_id: cand.id,
        statement: cand.statement,
        stage: cand.stage,
        confidence: cand.confidence.value(),
        evidence_count: cand.evidences.len(),
        human_validated: cand.human_validated,
        domain: cand.domain,
    });
}

#[when(expr = "solicito la explicación para {string}")]
async fn when_request_explanation(world: &mut MemoryWorld, query: String) {
    let uc = world
        .explain_uc
        .as_ref()
        .expect("ExplainUseCase no inicializado");
    let report = uc
        .execute(ExplainQuery::new(query))
        .await
        .expect("Debe generar informe de explicación");
    world.explanation_report = Some(report);
}

#[then(expr = "la explicación contiene la conclusión {string}")]
async fn then_explanation_has_conclusion(world: &mut MemoryWorld, expected: String) {
    let report = world
        .explanation_report
        .as_ref()
        .expect("No hay informe de explicación");
    assert_eq!(report.conclusion, expected);
}

#[then(expr = "la explicación reporta confianza superior a {float}")]
async fn then_explanation_confidence_above(world: &mut MemoryWorld, threshold: f32) {
    let report = world
        .explanation_report
        .as_ref()
        .expect("No hay informe de explicación");
    assert!(
        report.confidence > threshold,
        "Confianza {} debe ser mayor a {}",
        report.confidence,
        threshold
    );
}

#[then(expr = "la explicación detalla las evidencias {string} y {string}")]
async fn then_explanation_has_evidences(world: &mut MemoryWorld, ev1: String, ev2: String) {
    let report = world
        .explanation_report
        .as_ref()
        .expect("No hay informe de explicación");
    let has_ev1 = report.evidences.iter().any(|e| e.content.contains(&ev1));
    let has_ev2 = report.evidences.iter().any(|e| e.content.contains(&ev2));
    assert!(has_ev1, "La explicación debe detallar la evidencia '{ev1}'");
    assert!(has_ev2, "La explicación debe detallar la evidencia '{ev2}'");
}

#[then(expr = "la explicación indica validación humana aprobada")]
async fn then_explanation_human_validated(world: &mut MemoryWorld) {
    let report = world
        .explanation_report
        .as_ref()
        .expect("No hay informe de explicación");
    assert!(
        report.human_validated,
        "Debe indicar validación humana aprobada"
    );
    assert!(report
        .formatted_explanation
        .contains("Validación Humana:\nSí"));
}

#[tokio::main]
async fn main() {
    MemoryWorld::cucumber()
        .max_concurrent_scenarios(1)
        .run_and_exit("tests/features")
        .await;
}
