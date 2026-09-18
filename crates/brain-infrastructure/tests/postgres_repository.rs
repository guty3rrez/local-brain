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

#[tokio::test]
async fn postgres_repository_specialized_types_and_session_expiration() {
    use brain_domain::model::{
        AssociativeMemoryData, EpisodicMemoryData, MemoryTypeData, ProceduralMemoryData,
        ProcedureStep, SemanticMemoryData, WorkingMemoryData,
    };

    let Some(repo) = get_test_repository().await else {
        eprintln!("PostgreSQL no disponible, omitiendo test de integración.");
        return;
    };

    let session_id = format!("sess-{}", uuid::Uuid::new_v4());
    let prov = Provenance::new(MemoryOrigin::Observation).with_session(&session_id);

    // 1. Episodic Memory
    let ep_data = EpisodicMemoryData::new(
        "test-proj",
        "antigravity",
        "Migración SQLx",
        "Añadir columna expires_at",
        "Migración idempotente ejecutada",
    )
    .unwrap();
    let ep_mem = Memory::new_episodic_specialized(ep_data.clone(), prov.clone(), None).unwrap();
    repo.save(&ep_mem).await.unwrap();

    let fetched_ep = repo.find_by_id(&ep_mem.id).await.unwrap().unwrap();
    assert_eq!(fetched_ep.memory_type, MemoryType::Episodic);
    assert_eq!(fetched_ep.type_data, MemoryTypeData::Episodic(ep_data));

    // 2. Semantic Memory
    let sem_data = SemanticMemoryData::new(
        "Las migraciones idempotentes reducen fallos en CI",
        Confidence::verified(),
        vec![ep_mem.id],
    )
    .unwrap();
    let sem_mem =
        Memory::new_semantic_specialized(sem_data.clone(), Confidence::verified(), prov.clone())
            .unwrap();
    repo.save(&sem_mem).await.unwrap();

    let fetched_sem = repo.find_by_id(&sem_mem.id).await.unwrap().unwrap();
    assert_eq!(fetched_sem.memory_type, MemoryType::Semantic);
    assert_eq!(fetched_sem.type_data, MemoryTypeData::Semantic(sem_data));

    // 3. Procedural Memory
    let steps = vec![
        ProcedureStep::new(1, "Escribir archivo .sql").unwrap(),
        ProcedureStep::new(2, "Ejecutar sqlx migrate run").unwrap(),
    ];
    let proc_data =
        ProceduralMemoryData::new("Nueva migración", "Aplicar cambios de esquema", steps).unwrap();
    let proc_mem = Memory::new_procedural_specialized(proc_data.clone(), prov.clone()).unwrap();
    repo.save(&proc_mem).await.unwrap();

    let fetched_proc = repo.find_by_id(&proc_mem.id).await.unwrap().unwrap();
    assert_eq!(fetched_proc.memory_type, MemoryType::Procedural);
    assert_eq!(
        fetched_proc.type_data,
        MemoryTypeData::Procedural(proc_data)
    );

    // 4. Associative Memory y find_associations
    let assoc_data = AssociativeMemoryData::new("SQLx", "PostgreSQL", "driver_for", 0.95).unwrap();
    let assoc_mem = Memory::new_associative_specialized(assoc_data.clone(), prov.clone()).unwrap();
    repo.save(&assoc_mem).await.unwrap();

    let fetched_assoc = repo.find_by_id(&assoc_mem.id).await.unwrap().unwrap();
    assert_eq!(fetched_assoc.memory_type, MemoryType::Associative);

    let associations = repo.find_associations("SQLx", 10).await.unwrap();
    assert!(!associations.is_empty());
    assert!(associations.iter().any(|m| m.id == assoc_mem.id));

    // 5. Working Memory con sesión y TTL
    let work_data = WorkingMemoryData::new(&session_id)
        .unwrap()
        .with_goal("Verificar índices de sesión")
        .with_ttl(3600)
        .unwrap();
    let work_mem = Memory::new_working_specialized(work_data, prov.clone()).unwrap();
    repo.save(&work_mem).await.unwrap();

    let session_active = repo.find_active_by_session(&session_id, 10).await.unwrap();
    assert_eq!(session_active.len(), 1);
    assert_eq!(session_active[0].id, work_mem.id);

    // 6. expire_session archiva las working memories de la sesión
    let expired_count = repo.expire_session(&session_id).await.unwrap();
    assert_eq!(expired_count, 1);

    let session_after = repo.find_active_by_session(&session_id, 10).await.unwrap();
    assert!(session_after.is_empty());

    let re_work = repo.find_by_id(&work_mem.id).await.unwrap().unwrap();
    assert_eq!(re_work.status, MemoryStatus::Archived);
}
