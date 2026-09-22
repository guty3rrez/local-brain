//! Pruebas de integración de caja negra para la CLI de Local Brain (`brain`).
//!
//! Valida el comportamiento de los subcomandos `init`, `remember`, `recall` y `status` (SRS §25).

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn cli_help_and_version() {
    let mut cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Local Brain"))
        .stdout(predicate::str::contains("remember"))
        .stdout(predicate::str::contains("recall"))
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("relate"))
        .stdout(predicate::str::contains("graph"))
        .stdout(predicate::str::contains("learn"))
        .stdout(predicate::str::contains("explain"))
        .stdout(predicate::str::contains("reflect"))
        .stdout(predicate::str::contains("conflicts"));

    let mut ver_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    ver_cmd
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("0.1.0"));
}

#[test]
fn cli_status_command() {
    let mut cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    cmd.arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("Estado del Sistema"))
        .stdout(predicate::str::contains("Hexagonal"));
}

#[test]
fn cli_status_in_memory() {
    let mut cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    cmd.arg("--in-memory")
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("Modo volátil en memoria"));
}

#[test]
fn cli_init_command() {
    let mut cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    cmd.arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Base de datos inicializada exitosamente",
        ))
        .stdout(predicate::str::contains("Migraciones"));
}

#[test]
fn cli_remember_and_recall_integration() {
    let tag = uuid::Uuid::new_v4().to_string();
    let project = format!("cli-suite-{tag}");
    let content = format!("Contenido de prueba CLI con persistencia PostgreSQL [{tag}]");

    // 1. Remember
    let mut rem_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    rem_cmd
        .arg("remember")
        .arg(&content)
        .arg("--project")
        .arg(&project)
        .arg("--importance")
        .arg("0.85")
        .arg("--confidence")
        .arg("0.90")
        .assert()
        .success()
        .stdout(predicate::str::contains("Recuerdo almacenado con éxito"))
        .stdout(predicate::str::contains(&project))
        .stdout(predicate::str::contains("0.85"));

    // 2. Recall por proyecto
    let mut rec_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    rec_cmd
        .arg("recall")
        .arg("--project")
        .arg(&project)
        .assert()
        .success()
        .stdout(predicate::str::contains("Encontrados 1 recuerdo(s)"))
        .stdout(predicate::str::contains(&content))
        .stdout(predicate::str::contains(&project));
}

#[test]
fn cli_validation_invalid_importance() {
    let mut cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    cmd.arg("remember")
        .arg("Contenido con importancia fuera de rango")
        .arg("--importance")
        .arg("1.5")
        .assert()
        .failure();
}

#[test]
fn cli_validation_invalid_uuid() {
    let mut cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    cmd.arg("recall")
        .arg("--id")
        .arg("not-a-valid-uuid")
        .assert()
        .failure()
        .stderr(predicate::str::contains("ID de memoria inválido"));
}

#[test]
fn cli_embed_pending_command() {
    let mut cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    cmd.arg("--in-memory")
        .arg("embed-pending")
        .assert()
        .success()
        .stdout(predicate::str::contains("Procesando recuerdos pendientes"));
}

#[tokio::test]
async fn cli_semantic_recall_with_mock_provider() {
    let mock_server = wiremock::MockServer::start().await;
    let dummy_embedding: Vec<f32> = (0..768).map(|i| (i as f32) / 1000.0).collect();
    let response_body = serde_json::json!({
        "embedding": dummy_embedding
    });

    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/embedding"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&mock_server)
        .await;

    let emb_url = format!("{}/embedding", mock_server.uri());
    let tag = uuid::Uuid::new_v4().to_string();
    let project = format!("cli-sem-{tag}");
    let content = format!("Patrón de diseño Hexagonal y desacople cognitivo [{tag}]");

    // 1. Guardar memoria con provider mock
    let mut rem_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    rem_cmd
        .arg("--embedding-url")
        .arg(&emb_url)
        .arg("remember")
        .arg(&content)
        .arg("--project")
        .arg(&project)
        .assert()
        .success()
        .stdout(predicate::str::contains("vector 768 dims indexado"));

    // 2. Buscar semánticamente por query con provider mock
    let mut rec_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    rec_cmd
        .arg("--embedding-url")
        .arg(&emb_url)
        .arg("recall")
        .arg("-q")
        .arg("Hexagonal desacople")
        .arg("--project")
        .arg(&project)
        .assert()
        .success()
        .stdout(predicate::str::contains("Encontrados 1 recuerdo(s)"))
        .stdout(predicate::str::contains(&content));
}

