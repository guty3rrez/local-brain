//! # Local Brain CLI (`brain`)
//!
//! Interfaz de línea de comandos para gestión, inicialización, consulta y búsqueda semántica
//! de memoria cognitiva local (SRS §22, §25, §36).

use std::sync::Arc;
use std::time::Duration;

use clap::{Args, Parser, Subcommand, ValueEnum};
use sqlx::Row;

use brain_application::{
    EmbedPendingUseCase, RecallQuery, RecallUseCase, RememberCommand, RememberUseCase,
};
use brain_core::CORE_VERSION;
use brain_domain::model::{DomainError, MemoryId, MemoryStatus, MemoryType};
use brain_domain::ports::{
    EmbeddingProvider, InMemoryEmbeddingProvider, InMemoryMemoryRepository,
    InMemoryVectorRepository, MemoryRepository, VectorRepository, DEFAULT_EMBEDDING_DIMENSION,
};
use brain_infrastructure::embeddings::{LlamaCppConfig, LlamaCppEmbeddingProvider};
use brain_infrastructure::PostgresMemoryRepository;

#[derive(Parser, Debug)]
#[command(
    name = "brain",
    version = CORE_VERSION,
    about = "🧠 Local Brain — Memoria persistente local, agéntica y orientada a conocimiento",
    long_about = "Local Brain proporciona persistencia de memoria cognitiva local-first con tipos especializados (episódica, semántica, procedimental, etc.) y búsqueda vectorial de alta fidelidad (768 dimensiones) para desarrolladores y agentes de IA."
)]
struct Cli {
    /// URL de conexión a PostgreSQL (sobrescribe la variable de entorno DATABASE_URL)
    #[arg(long, global = true)]
    database_url: Option<String>,

    /// URL del endpoint de embeddings de llama.cpp (sobrescribe EMBEDDING_URL)
    #[arg(long, global = true)]
    embedding_url: Option<String>,

    /// Ejecutar en modo efímero en memoria (sin persistencia en PostgreSQL ni red)
    #[arg(long, global = true)]
    in_memory: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Inicializa la persistencia y ejecuta las migraciones de Local Brain en PostgreSQL (RF-001)
    Init,

    /// Registra un nuevo recuerdo en el cerebro local (RF-002, SRS §12.3)
    Remember(RememberArgs),

    /// Recupera recuerdos almacenados por ID, proyecto, tipo o búsqueda semántica vectorial (RF-003, SRS §13)
    Recall(RecallArgs),

    /// Procesa recuerdos en estado 'pending_embedding' cuando el runtime de embeddings está disponible (SRS §12.3)
    EmbedPending(EmbedPendingArgs),

    /// Muestra el estado del sistema, diagnóstico de persistencia, pgvector y llama.cpp
    Status,
}

#[derive(Args, Debug)]
struct RememberArgs {
    /// Contenido textual del recuerdo
    content: String,

    /// Proyecto asociado
    #[arg(short, long)]
    project: Option<String>,

    /// Agente que generó el recuerdo
    #[arg(short, long)]
    agent: Option<String>,

    /// Tipo de memoria cognitiva
    #[arg(short = 't', long = "type", value_enum, default_value_t = CliMemoryType::Episodic)]
    memory_type: CliMemoryType,

    /// Grado de importancia intrínseca (0.0 a 1.0)
    #[arg(short, long)]
    importance: Option<f32>,

    /// Nivel de confianza o evidencia empírica (0.0 a 1.0)
    #[arg(short, long)]
    confidence: Option<f32>,
}

#[derive(Args, Debug)]
struct RecallArgs {
    /// Búsqueda semántica por significado textual utilizando embeddings densos (768 dim)
    #[arg(short = 'q', long)]
    query: Option<String>,

    /// Identificador UUID de un recuerdo específico
    #[arg(long)]
    id: Option<String>,

    /// Filtrar por proyecto
    #[arg(short, long)]
    project: Option<String>,

    /// Filtrar por tipo de memoria
    #[arg(short = 't', long = "type", value_enum)]
    memory_type: Option<CliMemoryType>,

    /// Similitud mínima coseno requerida en búsqueda semántica (0.0 a 1.0)
    #[arg(long)]
    min_similarity: Option<f32>,

    /// Límite máximo de resultados
    #[arg(short = 'n', long, default_value_t = 5)]
    limit: usize,
}

