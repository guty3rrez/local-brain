//! Casos de Uso para Observabilidad y Diagnóstico de Salud del Sistema (SRS §35).
//!
//! Orquesta la verificación integral de todos los subsistemas (entorno, persistencia,
//! extensión pgvector, runtime de inferencia llama.cpp, colas de trabajo y postura offline).

use std::sync::Arc;

use async_trait::async_trait;

use brain_core::CORE_VERSION;
use brain_domain::model::{DoctorReport, HealthCheck, OfflinePolicy};

use crate::use_cases::ApplicationError;

/// Puerto secundario para que los adaptadores de infraestructura provean verificaciones de salud detalladas.
#[async_trait]
pub trait DoctorDiagnostician: Send + Sync {
    /// Verifica la conectividad y latencia contra el motor de persistencia.
    async fn check_database(&self) -> Result<HealthCheck, ApplicationError>;

    /// Verifica la integridad del esquema relacional y tablas necesarias.
    async fn check_schema(&self) -> Result<Vec<HealthCheck>, ApplicationError>;

    /// Verifica la instalación y funcionamiento de la extensión pgvector.
    async fn check_vector_extension(&self) -> Result<HealthCheck, ApplicationError>;

    /// Verifica el estado y dimensiones del runtime de embeddings (llama.cpp).
    async fn check_embedding_runtime(&self) -> Result<HealthCheck, ApplicationError>;

    /// Verifica el estado de las colas y registros pendientes (embeddings, purga, conflictos).
    async fn check_queue_health(&self) -> Result<Vec<HealthCheck>, ApplicationError>;
}

/// Parámetros de entrada para ejecutar el diagnóstico del doctor.
#[derive(Debug, Clone)]
pub struct DoctorCommand {
    pub database_url: Option<String>,
    pub embedding_url: Option<String>,
    pub in_memory: bool,
    pub offline: bool,
    pub config_path: Option<String>,
}

/// Caso de uso para auditoría y diagnóstico del estado de Local Brain (`brain doctor`).
pub struct DoctorUseCase {
    diagnostician: Option<Arc<dyn DoctorDiagnostician>>,
}

impl DoctorUseCase {
    pub fn new(diagnostician: Option<Arc<dyn DoctorDiagnostician>>) -> Self {
        Self { diagnostician }
    }