#[tokio::test]
async fn cli_retrieve_json_format() {
    let mock_server = wiremock::MockServer::start().await;
    let dummy_embedding: Vec<f32> = (0..768).map(|i| (i as f32) / 1000.0).collect();
    let response_body = serde_json::json!({
        "embedding": dummy_embedding
    });

    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/embedding"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&mock_server)
        .await;

    let emb_url = format!("{}/embedding", mock_server.uri());
    let tag = uuid::Uuid::new_v4().to_string();
    let project = format!("cli-retrieve-json-{tag}");
    let content = format!("Recuperación híbrida combina vector, FTS y grafo [{tag}]");

    let mut rem_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    rem_cmd
        .arg("--embedding-url")
        .arg(&emb_url)
        .arg("remember")
        .arg(&content)
        .arg("--project")
        .arg(&project)
        .assert()
        .success();

    let mut retrieve_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    let output = retrieve_cmd
        .arg("--embedding-url")
        .arg(&emb_url)
        .arg("retrieve")
        .arg("--json")
        .arg("--project")
        .arg(&project)
        .arg("recuperación híbrida vector FTS grafo")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"items\":"))
        .stdout(predicate::str::contains("\"metrics\":"))
        .stdout(predicate::str::contains("\"assembled_context\":"))
        .stdout(predicate::str::contains("<untrusted_memory_context>"))
        .get_output()
        .stdout
        .clone();

    // El JSON debe ser válido y machine-parseable (consumido por scripts de
    // hooks vía `jq`, no solo por humanos leyendo texto).
    let parsed: serde_json::Value =
        serde_json::from_slice(&output).expect("salida de --json debe ser JSON válido");
    assert!(parsed["items"].is_array());
    assert!(parsed["assembled_context"]
        .as_str()
        .unwrap()
        .contains(&content));
}

#[test]
fn cli_mcp_help_command() {
    let mut cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    cmd.arg("mcp")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Model Context Protocol (MCP)"))
        .stdout(predicate::str::contains("--read-only"))
        .stdout(predicate::str::contains("--allow-delete"));
}

#[test]
fn cli_mcp_stdio_handshake() {
    let mut cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    let init_json = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\"}\n";

    cmd.arg("--in-memory")
        .arg("mcp")
        .write_stdin(init_json)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"protocolVersion\":\"2024-11-05\"",
        ))
        .stdout(predicate::str::contains("\"name\":\"local-brain\""));
}

#[test]
fn cli_specialized_episodic_memory() {
    let tag = uuid::Uuid::new_v4().to_string();
    let project = format!("cli-episodic-{tag}");

    let mut rem_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    rem_cmd
        .arg("remember")
        .arg("--project")
        .arg(&project)
        .arg("--context")
        .arg("Refactorización de base de datos")
        .arg("--action")
        .arg("Ejecutar migración 0005")
        .arg("--outcome")
        .arg("Columnas especializadas agregadas")
        .assert()
        .success()
        .stdout(predicate::str::contains("Recuerdo almacenado con éxito"))
        .stdout(predicate::str::contains("Episodic"));

    let mut rec_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    rec_cmd
        .arg("recall")
        .arg("--project")
        .arg(&project)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Contexto: Refactorización de base de datos",
        ))
        .stdout(predicate::str::contains("Acción: Ejecutar migración 0005"));
}

