//! Pruebas de integración para PostgresLearningRepository contra PostgreSQL real (SRS §15, §59, §60, Fase 6).

use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

use brain_infrastructure::persistence::PostgresLearningRepository;
use brain_learning::model::{CandidateKnowledge, Evidence, EvidenceSourceType, LearningStage};
use brain_learning::ports::LearningRepository;

async fn get_test_learning_repository() -> Option<PostgresLearningRepository> {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://localbrain:localbrain_secret@localhost:5433/local_brain".to_string()
    });

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&database_url)
        .await
        .ok()?;

    let repo = PostgresLearningRepository::new(pool);
    repo.run_migrations()
        .await
        .expect("Las migraciones de aprendizaje deben ejecutarse exitosamente");
    Some(repo)
}

#[tokio::test]
async fn postgres_learning_repository_crud_lifecycle() {
    let Some(repo) = get_test_learning_repository().await else {
        eprintln!("PostgreSQL no disponible, omitiendo test de integración.");
        return;
    };

    let unique_tag = uuid::Uuid::new_v4().to_string();
    let domain = format!("domain-{unique_tag}");
    let statement = format!("PostgreSQL escala eficientemente con pgvector [{unique_tag}]");

    // 1. Guardar creencia candidata con evidencia inicial
    let mut candidate =
        CandidateKnowledge::new_candidate(&statement, Some(domain.clone()), None).unwrap();
    let ev1 = Evidence::new(
        candidate.id,
        EvidenceSourceType::ToolExecution,
        format!("Benchmark inicial p95 < 8ms [{unique_tag}]"),
        true,
    )
    .unwrap();
    candidate.add_evidence(ev1);

    repo.save_candidate(&candidate)
        .await
        .expect("Debe guardar el candidato en PostgreSQL");

    // 2. Recuperar por ID y verificar integridad
    let found = repo
        .find_candidate_by_id(&candidate.id)
        .await
        .expect("Consulta por ID debe ser exitosa")
        .expect("El candidato debe existir");

    assert_eq!(found.id, candidate.id);
    assert_eq!(found.statement, statement);
    assert_eq!(found.stage, LearningStage::Candidate);
    assert_eq!(found.evidences.len(), 1);
    assert_eq!(
        found.evidences[0].source_type,
        EvidenceSourceType::ToolExecution
    );

    // 3. Añadir segunda evidencia empírica
    let ev2 = Evidence::new(
        candidate.id,
        EvidenceSourceType::Human,
        format!("Aprobado por el arquitecto de datos [{unique_tag}]"),
        true,
    )
    .unwrap();

    repo.add_evidence(&candidate.id, &ev2)
        .await
        .expect("Debe agregar evidencia adicional");

    let evidences = repo
        .get_evidences(&candidate.id)
        .await
        .expect("Debe recuperar evidencias");
    assert_eq!(evidences.len(), 2);

    // 4. Actualizar estado y confianza
    let mut updated = found;
    updated.human_validated = true;
    updated.stage = LearningStage::Validated;
    repo.update_candidate(&updated)
        .await
        .expect("Debe actualizar candidato");

    let found_updated = repo
        .find_candidate_by_id(&candidate.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found_updated.stage, LearningStage::Validated);
    assert!(found_updated.human_validated);

    // 5. Búsqueda por dominio y texto
    let by_domain = repo
        .find_candidates_by_domain(&domain, 10)
        .await
        .expect("Búsqueda por dominio debe funcionar");
    assert_eq!(by_domain.len(), 1);

    let search_res = repo
        .search_candidates(&unique_tag, 10)
        .await
        .expect("Búsqueda por texto debe funcionar");
    assert_eq!(search_res.len(), 1);

    // 6. Eliminar candidato y verificar borrado en cascada
    repo.delete_candidate(&candidate.id)
        .await
        .expect("Debe eliminar candidato");
    let after_delete = repo.find_candidate_by_id(&candidate.id).await.unwrap();
    assert!(after_delete.is_none());

    let evs_after = repo.get_evidences(&candidate.id).await.unwrap();
    assert!(
        evs_after.is_empty(),
        "Evidencias deben eliminarse en cascada"
    );
}