#[derive(Args, Debug)]
struct EmbedPendingArgs {
    /// Límite máximo de recuerdos pendientes a procesar
    #[arg(short = 'n', long, default_value_t = 50)]
    limit: usize,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum CliMemoryType {
    Working,
    Episodic,
    Semantic,
    Procedural,
    Associative,
}

impl From<CliMemoryType> for MemoryType {
    fn from(cli_type: CliMemoryType) -> Self {
        match cli_type {
            CliMemoryType::Working => MemoryType::Working,
            CliMemoryType::Episodic => MemoryType::Episodic,
            CliMemoryType::Semantic => MemoryType::Semantic,
            CliMemoryType::Procedural => MemoryType::Procedural,
            CliMemoryType::Associative => MemoryType::Associative,
        }
    }
}

fn resolve_database_url(cli_url: Option<String>) -> String {
    if let Some(url) = cli_url {
        return url;
    }
    std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://localbrain:localbrain_secret@localhost:5433/local_brain".to_string()
    })
}

fn resolve_embedding_url(cli_url: Option<String>) -> String {
    if let Some(url) = cli_url {
        return url;
    }
    std::env::var("EMBEDDING_URL").unwrap_or_else(|_| "http://127.0.0.1:8081/embedding".to_string())
}

async fn build_pg_pool(db_url: &str) -> Result<sqlx::PgPool, String> {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(db_url)
        .await
        .map_err(|e| {
            format!(
                "No se pudo conectar a PostgreSQL en '{db_url}': {e}\n\
                 💡 Asegúrate de que el contenedor esté corriendo con: docker compose up -d"
            )
        })
}

struct AppContext {
    memory_repo: Arc<dyn MemoryRepository>,
    vector_repo: Arc<dyn VectorRepository>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
}

