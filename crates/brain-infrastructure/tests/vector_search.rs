//! Pruebas de integración para generación de embeddings y búsqueda vectorial (SRS §12, §13, §25.2).

use std::time::Duration;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use brain_domain::model::{DomainError, Memory, MemoryContent, MemoryOrigin, Provenance};
use brain_domain::ports::{
    EmbeddingProvider, MemoryRepository, VectorRepository, DEFAULT_EMBEDDING_DIMENSION,
};
use brain_infrastructure::embeddings::{LlamaCppConfig, LlamaCppEmbeddingProvider};
use brain_infrastructure::PostgresMemoryRepository;

#[tokio::test]
async fn llama_cpp_embedding_provider_wiremock_success() {
    let mock_server = MockServer::start().await;

    // Generar un vector simulado de 768 dimensiones
    let dummy_embedding: Vec<f32> = (0..DEFAULT_EMBEDDING_DIMENSION)
        .map(|i| (i as f32) / 1000.0)
        .collect();

    let response_body = serde_json::json!({
        "embedding": dummy_embedding
    });

    Mock::given(method("POST"))
        .and(path("/embedding"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&mock_server)
        .await;

    let endpoint = format!("{}/embedding", mock_server.uri());
    let config = LlamaCppConfig::new(endpoint).with_timeout(Duration::from_secs(2));
    let provider = LlamaCppEmbeddingProvider::new(config).expect("Error creando provider");

    let result = provider
        .embed("Arquitectura Hexagonal en Rust")
        .await
        .expect("Debe generar embedding");

    assert_eq!(result.len(), DEFAULT_EMBEDDING_DIMENSION);
    assert_eq!(result[0], 0.0);
}

#[tokio::test]
async fn llama_cpp_embedding_provider_wiremock_openai_format() {
    let mock_server = MockServer::start().await;

    let dummy_embedding: Vec<f32> = vec![0.05; DEFAULT_EMBEDDING_DIMENSION];
    let response_body = serde_json::json!({
        "data": [
            {
                "embedding": dummy_embedding,
                "index": 0,
                "object": "embedding"
            }
        ]
    });

    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&mock_server)
        .await;

    let endpoint = format!("{}/v1/embeddings", mock_server.uri());
    let config = LlamaCppConfig::new(endpoint);
    let provider = LlamaCppEmbeddingProvider::new(config).unwrap();

    let result = provider
        .embed("Compatibilidad con formato OpenAI")
        .await
        .unwrap();

    assert_eq!(result.len(), DEFAULT_EMBEDDING_DIMENSION);
}

#[tokio::test]
async fn llama_cpp_embedding_provider_dimension_mismatch_error() {
    let mock_server = MockServer::start().await;

    // Devuelve 384 dimensiones cuando se esperan 768
    let wrong_dim_embedding = vec![0.1f32; 384];
    let response_body = serde_json::json!({
        "embedding": wrong_dim_embedding
    });

    Mock::given(method("POST"))
        .and(path("/embedding"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&mock_server)
        .await;

    let endpoint = format!("{}/embedding", mock_server.uri());
    let config = LlamaCppConfig::new(endpoint).with_dimension(DEFAULT_EMBEDDING_DIMENSION);
    let provider = LlamaCppEmbeddingProvider::new(config).unwrap();

    let result = provider.embed("Texto con dimensión errónea").await;
    match result {
        Err(DomainError::RepositoryError(msg)) => {
            assert!(msg.contains("Dimensión inesperada del embedding"));
        }
        other => panic!(
            "Se esperaba error de dimensión inesperada, recibido: {:?}",
            other
        ),
    }
}

#[tokio::test]
async fn llama_cpp_embedding_provider_timeout_or_server_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/embedding"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock_server)
        .await;

    let endpoint = format!("{}/embedding", mock_server.uri());
    let config = LlamaCppConfig::new(endpoint).with_timeout(Duration::from_millis(500));
    let provider = LlamaCppEmbeddingProvider::new(config).unwrap();

    let result = provider.embed("Fallo de servidor").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn postgres_vector_repository_integration() {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://localbrain:localbrain_secret@localhost:5433/local_brain".to_string()
    });

    let pool = match sqlx::PgPool::connect(&db_url).await {
        Ok(p) => p,
        Err(_) => {
            eprintln!(
                "PostgreSQL no disponible en {db_url}, saltando prueba de integración pgvector."
            );
            return;
        }
    };

    let repo = PostgresMemoryRepository::new(pool);
    repo.run_migrations()
        .await
        .expect("Migraciones deben ejecutarse con éxito");

    // Crear memorias con vectores de 768 dimensiones
    let mut vec_a = vec![0.0f32; DEFAULT_EMBEDDING_DIMENSION];
    vec_a[0] = 1.0; // Vector orientado en dimensión 0

    let mut vec_b = vec![0.0f32; DEFAULT_EMBEDDING_DIMENSION];
    vec_b[1] = 1.0; // Vector orientado en dimensión 1

    let mem_a = Memory::new_episodic(
        MemoryContent::new("Uso de Rust para sistemas concurrentes seguros").unwrap(),
        Provenance::new(MemoryOrigin::Observation).with_agent("agent-test"),
        Some("local-brain".to_string()),
    );
    let mut mem_a_with_emb = mem_a.clone();
    mem_a_with_emb.embedding = Some(vec_a.clone());

    let mem_b = Memory::new_episodic(
        MemoryContent::new("Receta para hornear pan casero").unwrap(),
        Provenance::new(MemoryOrigin::Observation).with_agent("agent-test"),
        Some("local-brain".to_string()),
    );
    let mut mem_b_with_emb = mem_b.clone();
    mem_b_with_emb.embedding = Some(vec_b.clone());

    repo.save(&mem_a_with_emb)
        .await
        .expect("Debe guardar mem_a");
    repo.save(&mem_b_with_emb)
        .await
        .expect("Debe guardar mem_b");

    // Buscar con vector similar a vec_a (orientado a dim 0)
    let query_vector = vec_a.clone();
    let similar = repo
        .search_similar(&query_vector, 5)
        .await
        .expect("Búsqueda similar debe ejecutarse");

    assert!(!similar.is_empty());
    // El primer resultado debe ser mem_a con alta similitud (~1.0)
    let (top_id, top_sim) = &similar[0];
    assert_eq!(*top_id, mem_a.id);
    assert!(
        *top_sim > 0.99,
        "La similitud de vec_a con vec_a debe ser ~1.0, recibido: {top_sim}"
    );

    // Probar store_embedding explícito
    let mut vec_c = vec![0.0f32; DEFAULT_EMBEDDING_DIMENSION];
    vec_c[5] = 1.0;
    repo.store_embedding(&mem_b.id, &vec_c)
        .await
        .expect("Debe actualizar embedding");

    // Cleanup
    let _ = repo.soft_delete(&mem_a.id).await;
    let _ = repo.soft_delete(&mem_b.id).await;
}