#[test]
fn cli_working_memory_session_lifecycle() {
    let session_id = format!("ses-cli-{}", uuid::Uuid::new_v4());

    // 1. Guardar working memory con session-id y ttl
    let mut rem_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    rem_cmd
        .arg("remember")
        .arg("Contexto volátil de depuración")
        .arg("--type")
        .arg("working")
        .arg("--session-id")
        .arg(&session_id)
        .arg("--ttl")
        .arg("3600")
        .assert()
        .success()
        .stdout(predicate::str::contains("Recuerdo almacenado con éxito"))
        .stdout(predicate::str::contains("Working"));

    // 2. Recall por session-id
    let mut rec_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    rec_cmd
        .arg("recall")
        .arg("--session-id")
        .arg(&session_id)
        .assert()
        .success()
        .stdout(predicate::str::contains("Encontrados 1 recuerdo(s)"))
        .stdout(predicate::str::contains("Contexto volátil de depuración"));

    // 3. Finalizar sesión
    let mut end_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    end_cmd
        .arg("session-end")
        .arg(&session_id)
        .assert()
        .success()
        .stdout(predicate::str::contains("finalizada exitosamente"))
        .stdout(predicate::str::contains(
            "Recuerdos de trabajo archivados/expirados: 1",
        ));

    // 4. Recall después de finalizar sesión: ya no debe retornar resultados
    let mut rec_after_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    rec_after_cmd
        .arg("recall")
        .arg("--session-id")
        .arg(&session_id)
        .assert()
        .success()
        .stdout(predicate::str::contains("No se encontraron recuerdos"));
}

#[test]
fn cli_associative_memory_creation() {
    let tag = uuid::Uuid::new_v4().to_string();
    let project = format!("cli-assoc-{tag}");

    let mut rem_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    rem_cmd
        .arg("remember")
        .arg("--project")
        .arg(&project)
        .arg("--source-concept")
        .arg("Rust")
        .arg("--target-concept")
        .arg("PostgreSQL")
        .arg("--predicate")
        .arg("persists_to")
        .arg("--strength")
        .arg("0.95")
        .assert()
        .success()
        .stdout(predicate::str::contains("Recuerdo almacenado con éxito"))
        .stdout(predicate::str::contains("Associative"));

    let mut rec_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    rec_cmd
        .arg("recall")
        .arg("--concept")
        .arg("Rust")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Rust --[persists_to]--> PostgreSQL",
        ));
}

#[test]
fn cli_purge_expired_command() {
    let mut cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    cmd.arg("--in-memory")
        .arg("purge-expired")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Purga de recuerdos expirados completada exitosamente",
        ));
}

#[test]
fn cli_relate_and_graph_in_memory() {
    let tag = uuid::Uuid::new_v4().to_string();
    let src = format!("concept-cli-a-{tag}");
    let tgt = format!("concept-cli-b-{tag}");

    // 1. Relate en modo efímero
    let mut rel_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    rel_cmd
        .arg("--in-memory")
        .arg("relate")
        .arg(&src)
        .arg(&tgt)
        .arg("--type")
        .arg("used-in")
        .arg("--weight")
        .arg("0.85")
        .arg("--context")
        .arg("Arquitectura modular")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Relación establecida exitosamente en el Knowledge Graph",
        ))
        .stdout(predicate::str::contains("USED_IN"))
        .stdout(predicate::str::contains("0.85"));
}

