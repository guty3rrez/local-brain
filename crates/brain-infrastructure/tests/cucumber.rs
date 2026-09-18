//! Runner de BDD Cucumber en Rust para pruebas de persistencia de memoria y búsqueda vectorial (SRS §25).

use cucumber::{given, then, when, World};
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

use brain_domain::model::{
    AssociativeMemoryData, EpisodicMemoryData, Memory, MemoryContent, MemoryId, MemoryOrigin,
    MemoryStatus, MemoryTypeData, ProceduralMemoryData, ProcedureStep, Provenance,
    SemanticMemoryData, WorkingMemoryData,
};
use brain_domain::ports::{MemoryRepository, VectorRepository, DEFAULT_EMBEDDING_DIMENSION};
use brain_infrastructure::persistence::PostgresMemoryRepository;

#[derive(Debug, Default, World)]
pub struct MemoryWorld {
    repo: Option<PostgresMemoryRepository>,
    saved_memory: Option<Memory>,
    retrieved_memory: Option<Memory>,
    indexed_vector: Option<Vec<f32>>,
    search_results: Option<Vec<(MemoryId, f32)>>,
    domain_error: Option<String>,
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

#[tokio::main]
async fn main() {
    MemoryWorld::run("tests/features").await;
}
