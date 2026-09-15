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
        .stdout(predicate::str::contains("status"));

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
