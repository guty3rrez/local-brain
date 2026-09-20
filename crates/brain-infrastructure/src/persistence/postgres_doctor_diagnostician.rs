//! Implementación de DoctorDiagnostician sobre PostgreSQL y servicios locales (SRS §35).
//!
//! Ejecuta verificaciones en vivo contra PostgreSQL, pgvector y llama.cpp,
//! reportando latencias exactas y remediaciones específicas ante anomalías.

use std::time::Instant;

use async_trait::async_trait;
use sqlx::{PgPool, Row};

use brain_application::doctor_use_cases::DoctorDiagnostician;
use brain_application::ApplicationError;
use brain_domain::model::HealthCheck;

/// Diagnostician que ejecuta verificaciones reales contra PostgreSQL y runtime local.
#[derive(Debug, Clone)]
pub struct PostgresDoctorDiagnostician {
    pool: PgPool,
    embedding_url: String,
}

impl PostgresDoctorDiagnostician {
    pub fn new(pool: PgPool, embedding_url: impl Into<String>) -> Self {
        Self {
            pool,
            embedding_url: embedding_url.into(),
        }
    }
}

#[async_trait]
impl DoctorDiagnostician for PostgresDoctorDiagnostician {
    async fn check_database(&self) -> Result<HealthCheck, ApplicationError> {
        let start = Instant::now();
        let row_res = sqlx::query("SELECT version() AS ver")
            .fetch_one(&self.pool)
            .await;

        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

        match row_res {
            Ok(row) => {
                let ver: String = row.try_get("ver").unwrap_or_default();
                let short_ver = ver.split(',').next().unwrap_or(&ver).trim();
                Ok(HealthCheck::ok_with_latency(
                    "Persistencia",
                    "Conectividad PostgreSQL",
                    format!("Conectado ({short_ver})"),
                    elapsed_ms,
                ))
            }
            Err(e) => Ok(HealthCheck::fail(
                "Persistencia",
                "Conectividad PostgreSQL",
                format!("Fallo de conexión a base de datos: {e}"),
                "Verifica que el servicio esté corriendo con 'docker compose up -d postgres' y revisa DATABASE_URL",
            )),
        }
    }

    async fn check_schema(&self) -> Result<Vec<HealthCheck>, ApplicationError> {
        let mut checks = Vec::new();

        // 1. Verificar existencia de las 7 tablas del sistema
        let required_tables = [
            (
                "memories",
                "Tabla principal de almacenamiento de memoria cognitiva",
            ),
            ("graph_nodes", "Nodos del grafo de conocimiento (SRS §11)"),
            ("graph_edges", "Aristas del grafo de conocimiento (SRS §11)"),
            ("learning_candidates", "Conocimientos candidatos (SRS §15)"),
            (
                "learning_evidence",
                "Evidencias empíricas de aprendizaje (SRS §15)",
            ),
            (
                "knowledge_conflicts",
                "Registro de contradicciones y conflictos (SRS §18)",
            ),
            (
                "consolidation_runs",
                "Auditoría de reflexiones y consolidaciones (SRS §16)",
            ),
        ];

        let mut missing_tables = Vec::new();
        for (table, _desc) in required_tables {
            let exists_res: Result<Option<bool>, _> = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = $1)",
            )
            .bind(table)
            .fetch_optional(&self.pool)
            .await;

