//! Pruebas de integración para PostgresMemoryRepository contra PostgreSQL real (SRS §25).

use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

use brain_domain::model::{
    Confidence, DomainError, Importance, Memory, MemoryContent, MemoryId, MemoryOrigin,
    MemoryStatus, MemoryType, Provenance,
};
use brain_domain::ports::MemoryRepository;
use brain_infrastructure::persistence::PostgresMemoryRepository;

async fn get_test_repository() -> Option<PostgresMemoryRepository> {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://localbrain:localbrain_secret@localhost:5433/local_brain".to_string()
    });

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&database_url)
        .await
        .ok()?;

    let repo = PostgresMemoryRepository::new(pool);
    repo.run_migrations()
        .await
        .expect("Las migraciones deben ejecutarse exitosamente");
    Some(repo)
}

#[tokio::test]
async fn postgres_repository_crud_lifecycle() {
    let Some(repo) = get_test_repository().await else {
        eprintln!("PostgreSQL no disponible, omitiendo test de integración.");
        return;
    };

    let unique_tag = uuid::Uuid::new_v4().to_string();
    let project_name = format!("test-project-{unique_tag}");
    let content_text =
        format!("Decisión técnica en Rust con persistencia PostgreSQL [{unique_tag}]");

    let content = MemoryContent::new(&content_text).unwrap();
    let prov = Provenance::new(MemoryOrigin::Observation)
        .with_agent("antigravity")
        .with_session(format!("session-{unique_tag}"))
        .with_context("src/persistence/postgres_repository.rs");

    let mut memory = Memory::new_episodic(content, prov, Some(project_name.clone()));
    memory.importance = Importance::new(0.85).unwrap();
    memory.confidence = Confidence::new(0.95).unwrap();
    let memory_id = memory.id;

    // 1. Guardar memoria
    repo.save(&memory)
        .await
        .expect("Debe guardar la memoria en PostgreSQL");

    // 2. Detección de duplicados (PK violation)
    let duplicate_result = repo.save(&memory).await;
    assert!(
        matches!(duplicate_result, Err(DomainError::RepositoryError(_))),
        "Guardar el mismo ID dos veces debe retornar error de repositorio"
    );

    // 3. Buscar por ID
    let fetched = repo
        .find_by_id(&memory_id)
        .await
        .expect("Consulta por ID no debe fallar")
        .expect("La memoria debe existir en la base de datos");

    assert_eq!(fetched.id, memory_id);
    assert_eq!(fetched.content.text(), content_text);
    assert_eq!(fetched.content.hash(), memory.content.hash());
    assert_eq!(fetched.project.as_deref(), Some(project_name.as_str()));
    assert_eq!(fetched.memory_type, MemoryType::Episodic);
    assert_eq!(fetched.status, MemoryStatus::Active);
    assert_eq!(fetched.importance.value(), 0.85);
    assert_eq!(fetched.confidence.value(), 0.95);
    assert_eq!(fetched.source.origin, MemoryOrigin::Observation);
    assert_eq!(fetched.source.agent.as_deref(), Some("antigravity"));
    assert_eq!(
        fetched.source.session_id.as_deref(),
        Some(format!("session-{unique_tag}").as_str())
    );

    // 4. Buscar por proyecto
    let project_memories = repo
        .find_by_project(&project_name, 10)
        .await
        .expect("Búsqueda por proyecto no debe fallar");
    assert_eq!(project_memories.len(), 1);
    assert_eq!(project_memories[0].id, memory_id);

    // 5. Buscar por tipo
    let type_memories = repo
        .find_by_type(MemoryType::Episodic, 50)
        .await
        .expect("Búsqueda por tipo no debe fallar");
    assert!(type_memories.iter().any(|m| m.id == memory_id));

    // 6. Actualizar contenido y elevar versión
    let updated_text = format!("Decisión técnica actualizada y refactorizada [{unique_tag}]");
    let mut to_update = fetched;
    to_update.update_content(MemoryContent::new(&updated_text).unwrap());
    assert_eq!(to_update.version.value(), 2);

    repo.update(&to_update)
        .await
        .expect("Actualización de memoria debe ser exitosa");

    let refetched = repo
        .find_by_id(&memory_id)
        .await
        .unwrap()
        .expect("Debe existir tras actualización");
    assert_eq!(refetched.content.text(), updated_text);
    assert_eq!(refetched.version.value(), 2);

    // 7. Soft Delete
    repo.soft_delete(&memory_id)
        .await
        .expect("Soft delete debe ser exitoso");

    let deleted = repo
        .find_by_id(&memory_id)
        .await
        .unwrap()
        .expect("Debe seguir existiendo pero marcada como soft-deleted");
    assert_eq!(deleted.status, MemoryStatus::SoftDeleted);

    // 8. Ya no debe aparecer en búsquedas activas por proyecto ni tipo
    let active_by_project = repo.find_by_project(&project_name, 10).await.unwrap();
    assert!(
        active_by_project.is_empty(),
        "Memoria soft-deleted no debe aparecer en proyecto"
    );

    let active_by_type = repo.find_by_type(MemoryType::Episodic, 50).await.unwrap();
    assert!(
        !active_by_type.iter().any(|m| m.id == memory_id),
        "Memoria soft-deleted no debe aparecer en búsqueda por tipo"
    );
}

#[tokio::test]
async fn postgres_repository_not_found_errors() {
    let Some(repo) = get_test_repository().await else {
        eprintln!("PostgreSQL no disponible, omitiendo test de integración.");
        return;
    };

    let nonexistent_id = MemoryId::new();

    // find_by_id retorna None para IDs inexistentes
    let result = repo.find_by_id(&nonexistent_id).await.unwrap();
    assert!(result.is_none());

    // soft_delete retorna error para IDs inexistentes
    let delete_result = repo.soft_delete(&nonexistent_id).await;
    assert!(matches!(
        delete_result,
        Err(DomainError::RepositoryError(_))
    ));

    // update retorna error para memorias inexistentes
    let fake_mem = Memory::new(
        MemoryContent::new("Inexistente").unwrap(),
        MemoryType::Working,
        Provenance::new(MemoryOrigin::System),
    );
    let update_result = repo.update(&fake_mem).await;
    assert!(matches!(
        update_result,
        Err(DomainError::RepositoryError(_))
    ));
}