#[test]
fn cli_relate_and_graph_postgres_integration() {
    let tag = uuid::Uuid::new_v4().to_string();
    let src = format!("hexagonal-{tag}");
    let tgt = format!("ports-and-adapters-{tag}");

    // 1. Relate en PostgreSQL
    let mut rel_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    rel_cmd
        .arg("relate")
        .arg(&src)
        .arg(&tgt)
        .arg("--type")
        .arg("related-to")
        .arg("--weight")
        .arg("0.95")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Relación establecida exitosamente en el Knowledge Graph",
        ))
        .stdout(predicate::str::contains("RELATED_TO"));

    // 2. Graph exploration en texto legible
    let mut graph_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    graph_cmd
        .arg("graph")
        .arg(&src)
        .arg("--depth")
        .arg("2")
        .assert()
        .success()
        .stdout(predicate::str::contains("Knowledge Graph — Subgrafo desde"))
        .stdout(predicate::str::contains(&src))
        .stdout(predicate::str::contains(&tgt));

    // 3. Graph exploration en formato JSON
    let mut json_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    json_cmd
        .arg("graph")
        .arg(&src)
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"root\":"))
        .stdout(predicate::str::contains("\"nodes\":"))
        .stdout(predicate::str::contains("\"edges\":"));
}

#[test]
fn cli_relate_self_loop_rejection() {
    let mut cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    cmd.arg("--in-memory")
        .arg("relate")
        .arg("SelfConcept")
        .arg("SelfConcept")
        .assert()
        .failure()
        .stderr(predicate::str::contains("auto-bucle"));
}

#[test]
fn cli_relate_dag_cycle_rejection() {
    let tag = uuid::Uuid::new_v4().to_string();
    let n1 = format!("dag-a-{tag}");
    let n2 = format!("dag-b-{tag}");

    // 1. n1 SUPERSEDES n2
    let mut cmd1 = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    cmd1.arg("relate")
        .arg(&n1)
        .arg(&n2)
        .arg("--type")
        .arg("supersedes")
        .assert()
        .success();

    // 2. n2 SUPERSEDES n1 (debe fallar por ciclo en DAG)
    let mut cmd2 = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    cmd2.arg("relate")
        .arg(&n2)
        .arg(&n1)
        .arg("--type")
        .arg("supersedes")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Ciclo"));
}

#[test]
fn cli_learn_in_memory() {
    let mut cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    cmd.arg("--in-memory")
        .arg("learn")
        .arg("Rust previene memory leaks y data races")
        .arg("--evidence")
        .arg("El compilador y borrow checker garantizan thread safety")
        .arg("--source-type")
        .arg("direct-observation")
        .arg("--domain")
        .arg("rust-safety")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Aprendizaje registrado exitosamente en Local Brain",
        ))
        .stdout(predicate::str::contains("rust-safety"))
        .stdout(predicate::str::contains("CANDIDATE"));
}

#[test]
fn cli_learn_and_explain_integration() {
    let tag = uuid::Uuid::new_v4().to_string();
    let statement = format!("PostgreSQL soporta búsqueda semántica eficiente con pgvector [{tag}]");
    let evidence_1 =
        format!("Se realizaron benchmarks HNSW con indexación de 768 dimensiones [{tag}]");
    let evidence_2 = format!("La extensión pgvector está activa y verificada en CI [{tag}]");

    // 1. Primer aprendizaje
    let mut learn_cmd1 = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    learn_cmd1
        .arg("learn")
        .arg(&statement)
        .arg("--evidence")
        .arg(&evidence_1)
        .arg("--source-type")
        .arg("tool-execution")
        .arg("--domain")
        .arg("database")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Aprendizaje registrado exitosamente en Local Brain",
        ))
        .stdout(predicate::str::contains(&statement))
        .stdout(predicate::str::contains("Evidencias:          1"));

    // 2. Agregar evidencia adicional que refuerza la creencia
    let mut learn_cmd2 = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    learn_cmd2
        .arg("learn")
        .arg(&statement)
        .arg("--evidence")
        .arg(&evidence_2)
        .arg("--source-type")
        .arg("direct-observation")
        .arg("--domain")
        .arg("database")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Aprendizaje registrado exitosamente en Local Brain",
        ))
        .stdout(predicate::str::contains("Evidencias:          2"));

    // 3. Explicar epistémicamente el conocimiento
    let mut explain_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    explain_cmd
        .arg("explain")
        .arg(&statement)
        .assert()
        .success()
        .stdout(predicate::str::contains("Explicabilidad Cognitiva"))
        .stdout(predicate::str::contains(&statement))
        .stdout(predicate::str::contains("Confianza:"))
        .stdout(predicate::str::contains("Evidencia:"));
}

