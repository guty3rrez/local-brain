//! Runner de BDD Cucumber en Rust para pruebas de persistencia de memoria (SRS §25).

use cucumber::{given, then, when, World};
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

use brain_domain::model::{Memory, MemoryContent, MemoryOrigin, MemoryStatus, Provenance};
use brain_domain::ports::MemoryRepository;
use brain_infrastructure::persistence::PostgresMemoryRepository;

#[derive(Debug, Default, World)]
pub struct MemoryWorld {
    repo: Option<PostgresMemoryRepository>,
    saved_memory: Option<Memory>,
    retrieved_memory: Option<Memory>,
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

#[tokio::main]
async fn main() {
    MemoryWorld::run("tests/features").await;
}