    pub async fn execute(&self, cmd: DoctorCommand) -> Result<DoctorReport, ApplicationError> {
        let mut report = DoctorReport::new(CORE_VERSION, std::env::consts::OS);

        // 1. Chequeo de Entorno y Sistema
        report.add_check(HealthCheck::ok(
            "Sistema",
            "Versión de Local Brain",
            format!(
                "Core v{} en plataforma {}",
                CORE_VERSION,
                std::env::consts::OS
            ),
        ));

        // 2. Chequeo de Archivo de Configuración brain.toml
        let config_file = cmd.config_path.as_deref().unwrap_or("brain.toml");
        if std::path::Path::new(config_file).exists() {
            match std::fs::read_to_string(config_file) {
                Ok(content) => match toml::from_str::<toml::Value>(&content) {
                    Ok(_) => {
                        report.add_check(HealthCheck::ok(
                            "Configuración",
                            "Archivo brain.toml",
                            format!("Archivo '{}' cargado y validado correctamente", config_file),
                        ));
                    }
                    Err(e) => {
                        report.add_check(HealthCheck::warn(
                            "Configuración",
                            "Archivo brain.toml",
                            format!("Error de sintaxis en '{}': {}", config_file, e),
                            "Corrige la sintaxis TOML en brain.toml o restaura el archivo predeterminado",
                        ));
                    }
                },
                Err(e) => {
                    report.add_check(HealthCheck::warn(
                        "Configuración",
                        "Archivo brain.toml",
                        format!("No se pudo leer '{}': {}", config_file, e),
                        "Verifica los permisos de lectura de brain.toml",
                    ));
                }
            }
        } else {
            report.add_check(HealthCheck::warn(
                "Configuración",
                "Archivo brain.toml",
                format!(
                    "Archivo '{}' no encontrado (usando valores predeterminados en memoria)",
                    config_file
                ),
                "Crea un archivo brain.toml para personalizar pesos y umbrales de recuperación",
            ));
        }

        // 3. Chequeo de Postura Offline
        let offline_policy = if cmd.offline {
            OfflinePolicy::strict()
        } else {
            OfflinePolicy::permissive()
        };

        if cmd.offline {
            let mut offline_violations = Vec::new();

            if let Some(ref db) = cmd.database_url {
                if let Err(e) = offline_policy.validate_url(db) {
                    offline_violations.push(format!("DATABASE_URL: {e}"));
                }
            }
            if let Some(ref emb) = cmd.embedding_url {
                if let Err(e) = offline_policy.validate_url(emb) {
                    offline_violations.push(format!("EMBEDDING_URL: {e}"));
                }
            }

            if offline_violations.is_empty() {
                report.add_check(HealthCheck::ok(
                    "Seguridad",
                    "Modo Offline Estricto",
                    "Todos los endpoints configurados cumplen con bucle invertido local (localhost/127.0.0.1)",
                ));
            } else {
                report.add_check(HealthCheck::fail(
                    "Seguridad",
                    "Modo Offline Estricto",
                    format!(
                        "Violación de política offline: {}",
                        offline_violations.join(" | ")
                    ),
                    "Configura URLs locales (localhost o 127.0.0.1) o desactiva el modo offline",
                ));
            }
        } else {
            report.add_check(HealthCheck::ok(
                "Seguridad",
                "Modo Offline",
                "Modo offline estándar activo (prioriza local-first sin bloqueo forzado)",
            ));
        }

        // 4. Chequeos dependientes de infraestructura
        if cmd.in_memory {
            report.add_check(HealthCheck::ok(
                "Persistencia",
                "Almacenamiento en Memoria",
                "Operando en modo volátil efímero (--in-memory) sin persistencia en disco",
            ));
            report.add_check(HealthCheck::ok(
                "Persistencia",
                "Esquema y Tablas",
                "Estructuras de datos en memoria inicializadas correctamente",
            ));
            report.add_check(HealthCheck::ok(
                "Vectores",
                "Extensión pgvector",
                "Motor vectorial en memoria con similitud coseno pura activo (768 dimensiones)",
            ));
            report.add_check(HealthCheck::ok(
                "Embeddings",
                "Runtime de Embeddings",
                "Generador de embeddings en memoria / diferido activo",
            ));
            report.add_check(HealthCheck::ok(
                "Colas",
                "Estado de Procesamiento",
                "0 tareas pendientes en modo memoria",
            ));
        } else if let Some(ref diag) = self.diagnostician {
            // Base de datos
            match diag.check_database().await {
                Ok(chk) => report.add_check(chk),
                Err(e) => report.add_check(HealthCheck::fail(
                    "Persistencia",
                    "Conectividad PostgreSQL",
                    format!("Error de conexión: {e}"),
                    "Verifica que el servicio PostgreSQL esté activo: 'docker compose up -d postgres'",
                )),
            }

            // Esquema y tablas
            match diag.check_schema().await {
                Ok(checks) => {
                    for chk in checks {
                        report.add_check(chk);
                    }
                }
                Err(e) => report.add_check(HealthCheck::fail(
                    "Persistencia",
                    "Esquema de Tablas",
                    format!("Error al verificar tablas: {e}"),
                    "Ejecuta 'brain init' para aplicar las migraciones de base de datos",
                )),
            }

            // Extensión pgvector
            match diag.check_vector_extension().await {
                Ok(chk) => report.add_check(chk),
                Err(e) => report.add_check(HealthCheck::fail(
                    "Vectores",
                    "Extensión pgvector",
                    format!("Error al verificar extensión: {e}"),
                    "Asegúrate de que la imagen docker tenga pgvector y ejecuta 'brain init'",
                )),
            }

            // Runtime llama.cpp
            match diag.check_embedding_runtime().await {
                Ok(chk) => report.add_check(chk),
                Err(e) => report.add_check(HealthCheck::warn(
                    "Embeddings",
                    "Runtime llama.cpp",
                    format!("Runtime de embeddings no disponible: {e}"),
                    "Levanta el contenedor de embeddings: 'docker compose up -d llama-embed' o './scripts/setup-services.sh'",
                )),
            }

            // Colas y trabajos pendientes
            match diag.check_queue_health().await {
                Ok(checks) => {
                    for chk in checks {
                        report.add_check(chk);
                    }
                }
                Err(e) => report.add_check(HealthCheck::warn(
                    "Colas",
                    "Estado de Procesamiento",
                    format!("No se pudieron consultar colas de trabajo: {e}"),
                    "Verifica la conexión a la base de datos",
                )),
            }
        }

        Ok(report)
    }
}