async fn build_context(
    in_memory: bool,
    db_url_override: Option<String>,
    emb_url_override: Option<String>,
) -> Result<AppContext, Box<dyn std::error::Error>> {
    if in_memory {
        Ok(AppContext {
            memory_repo: Arc::new(InMemoryMemoryRepository::new()),
            vector_repo: Arc::new(InMemoryVectorRepository::new()),
            embedding_provider: Arc::new(InMemoryEmbeddingProvider::new()),
        })
    } else {
        let db_url = resolve_database_url(db_url_override);
        let pool = build_pg_pool(&db_url).await?;
        let pg_repo = Arc::new(PostgresMemoryRepository::new(pool));

        let emb_url = resolve_embedding_url(emb_url_override);
        let emb_provider = Arc::new(
            LlamaCppEmbeddingProvider::new(
                LlamaCppConfig::new(emb_url).with_dimension(DEFAULT_EMBEDDING_DIMENSION),
            )
            .map_err(|e| e.to_string())?,
        );

        Ok(AppContext {
            memory_repo: pg_repo.clone(),
            vector_repo: pg_repo,
            embedding_provider: emb_provider,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => {
            println!("🧠 Inicializando Local Brain...");
            if cli.in_memory {
                println!(
                    "ℹ️ Modo --in-memory activo. No se requiere inicialización de PostgreSQL."
                );
                return Ok(());
            }

            let db_url = resolve_database_url(cli.database_url);
            let pool = match build_pg_pool(&db_url).await {
                Ok(p) => p,
                Err(err) => {
                    eprintln!("❌ Error de conexión:\n{err}");
                    std::process::exit(1);
                }
            };

            let repo = PostgresMemoryRepository::new(pool);
            match repo.run_migrations().await {
                Ok(_) => {
                    println!("✅ Base de datos inicializada exitosamente.");
                    println!("   Instancia:    PostgreSQL 17 + extensión pgvector");
                    println!(
                        "   Migraciones:  Aplicadas con éxito (memories, pgvector, HNSW 768 dim)."
                    );
                    println!("🚀 Local Brain está listo para operar con búsqueda semántica.");
                }
                Err(DomainError::RepositoryError(msg)) => {
                    eprintln!("❌ Error al ejecutar migraciones SQLx: {msg}");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("❌ Error inesperado: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Remember(args) => {
            let ctx = match build_context(cli.in_memory, cli.database_url, cli.embedding_url).await
            {
                Ok(c) => c,
                Err(err) => {
                    eprintln!("❌ {err}");
                    std::process::exit(1);
                }
            };

            let use_case = RememberUseCase::with_embedding(
                ctx.memory_repo,
                ctx.embedding_provider,
                ctx.vector_repo,
            );
            let mut cmd = RememberCommand::new(&args.content).with_type(args.memory_type.into());

            if let Some(project) = args.project {
                cmd = cmd.with_project(project);
            }
            if let Some(agent) = args.agent {
                cmd = cmd.with_agent(agent);
            }
            if let Some(imp) = args.importance {
                cmd = cmd.with_importance(imp);
            }
            if let Some(conf) = args.confidence {
                cmd = cmd.with_confidence(conf);
            }

            match use_case.execute(cmd).await {
                Ok(memory) => {
                    println!("✅ Recuerdo almacenado con éxito");
                    println!("   ID:          {}", memory.id);
                    println!("   Tipo:        {:?}", memory.memory_type);
                    println!("   Versión:     v{}", memory.version.value());
                    println!("   Hash:        {}", memory.content.hash());
                    println!("   Importancia: {:.2}", memory.importance.value());
                    println!("   Confianza:   {:.2}", memory.confidence.value());

                    if memory.status == MemoryStatus::PendingEmbedding {
                        println!(
                            "   Estado:      ⚠️  pending_embedding (llama.cpp no disponible; ejecuta 'brain embed-pending')"
                        );
                    } else {
                        println!("   Estado:      🟢 active (vector 768 dims indexado)");
                    }

                    if let Some(proj) = memory.project {
                        println!("   Proyecto:    {}", proj);
                    }
                    if let Some(agent) = memory.agent {
                        println!("   Agente:      {}", agent);
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error al guardar recuerdo: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Recall(args) => {
            let ctx = match build_context(cli.in_memory, cli.database_url, cli.embedding_url).await
            {
                Ok(c) => c,
                Err(err) => {
                    eprintln!("❌ {err}");
                    std::process::exit(1);
                }
            };

            let use_case = RecallUseCase::with_embedding(
                ctx.memory_repo,
                ctx.embedding_provider,
                ctx.vector_repo,
            );
            let mut query = RecallQuery::default().with_limit(args.limit);

            if let Some(q) = args.query {
                query = query.with_query(q);
            }
            if let Some(id_str) = args.id {
                match id_str.parse::<MemoryId>() {
                    Ok(id) => query.id = Some(id),
                    Err(e) => {
                        eprintln!("❌ ID de memoria inválido: {e}");
                        std::process::exit(1);
                    }
                }
            }
            if let Some(proj) = args.project {
                query.project = Some(proj);
            }
            if let Some(mtype) = args.memory_type {
                query.memory_type = Some(mtype.into());
            }
            if let Some(min_sim) = args.min_similarity {
                query = query.with_min_similarity(min_sim);
            }

            match use_case.execute(query).await {
                Ok(memories) => {
                    if memories.is_empty() {
                        println!("ℹ️ No se encontraron recuerdos con los criterios especificados.");
                    } else {
                        println!("🔍 Encontrados {} recuerdo(s):", memories.len());
                        for m in memories {
                            println!("──────────────────────────────────────────────────");
                            println!("ID:          {}", m.id);
                            println!(
                                "Tipo:        {:?} | Versión: v{} | Estado: {:?}",
                                m.memory_type,
                                m.version.value(),
                                m.status
                            );
                            if let Some(p) = &m.project {
                                println!("Proyecto:    {}", p);
                            }
                            if let Some(a) = &m.agent {
                                println!("Agente:      {}", a);
                            }
                            println!(
                                "Importancia: {:.2} | Confianza: {:.2} | Utilidad: {:.2}",
                                m.importance.value(),
                                m.confidence.value(),
                                m.utility.value()
                            );
                            println!(
                                "Creado:      {}",
                                m.created_at.format("%Y-%m-%d %H:%M:%S UTC")
                            );
                            println!("Contenido:   {}", m.content.text());
                        }
                        println!("──────────────────────────────────────────────────");
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error al recuperar recuerdos: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::EmbedPending(args) => {
            let ctx = match build_context(cli.in_memory, cli.database_url, cli.embedding_url).await
            {
                Ok(c) => c,
                Err(err) => {
                    eprintln!("❌ {err}");
                    std::process::exit(1);
                }
            };

            let use_case =
                EmbedPendingUseCase::new(ctx.memory_repo, ctx.embedding_provider, ctx.vector_repo);

            println!(
                "🔄 Procesando recuerdos pendientes de embedding (límite: {})...",
                args.limit
            );
            match use_case.execute(args.limit).await {
                Ok(processed) => {
                    if processed == 0 {
                        println!("✨ No hay recuerdos pendientes de indexación vectorial.");
                    } else {
                        println!(
                            "✅ Se generaron e indexaron embeddings para {} recuerdo(s).",
                            processed
                        );
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error al procesar embeddings pendientes: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Status => {
            println!("🧠 Local Brain — Estado del Sistema");
            println!("   Versión del Core:    {}", CORE_VERSION);
            println!(
                "   Dimensión Vectorial: {} (Alta Fidelidad Semántica)",
                DEFAULT_EMBEDDING_DIMENSION
            );
            println!("   Arquitectura:        Hexagonal / Clean Architecture (RNF-006)");

            if cli.in_memory {
                println!("   Persistencia:        Modo volátil en memoria (--in-memory)");
                return Ok(());
            }

            let db_url = resolve_database_url(cli.database_url);
            println!("   Persistencia:        PostgreSQL 17 + pgvector");
            println!("   URL Destino:         {}", db_url);

            match build_pg_pool(&db_url).await {
                Ok(pool) => {
                    println!("   Estado Conexión DB:  🟢 Conectado y operativo");

                    // Verificación de extensión pgvector
                    let ext_check: Result<Option<String>, _> = sqlx::query_scalar(
                        "SELECT installed_version FROM pg_available_extensions WHERE name = 'vector'",
                    )
                    .fetch_optional(&pool)
                    .await;

                    match ext_check {
                        Ok(Some(ver)) => {
                            println!("   Extensión pgvector:  🟢 Instalada (v{ver})");
                        }
                        Ok(None) => {
                            println!("   Extensión pgvector:  🟡 Disponible en PostgreSQL pero no activada. Ejecuta 'brain init'.");
                        }
                        Err(e) => {
                            println!(
                                "   Extensión pgvector:  ⚠️ Error al consultar extensión: {e}"
                            );
                        }
                    }

                    // Estadísticas de recuerdos
                    let stats_result = sqlx::query(
                        r#"
                        SELECT 
                            COUNT(*)::bigint AS total,
                            COUNT(*) FILTER (WHERE status = 'active')::bigint AS active,
                            COUNT(*) FILTER (WHERE status = 'pending_embedding')::bigint AS pending,
                            COUNT(*) FILTER (WHERE status = 'soft_deleted')::bigint AS soft_deleted
                        FROM memories
                        "#,
                    )
                    .fetch_one(&pool)
                    .await;

                    match stats_result {
                        Ok(row) => {
                            let total: i64 = row.try_get("total").unwrap_or(0);
                            let active: i64 = row.try_get("active").unwrap_or(0);
                            let pending: i64 = row.try_get("pending").unwrap_or(0);
                            let soft_deleted: i64 = row.try_get("soft_deleted").unwrap_or(0);
                            println!(
                                "   Recuerdos:           Total: {total} | Activos: {active} | Pendientes Embedding: {pending} | Eliminados: {soft_deleted}"
                            );
                        }
                        Err(sqlx::Error::Database(db_err))
                            if db_err.code().as_deref() == Some("42P01") =>
                        {
                            println!(
                                "   Esquema:             🟡 Tabla 'memories' no encontrada. Ejecuta 'brain init'."
                            );
                        }
                        Err(e) => {
                            println!(
                                "   Recuerdos:           ⚠️ No se pudieron leer estadísticas: {e}"
                            );
                        }
                    }
                }
                Err(err) => {
                    println!("   Estado Conexión DB:  🔴 No disponible");
                    eprintln!("   Detalle:             {err}");
                }
            }

            // Diagnóstico de endpoint llama.cpp
            let emb_url = resolve_embedding_url(cli.embedding_url);
            print!("   Runtime llama.cpp:   ");
            let http_client = reqwest::Client::builder()
                .timeout(Duration::from_millis(800))
                .build();

            if let Ok(client) = http_client {
                match client.get(&emb_url).send().await {
                    Ok(_) => println!("🟢 En línea ({emb_url})"),
                    Err(_) => println!("⚪ Fuera de línea o endpoint diferido ({emb_url})"),
                }
            } else {
                println!("⚠️ No se pudo inicializar cliente HTTP");
            }
        }
    }

    Ok(())
}