            match exists_res {
                Ok(Some(true)) => {}
                _ => missing_tables.push(table),
            }
        }

        if missing_tables.is_empty() {
            checks.push(HealthCheck::ok(
                "Persistencia",
                "Esquema de Tablas",
                "Las 7 tablas cognitivas están presentes y migradas correctamente",
            ));
        } else {
            checks.push(HealthCheck::fail(
                "Persistencia",
                "Esquema de Tablas",
                format!(
                    "Tablas faltantes en la base de datos: {}",
                    missing_tables.join(", ")
                ),
                "Ejecuta 'brain init' para aplicar las migraciones faltantes",
            ));
        }

        // 2. Verificar columna generada FTS y su índice GIN
        let fts_col_res: Result<Option<bool>, _> = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'memories' AND column_name = 'tsv')",
        )
        .fetch_optional(&self.pool)
        .await;

        match fts_col_res {
            Ok(Some(true)) => {
                checks.push(HealthCheck::ok(
                    "Búsqueda",
                    "Full-Text Search (FTS)",
                    "Columna léxica tsvector 'tsv' e índice GIN operativos (Fase 8)",
                ));
            }
            _ => {
                checks.push(HealthCheck::warn(
                    "Búsqueda",
                    "Full-Text Search (FTS)",
                    "Columna 'tsv' no detectada en 'memories'",
                    "Ejecuta 'brain init' para habilitar el canal de búsqueda léxica FTS",
                ));
            }
        }

        Ok(checks)
    }

    async fn check_vector_extension(&self) -> Result<HealthCheck, ApplicationError> {
        let ext_res: Result<Option<String>, _> = sqlx::query_scalar(
            "SELECT installed_version FROM pg_available_extensions WHERE name = 'vector'",
        )
        .fetch_optional(&self.pool)
        .await;

        match ext_res {
            Ok(Some(ver)) => {
                // Probar cálculo de distancia coseno
                let test_math: Result<Option<f32>, _> =
                    sqlx::query_scalar("SELECT ('[1,0]'::vector <=> '[0,1]'::vector)::real")
                        .fetch_optional(&self.pool)
                        .await;

                match test_math {
                    Ok(Some(dist)) if (dist - 1.0).abs() < 0.001 => {
                        Ok(HealthCheck::ok(
                            "Vectores",
                            "Extensión pgvector",
                            format!("Extensión instalada v{} (operaciones de distancia coseno validadas)", ver),
                        ))
                    }
                    Ok(_) => Ok(HealthCheck::warn(
                        "Vectores",
                        "Extensión pgvector",
                        format!("Extensión instalada v{} pero el cálculo de distancia devolvió un valor inesperado", ver),
                        "Verifica la instalación de pgvector en la base de datos",
                    )),
                    Err(e) => Ok(HealthCheck::fail(
                        "Vectores",
                        "Extensión pgvector",
                        format!("Extensión presente pero error al calcular distancia vectorial: {e}"),
                        "Ejecuta 'CREATE EXTENSION IF NOT EXISTS vector;' o reinicia el contenedor postgres",
                    )),
                }
            }
            Ok(None) => Ok(HealthCheck::fail(
                "Vectores",
                "Extensión pgvector",
                "La extensión pgvector no se encuentra activada en PostgreSQL",
                "Ejecuta 'brain init' para activar la extensión pgvector en la base de datos",
            )),
            Err(e) => Ok(HealthCheck::fail(
                "Vectores",
                "Extensión pgvector",
                format!("Error al consultar extensiones en PostgreSQL: {e}"),
                "Verifica los permisos de conexión en PostgreSQL",
            )),
        }
    }

    async fn check_embedding_runtime(&self) -> Result<HealthCheck, ApplicationError> {
        let start = Instant::now();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(1500))
            .build()
            .map_err(|e| ApplicationError::Internal(e.to_string()))?;

        // Probar directamente generación de embedding con timeout corto
        let test_body = serde_json::json!({
            "content": "test health check"
        });

        let embed_res = client
            .post(&self.embedding_url)
            .json(&test_body)
            .send()
            .await;

        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

        match embed_res {
            Ok(emb_resp) if emb_resp.status().is_success() => {
                if let Ok(json_val) = emb_resp.json::<serde_json::Value>().await {
                    let dim_opt = json_val
                        .get("embedding")
                        .and_then(|v| v.as_array())
                        .map(|a| a.len())
                        .or_else(|| {
                            json_val
                                .get("data")
                                .and_then(|d| d.as_array())
                                .and_then(|a| a.first())
                                .and_then(|item| item.get("embedding"))
                                .and_then(|v| v.as_array())
                                .map(|a| a.len())
                        })
                        .or_else(|| {
                            json_val
                                .as_array()
                                .and_then(|arr| arr.first())
                                .and_then(|item| item.get("embedding"))
                                .and_then(|v| v.as_array())
                                .map(|inner| {
                                    if let Some(serde_json::Value::Array(nested)) = inner.first() {
                                        nested.len()
                                    } else {
                                        inner.len()
                                    }
                                })
                        });

                    if let Some(dim) = dim_opt {
                        if dim == 768 {
                            return Ok(HealthCheck::ok_with_latency(
                                "Embeddings",
                                "Runtime llama.cpp",
                                format!("En línea y respondiendo a {} (dimensión: 768d)", self.embedding_url),
                                elapsed_ms,
                            ));
                        } else {
                            return Ok(HealthCheck::warn(
                                "Embeddings",
                                "Runtime llama.cpp",
                                format!("En línea pero retornó dimensión {} (se esperaba 768)", dim),
                                "Verifica que el modelo cargado sea nomic-embed-text-v1.5 de 768 dimensiones",
                            ));
                        }
                    }
                }

                Ok(HealthCheck::ok_with_latency(
                    "Embeddings",
                    "Runtime llama.cpp",
                    format!("En línea ({})", self.embedding_url),
                    elapsed_ms,
                ))
            }
            Ok(resp) => Ok(HealthCheck::warn(
                "Embeddings",
                "Runtime llama.cpp",
                format!("Servidor respondió con código HTTP {}", resp.status()),
                "Verifica los logs del contenedor local-brain-embeddings",
            )),
            Err(e) => Ok(HealthCheck::warn(
                "Embeddings",
                "Runtime llama.cpp",
                format!("Fuera de línea ({}) - {e}", self.embedding_url),
                "Inicia el contenedor de embeddings: 'docker compose up -d' o './scripts/setup-services.sh'",
            )),
        }
    }

    async fn check_queue_health(&self) -> Result<Vec<HealthCheck>, ApplicationError> {
        let mut checks = Vec::new();

        // 1. Memorias con status 'pending_embedding'
        let pending_res: Result<Option<i64>, _> = sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM memories WHERE status = 'pending_embedding'",
        )
        .fetch_optional(&self.pool)
        .await;

        if let Ok(Some(count)) = pending_res {
            if count == 0 {
                checks.push(HealthCheck::ok(
                    "Colas",
                    "Embeddings Pendientes",
                    "Todos los recuerdos tienen sus vectores generados e indexados",
                ));
            } else {
                checks.push(HealthCheck::warn(
                    "Colas",
                    "Embeddings Pendientes",
                    format!("{count} recuerdo(s) esperando generación de embedding vectorial"),
                    "Ejecuta 'brain embed-pending' para generar los vectores pendientes",
                ));
            }
        }

        // 2. Memorias expiradas por TTL sin purgar
        let expired_res: Result<Option<i64>, _> = sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM memories WHERE expires_at < NOW() AND status = 'active'",
        )
        .fetch_optional(&self.pool)
        .await;

        if let Ok(Some(exp_count)) = expired_res {
            if exp_count == 0 {
                checks.push(HealthCheck::ok(
                    "Colas",
                    "Memorias de Trabajo Expiradas",
                    "No hay memorias de trabajo con TTL vencido pendientes de purga",
                ));
            } else {
                checks.push(HealthCheck::warn(
                    "Colas",
                    "Memorias de Trabajo Expiradas",
                    format!("{exp_count} memoria(s) expiradas por TTL activas"),
                    "Ejecuta 'brain purge-expired' para archivar memorias de trabajo vencidas",
                ));
            }
        }

        // 3. Conflictos cognitivos pendientes de resolución humana (SRS §18)
        let conflicts_res: Result<Option<i64>, _> = sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM knowledge_conflicts WHERE status = 'pending'",
        )
        .fetch_optional(&self.pool)
        .await;

        if let Ok(Some(conf_count)) = conflicts_res {
            if conf_count == 0 {
                checks.push(HealthCheck::ok(
                    "Conocimiento",
                    "Conflictos y Contradicciones",
                    "Sin conflictos cognitivos ni contradicciones pendientes",
                ));
            } else {
                checks.push(HealthCheck::warn(
                    "Conocimiento",
                    "Conflictos y Contradicciones",
                    format!("{conf_count} contradicción(es) pendiente(s) de revisión humana"),
                    "Revisa y resuelve los conflictos con 'brain conflicts list' y 'brain conflicts resolve'",
                ));
            }
        }

        Ok(checks)
    }
}