#[test]
fn cli_explain_json_format() {
    let tag = uuid::Uuid::new_v4().to_string();
    let statement = format!("Arquitectura Hexagonal desacopla el dominio de IO [{tag}]");
    let evidence = format!("Las entidades puras en brain-domain no importan SQLx ni HTTP [{tag}]");

    let mut learn_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    learn_cmd
        .arg("learn")
        .arg(&statement)
        .arg("--evidence")
        .arg(&evidence)
        .arg("--source-type")
        .arg("tool-execution")
        .arg("--domain")
        .arg("architecture")
        .assert()
        .success();

    let mut explain_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    explain_cmd
        .arg("explain")
        .arg(&statement)
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"conclusion\":"))
        .stdout(predicate::str::contains(&statement))
        .stdout(predicate::str::contains("\"confidence\":"))
        .stdout(predicate::str::contains("\"evidences\":"));
}

#[test]
fn cli_learn_contradiction_flow() {
    let tag = uuid::Uuid::new_v4().to_string();
    let statement = format!("Todo microservicio debe usar gRPC [{tag}]");
    let evidence_sup = format!("gRPC ofrece alto rendimiento binario con protobuf [{tag}]");
    let evidence_contra = format!("Para clientes web públicos REST es más accesible [{tag}]");

    // Registro inicial a favor
    let mut learn_cmd1 = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    learn_cmd1
        .arg("learn")
        .arg(&statement)
        .arg("--evidence")
        .arg(&evidence_sup)
        .arg("--source-type")
        .arg("direct-observation")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Aprendizaje registrado exitosamente en Local Brain",
        ))
        .stdout(predicate::str::contains("Evidencias:          1"));

    // Registro de evidencia contradictoria
    let mut learn_cmd2 = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    learn_cmd2
        .arg("learn")
        .arg(&statement)
        .arg("--evidence")
        .arg(&evidence_contra)
        .arg("--source-type")
        .arg("tool-execution")
        .arg("--refutes")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Aprendizaje registrado exitosamente en Local Brain",
        ))
        .stdout(predicate::str::contains("Evidencias:          2"));

    // Explicación debe reflejar la contradicción
    let mut explain_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    explain_cmd
        .arg("explain")
        .arg(&statement)
        .assert()
        .success()
        .stdout(predicate::str::contains(&statement))
        .stdout(predicate::str::contains("[-] REFUTA"));
}

#[test]
fn cli_reflect_command() {
    let tag = uuid::Uuid::new_v4().to_string();
    let project = format!("reflect-suite-{tag}");

    // Insertar 2 recuerdos para que el cluster se forme
    for i in 1..=2 {
        let mut rem_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
        rem_cmd
            .arg("remember")
            .arg(format!(
                "Observación sobre optimización de queries SQL número {i} [{tag}]"
            ))
            .arg("--project")
            .arg(&project)
            .assert()
            .success();
    }

    let mut ref_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    ref_cmd
        .arg("reflect")
        .arg("--project")
        .arg(&project)
        .arg("--limit")
        .arg("10")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Sesión de Reflexión y Consolidación",
        ))
        .stdout(predicate::str::contains("Memorias analizadas:  2"))
        .stdout(predicate::str::contains("Clusters formados:    1"));
}

#[test]
fn cli_conflicts_command() {
    let mut list_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    list_cmd
        .arg("conflicts")
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Contradicciones Pendientes"));
}

