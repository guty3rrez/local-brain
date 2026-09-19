//! Pruebas de integración para PostgresConflictRepository contra PostgreSQL real (SRS §16, §17, §18, Fase 7).

use chrono::Utc;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use uuid::Uuid;

use brain_consolidation::model::{ConflictStatus, ConflictType, Contradiction, ReflectionReport};
use brain_consolidation::ports::ConflictRepository;
use brain_domain::model::{Memory, MemoryContent, MemoryOrigin, Provenance};
use brain_domain::ports::MemoryRepository;
use brain_infrastructure::persistence::{PostgresConflictRepository, PostgresMemoryRepository};

async fn get_test_conflict_repository(
) -> Option<(PostgresConflictRepository, PostgresMemoryRepository)> {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://localbrain:localbrain_secret@localhost:5433/local_brain".to_string()
    });

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&database_url)
        .await
        .ok()?;

    let conflict_repo = PostgresConflictRepository::new(pool.clone());
    let memory_repo = PostgresMemoryRepository::new(pool);

    conflict_repo
        .run_migrations()
        .await
        .expect("Las migraciones de consolidación deben ejecutarse exitosamente");

    Some((conflict_repo, memory_repo))
}

#[tokio::test]
async fn postgres_conflict_repository_crud_and_run_lifecycle() {
    let Some((conflict_repo, memory_repo)) = get_test_conflict_repository().await else {
        eprintln!("PostgreSQL no disponible, omitiendo test de integración.");
        return;
    };

    let project = format!("test-proj-{}", Uuid::new_v4());

    // Crear dos memorias base para satisfacer las FK
    let content_a = MemoryContent::new("Supabase es preferido para proyectos pequeños.").unwrap();
    let content_b = MemoryContent::new(".NET es preferido para proyectos pequeños.").unwrap();
    let prov = Provenance::new(MemoryOrigin::Observation);

    let mem_a = Memory::new_episodic(content_a, prov.clone(), Some(project.clone()));
    memory_repo.save(&mem_a).await.unwrap();

    let mem_b = Memory::new_episodic(content_b, prov, Some(project.clone()));
    memory_repo.save(&mem_b).await.unwrap();

    // 1. Crear y persistir contradicción
    let mut conflict = Contradiction::new(
        mem_a.id,
        mem_b.id,
        ConflictType::MutuallyExclusivePreference,
        "Preferencia en conflicto: Supabase vs .NET",
    )
    .unwrap()
    .with_suggested_resolution("Diferenciar por escala y complejidad de dominio");

    conflict_repo
        .save_conflict(&conflict)
        .await
        .expect("Debe guardar conflicto");

    // 2. Recuperar por ID
    let fetched = conflict_repo
        .find_conflict_by_id(&conflict.id)
        .await
        .unwrap()
        .expect("Conflicto debe existir");
    assert_eq!(fetched.id, conflict.id);
    assert_eq!(
        fetched.conflict_type,
        ConflictType::MutuallyExclusivePreference
    );
    assert_eq!(fetched.status, ConflictStatus::Pending);

    // 3. Buscar por par de memorias
    let pair = conflict_repo
        .find_by_memory_pair(&mem_b.id, &mem_a.id)
        .await
        .unwrap()
        .expect("Debe encontrar por par simétrico");
    assert_eq!(pair.id, conflict.id);

    // 4. Listar pendientes por proyecto
    let pending_list = conflict_repo
        .list_pending_conflicts(Some(&project))
        .await
        .unwrap();
    assert!(pending_list.iter().any(|c| c.id == conflict.id));

    // 5. Resolver conflicto
    conflict
        .resolve("Supabase para prototipos rápidos, .NET para arquitecturas complejas")
        .unwrap();
    conflict_repo.update_conflict(&conflict).await.unwrap();

    let resolved = conflict_repo
        .find_conflict_by_id(&conflict.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(resolved.status, ConflictStatus::Resolved);
    assert!(resolved.resolved_at.is_some());

    // 6. Registrar corrida de consolidación
    let report = ReflectionReport {
        run_id: Uuid::now_v7(),
        project: Some(project.clone()),
        memories_analyzed: 2,
        clusters_formed: 1,
        patterns_detected: vec![],
        hypotheses: vec![],
        conflicts_detected: vec![conflict],
        created_candidates: vec![],
        summary: "Corrida de prueba de integración de consolidación".to_string(),
        executed_at: Utc::now(),
    };
    conflict_repo
        .record_consolidation_run(&report)
        .await
        .expect("Debe registrar auditoría de corrida");
}