/// Diagnóstico simulado en memoria para pruebas unitarias.
#[derive(Debug, Default)]
pub struct MockDoctorDiagnostician {
    pub db_ok: bool,
    pub pgvector_ok: bool,
    pub llm_ok: bool,
}

#[async_trait]
impl DoctorDiagnostician for MockDoctorDiagnostician {
    async fn check_database(&self) -> Result<HealthCheck, ApplicationError> {
        if self.db_ok {
            Ok(HealthCheck::ok_with_latency(
                "Persistencia",
                "Conexión PostgreSQL",
                "Operativo",
                1.2,
            ))
        } else {
            Ok(HealthCheck::fail(
                "Persistencia",
                "Conexión PostgreSQL",
                "Caído",
                "docker compose up -d postgres",
            ))
        }
    }

    async fn check_schema(&self) -> Result<Vec<HealthCheck>, ApplicationError> {
        Ok(vec![HealthCheck::ok(
            "Persistencia",
            "Esquema de Tablas",
            "7 tablas verificadas",
        )])
    }

    async fn check_vector_extension(&self) -> Result<HealthCheck, ApplicationError> {
        if self.pgvector_ok {
            Ok(HealthCheck::ok(
                "Vectores",
                "Extensión pgvector",
                "Instalada v0.8.0",
            ))
        } else {
            Ok(HealthCheck::fail(
                "Vectores",
                "Extensión pgvector",
                "No instalada",
                "brain init",
            ))
        }
    }

    async fn check_embedding_runtime(&self) -> Result<HealthCheck, ApplicationError> {
        if self.llm_ok {
            Ok(HealthCheck::ok_with_latency(
                "Embeddings",
                "Runtime llama.cpp",
                "En línea 768d",
                5.0,
            ))
        } else {
            Ok(HealthCheck::warn(
                "Embeddings",
                "Runtime llama.cpp",
                "Fuera de línea",
                "docker compose up -d llama-embed",
            ))
        }
    }

    async fn check_queue_health(&self) -> Result<Vec<HealthCheck>, ApplicationError> {
        Ok(vec![HealthCheck::ok(
            "Colas",
            "Embeddings Pendientes",
            "0 recuerdos pendientes",
        )])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brain_domain::model::CheckStatus;

    #[tokio::test]
    async fn test_doctor_in_memory_execution() {
        let use_case = DoctorUseCase::new(None);
        let cmd = DoctorCommand {
            database_url: None,
            embedding_url: None,
            in_memory: true,
            offline: true,
            config_path: None,
        };

        let report = use_case.execute(cmd).await.expect("Diagnóstico exitoso");
        assert!(report.is_healthy());
        assert_eq!(report.fail_count(), 0);
        assert!(report.ok_count() >= 4);
    }

    #[tokio::test]
    async fn test_doctor_offline_violation_detection() {
        let use_case = DoctorUseCase::new(None);
        let cmd = DoctorCommand {
            database_url: Some("postgres://user:pass@db.external.com:5432/brain".to_string()),
            embedding_url: Some("https://api.openai.com/v1/embeddings".to_string()),
            in_memory: false,
            offline: true,
            config_path: None,
        };

        let report = use_case.execute(cmd).await.expect("Diagnóstico exitoso");
        assert!(!report.is_healthy());
        assert!(report.fail_count() >= 1);
        assert!(report
            .checks
            .iter()
            .any(|c| c.name == "Modo Offline Estricto" && c.status == CheckStatus::Fail));
    }

    #[tokio::test]
    async fn test_doctor_with_mock_diagnostician() {
        let mock = Arc::new(MockDoctorDiagnostician {
            db_ok: true,
            pgvector_ok: true,
            llm_ok: false,
        });

        let use_case = DoctorUseCase::new(Some(mock));
        let cmd = DoctorCommand {
            database_url: Some(
                "postgres://localbrain:secret@localhost:5433/local_brain".to_string(),
            ),
            embedding_url: Some("http://127.0.0.1:8081/embedding".to_string()),
            in_memory: false,
            offline: true,
            config_path: None,
        };

        let report = use_case.execute(cmd).await.expect("Diagnóstico exitoso");
        assert!(report.is_healthy()); // Advertencias no son fallos críticos
        assert_eq!(report.warn_count(), 2); // brain.toml (si no en ruta de test) + llama.cpp
        assert_eq!(report.fail_count(), 0);
    }
}