#[test]
fn cli_offline_status_command() {
    let mut cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    cmd.arg("offline")
        .assert()
        .success()
        .stdout(predicate::str::contains("Modo Offline Estricto"))
        .stdout(predicate::str::contains("Auditoría de Aislamiento de Red"))
        .stdout(predicate::str::contains("Bucle invertido verificado"))
        .stdout(predicate::str::contains("Postura Offline: CUMPLIDA"));
}

#[test]
fn cli_offline_mode_blocks_external_url() {
    let mut cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    cmd.arg("--offline")
        .arg("--database-url")
        .arg("postgres://user:pass@db.external.com:5432/brain")
        .arg("status")
        .assert()
        .failure();
}

#[test]
fn cli_doctor_command_healthy() {
    let mut cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    cmd.arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Local Brain Doctor"))
        .stdout(predicate::str::contains("Versión de Local Brain"))
        .stdout(predicate::str::contains("Conectividad PostgreSQL"))
        .stdout(predicate::str::contains("Esquema de Tablas"))
        .stdout(predicate::str::contains("Extensión pgvector"));
}

#[test]
fn cli_doctor_command_in_memory() {
    let mut cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    cmd.arg("--in-memory")
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Local Brain Doctor"))
        .stdout(predicate::str::contains("Almacenamiento en Memoria"))
        .stdout(predicate::str::contains("SALUDABLE Y OPERATIVO"));
}

#[test]
fn cli_doctor_command_json() {
    let mut cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    cmd.arg("doctor")
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"core_version\":"))
        .stdout(predicate::str::contains("\"checks\":"))
        .stdout(predicate::str::contains("\"category\": \"Persistencia\""));
}

#[test]
fn cli_backup_and_restore_roundtrip() {
    let tag = uuid::Uuid::new_v4().to_string();
    let project = format!("backup-test-{tag}");
    let content = format!("Recuerdo de prueba para backup y restore [{tag}]");
    let backup_file = format!("/tmp/test_backup_{tag}.jsonl");

    // 1. Crear memoria
    let mut rem_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    rem_cmd
        .arg("remember")
        .arg(&content)
        .arg("--project")
        .arg(&project)
        .assert()
        .success();

    // 2. Exportar backup
    let mut bak_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    bak_cmd
        .arg("backup")
        .arg("-o")
        .arg(&backup_file)
        .arg("--project")
        .arg(&project)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Respaldo de Local Brain completado con éxito",
        ))
        .stdout(predicate::str::contains("Total Memorias:       1"));

    // 3. Simulación de Restore (--dry-run)
    let mut dry_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    dry_cmd
        .arg("restore")
        .arg("-i")
        .arg(&backup_file)
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Simulación de Restauración (--dry-run)",
        ))
        .stdout(predicate::str::contains(
            "Checksum SHA-256:     🟢 Verificado y válido",
        ))
        .stdout(predicate::str::contains("Memorias a importar:  1"));

    // 4. Restauración real
    let mut res_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    res_cmd
        .arg("restore")
        .arg("-i")
        .arg(&backup_file)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Restauración de Local Brain completada exitosamente",
        ))
        .stdout(predicate::str::contains("Memorias importadas:  1"));

    // 5. Verificar que se pueda recuperar con recall
    let mut rec_cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    rec_cmd
        .arg("recall")
        .arg("--project")
        .arg(&project)
        .assert()
        .success()
        .stdout(predicate::str::contains(&content));

    // Limpieza
    let _ = std::fs::remove_file(&backup_file);
}

#[test]
fn cli_restore_wipe_requires_confirm() {
    let dummy_file = "/tmp/dummy_restore.jsonl";
    let _ = std::fs::write(dummy_file, "{}");

    let mut cmd = Command::cargo_bin("brain").expect("Binario 'brain' disponible");
    cmd.arg("restore")
        .arg("-i")
        .arg(dummy_file)
        .arg("--wipe")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "debes confirmar con el flag '--confirm'",
        ));

    let _ = std::fs::remove_file(dummy_file);
}
