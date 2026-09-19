//! # Local Brain CLI (`brain`)
//!
//! Interfaz de línea de comandos para gestión, inicialización, consulta y búsqueda semántica
//! de memoria cognitiva local (SRS §22, §25, §36).

use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use clap::{Args, Parser, Subcommand, ValueEnum};
use sqlx::Row;

use brain_application::{
    ConsolidateUseCase, EmbedPendingUseCase, ExpireSessionUseCase, ExplainQuery, ExplainUseCase,
    ForgetUseCase, HybridRetrieveQuery, HybridRetrieveUseCase, LearnCommand, LearnUseCase,
    ListConflictsQuery, ListConflictsUseCase, PurgeExpiredUseCase, RecallQuery, RecallUseCase,
    ReflectCommand, ReflectUseCase, RelateCommand, RelateUseCase, RememberCommand, RememberUseCase,
    ResolveConflictCommand, ResolveConflictUseCase, TraverseGraphQuery, TraverseGraphUseCase,
};
use brain_consolidation::in_memory::{InMemoryConflictRepository, MockConsolidationLlm};
use brain_consolidation::model::ConflictId;
use brain_consolidation::ports::{ConflictRepository, ConsolidationLlmPort};
use brain_core::CORE_VERSION;
use brain_domain::model::{DomainError, MemoryId, MemoryStatus, MemoryType};
use brain_domain::ports::{
    EmbeddingProvider, InMemoryEmbeddingProvider, InMemoryMemoryRepository,
    InMemoryVectorRepository, MemoryRepository, VectorRepository, DEFAULT_EMBEDDING_DIMENSION,
};
use brain_graph::{GraphRepository, InMemoryGraphRepository, RelationType, TraversalDirection};
use brain_infrastructure::embeddings::{LlamaCppConfig, LlamaCppEmbeddingProvider};
use brain_infrastructure::{
    LlamaCppReflectionClient, PostgresConflictRepository, PostgresGraphRepository,
    PostgresLearningRepository, PostgresMemoryRepository,
};
use brain_learning::{EvidenceSourceType, InMemoryLearningRepository, LearningRepository};
use brain_mcp::{McpSecurityPolicy, McpServer};
use brain_retrieval::{
    calculate_decay_factor, FullTextSearchRepository, InMemoryFullTextSearchRepository,
    RetrievalConfig,
};

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

    /// Inicia el servidor Model Context Protocol (MCP) sobre stdio para agentes de IA (SRS §21, §31, §32)
    Mcp(McpArgs),

    /// Finaliza una sesión de trabajo activa expirando sus recuerdos de trabajo (SRS §10.1, §21.4)
    SessionEnd(SessionEndArgs),

    /// Purga o archiva recuerdos volátiles expirados por TTL (SRS §10.1)
    PurgeExpired,

    /// Crea una relación tipada entre dos recuerdos o conceptos en el grafo de conocimiento (SRS §11, §25)
    Relate(RelateArgs),

    /// Explora y navega el grafo de conocimiento a partir de un nodo raíz (SRS §11, §25, §38)
    Graph(GraphArgs),

    /// Registra una observación empírica o propone conocimiento candidato con evidencias (SRS §15, §59, §60, Fase 6)
    Learn(LearnArgs),

    /// Explica el fundamento empírico, evidencias y nivel de confianza de un conocimiento o creencia (SRS §57, Fase 6)
    Explain(ExplainArgs),

    /// Ejecuta el proceso de reflexión asíncrona sobre experiencias recientes (SRS §16, §17, §18, Fase 7)
    Reflect(ReflectArgs),

    /// Gestiona y resuelve contradicciones cognitivas y conflictos de conocimiento (SRS §18, Fase 7)
    Conflicts(ConflictsArgs),

    /// Recuperación híbrida avanzada combinando vectores, FTS, grafo y scoring con decaimiento (SRS §13, §14, §56, §58, Fase 8)
    Retrieve(RetrieveArgs),

    /// Previsualiza el impacto del decaimiento temporal y políticas de olvido sin alterar datos (SRS §58, Fase 8)
    DecayPreview(DecayPreviewArgs),
}

#[derive(Args, Debug)]
struct RetrieveArgs {
    /// Consulta o intención de búsqueda semántica y léxica
    query: String,

    /// Filtro opcional por proyecto
    #[arg(short, long)]
    project: Option<String>,

    /// Filtro opcional por tipo de memoria
    #[arg(short = 't', long = "type", value_enum)]
    memory_type: Option<CliMemoryType>,

    /// Cantidad máxima de recuerdos relevantes a retornar (por defecto 10)
    #[arg(short = 'n', long = "limit", default_value_t = 10)]
    limit: usize,

    /// Puntaje mínimo de corte para admitir recuerdos en el ranking final
    #[arg(short = 'm', long = "min-score")]
    min_score: Option<f32>,

    /// Si se especifica, desglosa el cálculo matemático de scoring y señales de ranking (SRS §57)
    #[arg(short = 'e', long = "explain")]
    explain: bool,

    /// Ruta opcional a un archivo de configuración brain.toml personalizado
    #[arg(short = 'c', long = "config")]
    config: Option<String>,
}

#[derive(Args, Debug)]
struct DecayPreviewArgs {
    /// Filtro opcional por proyecto
    #[arg(short, long)]
    project: Option<String>,

    /// Cantidad máxima de recuerdos a analizar
    #[arg(short = 'n', long = "limit", default_value_t = 20)]
    limit: usize,

    /// Ruta opcional a un archivo de configuración brain.toml personalizado
    #[arg(short = 'c', long = "config")]
    config: Option<String>,
}

#[derive(Args, Debug)]
struct RelateArgs {
    /// Identificador del nodo origen (UUID de memoria o etiqueta conceptual)
    source: String,

    /// Identificador del nodo destino (UUID de memoria o etiqueta conceptual)
    target: String,

    /// Tipo de relación semántica o de conocimiento
    #[arg(short = 't', long = "type", value_enum, default_value_t = CliRelationType::RelatedTo)]
    relation_type: CliRelationType,

    /// Peso o fuerza de la relación (0.0 a 1.0)
    #[arg(short, long, default_value_t = 1.0)]
    weight: f32,

    /// Contexto situacional o justificación explicativa de la relación
    #[arg(short, long)]
    context: Option<String>,
}

#[derive(Args, Debug)]
struct GraphArgs {
    /// Nodo raíz para iniciar la exploración (UUID de memoria o etiqueta conceptual)
    root: String,

    /// Profundidad máxima de salto (hop depth, 1 a 5)
    #[arg(short = 'd', long = "depth", default_value_t = 2)]
    depth: u32,

    /// Límite máximo de nodos/aristas a recuperar
    #[arg(short = 'n', long = "limit", default_value_t = 50)]
    limit: usize,

    /// Filtrar por tipos de relación específicos
    #[arg(short = 't', long = "type", value_enum)]
    relation_types: Vec<CliRelationType>,

    /// Dirección del recorrido del grafo
    #[arg(long, value_enum, default_value_t = CliTraversalDirection::Outbound)]
    direction: CliTraversalDirection,

    /// Umbral mínimo de peso de arista
    #[arg(long)]
    min_weight: Option<f32>,

    /// Salida en formato JSON plano estructurado
    #[arg(long)]
    json: bool,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum CliRelationType {
    RelatedTo,
    UsedIn,
    CausedBy,
    Solves,
    Contradicts,
    Supersedes,
    DerivedFrom,
    DependsOn,
    Prefers,
    Avoid,
}

impl From<CliRelationType> for RelationType {
    fn from(cli: CliRelationType) -> Self {
        match cli {
            CliRelationType::RelatedTo => RelationType::RelatedTo,
            CliRelationType::UsedIn => RelationType::UsedIn,
            CliRelationType::CausedBy => RelationType::CausedBy,
            CliRelationType::Solves => RelationType::Solves,
            CliRelationType::Contradicts => RelationType::Contradicts,
            CliRelationType::Supersedes => RelationType::Supersedes,
            CliRelationType::DerivedFrom => RelationType::DerivedFrom,
            CliRelationType::DependsOn => RelationType::DependsOn,
            CliRelationType::Prefers => RelationType::Prefers,
            CliRelationType::Avoid => RelationType::Avoid,
        }
    }
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum CliTraversalDirection {
    Outbound,
    Inbound,
    Both,
}

impl From<CliTraversalDirection> for TraversalDirection {
    fn from(cli: CliTraversalDirection) -> Self {
        match cli {
            CliTraversalDirection::Outbound => TraversalDirection::Outbound,
            CliTraversalDirection::Inbound => TraversalDirection::Inbound,
            CliTraversalDirection::Both => TraversalDirection::Both,
        }
    }
}

#[derive(Args, Debug)]
struct LearnArgs {
    /// Afirmación, observación o creencia candidata
    statement: String,

    /// Evidencia empírica inicial que respalda o refuta la afirmación
    #[arg(short, long)]
    evidence: Option<String>,

    /// Origen o tipo de fuente de la evidencia
    #[arg(short = 's', long = "source-type", value_enum, default_value_t = CliEvidenceSourceType::DirectObservation)]
    source_type: CliEvidenceSourceType,

    /// Proyecto o dominio de conocimiento asociado
    #[arg(short, long)]
    domain: Option<String>,

    /// Identificador del agente que registra el aprendizaje
    #[arg(short, long)]
    agent: Option<String>,

    /// Indica si la evidencia refuta en lugar de apoyar la afirmación
    #[arg(long = "refutes", default_value_t = false)]
    refutes: bool,

    /// Marca la afirmación como validada y aprobada por un humano
    #[arg(long = "human-validated", default_value_t = false)]
    human_validated: bool,

    /// Registrar como observación factual puntual en lugar de creencia generalizada (SRS §59)
    #[arg(short = 'o', long = "observation", default_value_t = false)]
    is_observation: bool,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum CliEvidenceSourceType {
    Human,
    ToolExecution,
    DirectObservation,
    AgentHypothesis,
}

impl From<CliEvidenceSourceType> for EvidenceSourceType {
    fn from(val: CliEvidenceSourceType) -> Self {
        match val {
            CliEvidenceSourceType::Human => EvidenceSourceType::Human,
            CliEvidenceSourceType::ToolExecution => EvidenceSourceType::ToolExecution,
            CliEvidenceSourceType::DirectObservation => EvidenceSourceType::DirectObservation,
            CliEvidenceSourceType::AgentHypothesis => EvidenceSourceType::AgentHypothesis,
        }
    }
}

#[derive(Args, Debug)]
struct ExplainArgs {
    /// Pregunta conceptual, afirmación o UUID del conocimiento a explicar (ej. '¿Por qué recomendamos .NET?')
    query: String,

    /// Dominio o proyecto para filtrar la búsqueda
    #[arg(short, long)]
    domain: Option<String>,

    /// Muestra la salida en formato JSON estructurado
    #[arg(long, default_value_t = false)]
    json: bool,
}

#[derive(Args, Debug)]
struct ReflectArgs {
    /// Proyecto o dominio de conocimiento a reflexionar
    #[arg(short, long)]
    project: Option<String>,

    /// Límite máximo de recuerdos episódicos a analizar
    #[arg(short = 'n', long, default_value_t = 50)]
    limit: usize,

    /// Umbral mínimo de similitud coseno para clustering (0.0 a 1.0)
    #[arg(short = 't', long = "threshold", default_value_t = 0.82)]
    threshold: f32,

    /// Simula la reflexión sin persistir hipótesis ni estados de conflicto
    #[arg(long, default_value_t = false)]
    dry_run: bool,

    /// Muestra el resultado de la reflexión en formato JSON
    #[arg(long, default_value_t = false)]
    json: bool,
}

#[derive(Args, Debug)]
struct ConflictsArgs {
    #[command(subcommand)]
    command: ConflictCommands,
}

#[derive(Subcommand, Debug)]
enum ConflictCommands {
    /// Lista contradicciones pendientes de resolución
    List(ConflictListArgs),
    /// Resuelve una contradicción aportando contexto aclaratorio
    Resolve(ConflictResolveArgs),
}

#[derive(Args, Debug)]
struct ConflictListArgs {
    /// Filtrar por proyecto específico
    #[arg(short, long)]
    project: Option<String>,

    /// Salida en formato JSON
    #[arg(long, default_value_t = false)]
    json: bool,
}

#[derive(Args, Debug)]
struct ConflictResolveArgs {
    /// Identificador UUID del conflicto a resolver
    id: String,

    /// Contexto aclaratorio o directriz que resuelve la contradicción (SRS §18)
    #[arg(short, long)]
    resolution: String,
}

#[derive(Args, Debug)]
struct SessionEndArgs {
    /// Identificador de la sesión de trabajo que finaliza
    session_id: String,
}

#[derive(Args, Debug)]
struct McpArgs {
    /// Iniciar en modo solo lectura (deshabilita operaciones de escritura y borrado)
    #[arg(long)]
    read_only: bool,

    /// Permitir operaciones de borrado explícito mediante brain_forget (SRS §31)
    #[arg(long, default_value_t = true)]
    allow_delete: bool,
}

#[derive(Args, Debug)]
struct RememberArgs {
    /// Contenido textual del recuerdo (opcional si se proveen flags estructurados)
    content: Option<String>,

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

    /// Identificador de sesión para memorias de trabajo (working memory)
    #[arg(long)]
    session_id: Option<String>,

    /// Tiempo de vida en segundos antes de expirar (working memory)
    #[arg(long)]
    ttl: Option<u64>,

    /// Contexto situacional para recuerdos episódicos especializados
    #[arg(long)]
    context: Option<String>,

    /// Acción ejecutada para recuerdos episódicos especializados
    #[arg(long)]
    action: Option<String>,

    /// Resultado obtenido para recuerdos episódicos especializados
    #[arg(long)]
    outcome: Option<String>,

    /// Concepto de origen para memorias asociativas
    #[arg(long)]
    source_concept: Option<String>,

    /// Concepto de destino para memorias asociativas
    #[arg(long)]
    target_concept: Option<String>,

    /// Predicado o relación para memorias asociativas
    #[arg(long)]
    predicate: Option<String>,

    /// Fuerza de asociación (0.0 a 1.0)
    #[arg(long)]
    strength: Option<f32>,
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

    /// Filtrar por identificador de sesión
    #[arg(long)]
    session_id: Option<String>,

    /// Filtrar asociaciones conceptuales por término
    #[arg(long)]
    concept: Option<String>,

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
    fts_repo: Arc<dyn FullTextSearchRepository>,
    graph_repo: Arc<dyn GraphRepository>,
    learning_repo: Arc<dyn LearningRepository>,
    conflict_repo: Arc<dyn ConflictRepository>,
    llm_reflection_client: Arc<dyn ConsolidationLlmPort>,
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
            fts_repo: Arc::new(InMemoryFullTextSearchRepository::new()),
            graph_repo: Arc::new(InMemoryGraphRepository::new()),
            learning_repo: Arc::new(InMemoryLearningRepository::new()),
            conflict_repo: Arc::new(InMemoryConflictRepository::new()),
            llm_reflection_client: Arc::new(MockConsolidationLlm::new()),
        })
    } else {
        let db_url = resolve_database_url(db_url_override);
        let pool = build_pg_pool(&db_url).await?;
        let pg_repo = Arc::new(PostgresMemoryRepository::new(pool.clone()));
        let graph_repo = Arc::new(PostgresGraphRepository::new(pool.clone()));
        let learning_repo = Arc::new(PostgresLearningRepository::new(pool.clone()));
        let conflict_repo = Arc::new(PostgresConflictRepository::new(pool));

        let emb_url = resolve_embedding_url(emb_url_override);
        let emb_provider = Arc::new(
            LlamaCppEmbeddingProvider::new(
                LlamaCppConfig::new(emb_url.clone()).with_dimension(DEFAULT_EMBEDDING_DIMENSION),
            )
            .map_err(|e| e.to_string())?,
        );

        let completion_url = emb_url.replace("/embedding", "/completion");
        let llm_reflection_client = Arc::new(
            LlamaCppReflectionClient::from_endpoint(completion_url).map_err(|e| e.to_string())?,
        );

        Ok(AppContext {
            memory_repo: pg_repo.clone(),
            vector_repo: pg_repo.clone(),
            embedding_provider: emb_provider,
            fts_repo: pg_repo,
            graph_repo,
            learning_repo,
            conflict_repo,
            llm_reflection_client,
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

            let project = args.project.as_deref().unwrap_or("default");
            let agent = args.agent.as_deref().unwrap_or("cli");

            let mut cmd = if let (Some(ctx_str), Some(act), Some(out)) =
                (&args.context, &args.action, &args.outcome)
            {
                match RememberCommand::episodic(project, agent, ctx_str, act, out) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("❌ Error en parámetros de memoria episódica: {e}");
                        std::process::exit(1);
                    }
                }
            } else if let (Some(src), Some(tgt), Some(pred)) =
                (&args.source_concept, &args.target_concept, &args.predicate)
            {
                let strn = args.strength.unwrap_or(0.5);
                match RememberCommand::associative(src, tgt, pred, strn) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("❌ Error en parámetros de memoria asociativa: {e}");
                        std::process::exit(1);
                    }
                }
            } else if args.memory_type == CliMemoryType::Working
                || (args.session_id.is_some() && args.ttl.is_some())
            {
                let sid = args.session_id.as_deref().unwrap_or("default-session");
                let content = args.content.as_deref().unwrap_or("Memoria de trabajo CLI");
                match RememberCommand::working(sid, content, args.ttl) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("❌ Error en parámetros de memoria de trabajo: {e}");
                        std::process::exit(1);
                    }
                }
            } else if let Some(content) = args.content {
                RememberCommand::new(content).with_type(args.memory_type.into())
            } else {
                eprintln!("❌ Se requiere especificar el contenido o los flags especializados (--context/--action/--outcome, --source-concept/--target-concept/--predicate, o --session-id/--ttl).");
                std::process::exit(1);
            };

            if let Some(proj) = args.project {
                cmd = cmd.with_project(proj);
            }
            if let Some(ag) = args.agent {
                cmd = cmd.with_agent(ag);
            }
            if let Some(sid) = args.session_id {
                cmd = cmd.with_session(sid);
            }
            if let Some(ttl) = args.ttl {
                cmd = cmd.with_ttl(ttl);
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
            if let Some(sid) = args.session_id {
                query.session_id = Some(sid);
            }
            if let Some(concept) = args.concept {
                query.concept = Some(concept);
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

                            let type_stats = sqlx::query(
                                r#"
                                SELECT memory_type, COUNT(*)::bigint AS cnt
                                FROM memories
                                WHERE status = 'active'
                                GROUP BY memory_type
                                ORDER BY cnt DESC
                                "#,
                            )
                            .fetch_all(&pool)
                            .await;

                            if let Ok(rows) = type_stats {
                                if !rows.is_empty() {
                                    let type_summary: Vec<String> = rows
                                        .iter()
                                        .map(|r| {
                                            let mtype: String =
                                                r.try_get("memory_type").unwrap_or_default();
                                            let cnt: i64 = r.try_get("cnt").unwrap_or(0);
                                            format!("{mtype}: {cnt}")
                                        })
                                        .collect();
                                    println!(
                                        "   Por Tipo (activos):  {}",
                                        type_summary.join(" | ")
                                    );
                                }
                            }

                            // Estadísticas de Knowledge Graph (SRS §11, §25)
                            let graph_stats = sqlx::query(
                                r#"
                                SELECT 
                                    (SELECT COUNT(*) FROM graph_nodes)::bigint AS nodes_count,
                                    (SELECT COUNT(*) FROM graph_edges)::bigint AS edges_count
                                "#,
                            )
                            .fetch_one(&pool)
                            .await;

                            match graph_stats {
                                Ok(g_row) => {
                                    let nodes: i64 = g_row.try_get("nodes_count").unwrap_or(0);
                                    let edges: i64 = g_row.try_get("edges_count").unwrap_or(0);
                                    println!(
                                        "   Knowledge Graph:     Nodos: {nodes} | Aristas/Relaciones: {edges}"
                                    );
                                }
                                Err(_) => {
                                    println!(
                                        "   Knowledge Graph:     🟡 Tablas 'graph_nodes' / 'graph_edges' no inicializadas. Ejecuta 'brain init'."
                                    );
                                }
                            }

                            // Estadísticas de Learning Engine (SRS §15, §59, Fase 6)
                            let learning_stats = sqlx::query(
                                r#"
                                SELECT 
                                    (SELECT COUNT(*) FROM learning_candidates)::bigint AS candidates_count,
                                    (SELECT COUNT(*) FROM learning_evidence)::bigint AS evidences_count,
                                    (SELECT COUNT(*) FROM learning_candidates WHERE stage = 'validated')::bigint AS validated_count
                                "#,
                            )
                            .fetch_one(&pool)
                            .await;

                            if let Ok(l_row) = learning_stats {
                                let cands: i64 = l_row.try_get("candidates_count").unwrap_or(0);
                                let evs: i64 = l_row.try_get("evidences_count").unwrap_or(0);
                                let valids: i64 = l_row.try_get("validated_count").unwrap_or(0);
                                println!(
                                    "   Learning Engine:     Candidatos: {cands} | Evidencias: {evs} | Validados: {valids}"
                                );
                            }

                            // Estadísticas de Consolidación y Conflictos (SRS §16, §17, §18, Fase 7)
                            let consolidation_stats = sqlx::query(
                                r#"
                                SELECT 
                                    (SELECT COUNT(*) FROM consolidation_runs)::bigint AS runs_count,
                                    (SELECT COUNT(*) FROM knowledge_conflicts WHERE status = 'pending')::bigint AS pending_conflicts_count,
                                    (SELECT COUNT(*) FROM knowledge_conflicts WHERE status = 'resolved')::bigint AS resolved_conflicts_count
                                "#,
                            )
                            .fetch_one(&pool)
                            .await;

                            if let Ok(c_row) = consolidation_stats {
                                let runs: i64 = c_row.try_get("runs_count").unwrap_or(0);
                                let pending: i64 =
                                    c_row.try_get("pending_conflicts_count").unwrap_or(0);
                                let resolved: i64 =
                                    c_row.try_get("resolved_conflicts_count").unwrap_or(0);
                                println!(
                                    "   Consolidación:       Sesiones: {runs} | Conflictos Pendientes: {pending} | Resueltos: {resolved}"
                                );
                            }
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

        Commands::Mcp(args) => {
            let ctx = match build_context(cli.in_memory, cli.database_url, cli.embedding_url).await
            {
                Ok(c) => c,
                Err(err) => {
                    eprintln!("❌ {err}");
                    std::process::exit(1);
                }
            };

            let remember_uc = Arc::new(RememberUseCase::with_embedding(
                ctx.memory_repo.clone(),
                ctx.embedding_provider.clone(),
                ctx.vector_repo.clone(),
            ));
            let recall_uc = Arc::new(RecallUseCase::with_embedding(
                ctx.memory_repo.clone(),
                ctx.embedding_provider.clone(),
                ctx.vector_repo.clone(),
            ));
            let forget_uc = Arc::new(ForgetUseCase::new(ctx.memory_repo.clone()));
            let expire_session_uc = Arc::new(ExpireSessionUseCase::new(ctx.memory_repo.clone()));
            let relate_uc = Arc::new(
                RelateUseCase::new(ctx.graph_repo.clone())
                    .with_memory_repository(ctx.memory_repo.clone()),
            );
            let traverse_uc = Arc::new(TraverseGraphUseCase::new(ctx.graph_repo.clone()));
            let learn_uc = Arc::new(
                LearnUseCase::new(ctx.learning_repo.clone())
                    .with_memory_repository(ctx.memory_repo.clone()),
            );
            let explain_uc = Arc::new(
                ExplainUseCase::new(ctx.learning_repo.clone())
                    .with_memory_repository(ctx.memory_repo.clone()),
            );
            let reflect_uc = Arc::new(
                ReflectUseCase::new(
                    ctx.memory_repo.clone(),
                    ctx.conflict_repo.clone(),
                    ctx.llm_reflection_client.clone(),
                )
                .with_learning_repository(Some(ctx.learning_repo.clone()))
                .with_graph_repository(Some(ctx.graph_repo.clone())),
            );
            let consolidate_uc = Arc::new(
                ConsolidateUseCase::new(
                    ctx.memory_repo.clone(),
                    ctx.conflict_repo.clone(),
                    ctx.llm_reflection_client.clone(),
                )
                .with_learning_repository(Some(ctx.learning_repo.clone())),
            );
            let retrieve_uc = Arc::new(HybridRetrieveUseCase::new(
                ctx.memory_repo.clone(),
                ctx.vector_repo.clone(),
                ctx.embedding_provider.clone(),
                ctx.fts_repo.clone(),
                Some(ctx.graph_repo.clone()),
                RetrievalConfig::default(),
            ));

            let security = if args.read_only {
                McpSecurityPolicy::new_read_only()
            } else if args.allow_delete {
                McpSecurityPolicy::new_full()
            } else {
                McpSecurityPolicy::new_default()
            };

            let server = McpServer::new(remember_uc, recall_uc, forget_uc, security)
                .with_expire_session_uc(expire_session_uc)
                .with_graph(relate_uc, traverse_uc)
                .with_learning(learn_uc, explain_uc)
                .with_consolidation(reflect_uc, consolidate_uc)
                .with_retrieve_uc(retrieve_uc);
            if let Err(e) = server.run_stdio().await {
                eprintln!("❌ Error en servidor MCP: {e}");
                std::process::exit(1);
            }
        }

        Commands::Relate(args) => {
            let ctx = match build_context(cli.in_memory, cli.database_url, cli.embedding_url).await
            {
                Ok(c) => c,
                Err(err) => {
                    eprintln!("❌ {err}");
                    std::process::exit(1);
                }
            };

            let use_case =
                RelateUseCase::new(ctx.graph_repo).with_memory_repository(ctx.memory_repo);
            let mut cmd = RelateCommand::new(args.source, args.target, args.relation_type.into())
                .with_weight(args.weight);

            if let Some(ctx_str) = args.context {
                cmd = cmd.with_context(ctx_str);
            }

            match use_case.execute(cmd).await {
                Ok(edge) => {
                    println!("✅ Relación establecida exitosamente en el Knowledge Graph");
                    println!("   Edge ID:     {}", edge.id);
                    println!("   Origen:      {}", edge.source_id);
                    println!("   Destino:     {}", edge.target_id);
                    println!("   Tipo:        {}", edge.relation_type.as_str());
                    println!("   Peso:        {:.2}", edge.weight);
                    if !edge.metadata.is_null() && edge.metadata != serde_json::json!({}) {
                        println!(
                            "   Metadatos:   {}",
                            serde_json::to_string(&edge.metadata).unwrap_or_default()
                        );
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error al crear relación en el grafo: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Graph(args) => {
            let ctx = match build_context(cli.in_memory, cli.database_url, cli.embedding_url).await
            {
                Ok(c) => c,
                Err(err) => {
                    eprintln!("❌ {err}");
                    std::process::exit(1);
                }
            };

            let use_case = TraverseGraphUseCase::new(ctx.graph_repo);
            let mut query = TraverseGraphQuery::new(args.root)
                .with_depth(args.depth)
                .with_direction(args.direction.into());

            if !args.relation_types.is_empty() {
                query =
                    query.with_relations(args.relation_types.into_iter().map(Into::into).collect());
            }
            if let Some(min_w) = args.min_weight {
                query.min_weight = Some(min_w);
            }
            query.limit = Some(args.limit);

            match use_case.execute(query).await {
                Ok(subgraph) => {
                    if args.json {
                        match serde_json::to_string_pretty(&subgraph) {
                            Ok(json_str) => println!("{json_str}"),
                            Err(e) => {
                                eprintln!("❌ Error serializando grafo a JSON: {e}");
                                std::process::exit(1);
                            }
                        }
                    } else {
                        println!(
                            "🕸️ Knowledge Graph — Subgrafo desde '{}' ({})",
                            subgraph.root.label, subgraph.root.id
                        );
                        println!("   Profundidad Máxima:  {}", args.depth);
                        println!("   Dirección:           {:?}", args.direction);
                        println!("   Nodos alcanzados:    {}", subgraph.nodes.len());
                        println!("   Relaciones/Aristas:  {}", subgraph.edges.len());

                        if subgraph.nodes.is_empty() && subgraph.edges.is_empty() {
                            println!("\nℹ️ No se encontraron nodos conectados adicionales.");
                        } else {
                            println!("\n📍 Nodos:");
                            for node in &subgraph.nodes {
                                let is_root = if node.id == subgraph.root.id {
                                    " (RAÍZ)"
                                } else {
                                    ""
                                };
                                println!("   • [{:?}] {}{}", node.node_type, node.label, is_root);
                                println!("     ID: {}", node.id);
                            }

                            if !subgraph.edges.is_empty() {
                                println!("\n🔗 Relaciones:");
                                for edge in &subgraph.edges {
                                    println!(
                                        "   • {} --[{}]--> {} (peso: {:.2})",
                                        edge.source_id,
                                        edge.relation_type.as_str(),
                                        edge.target_id,
                                        edge.weight
                                    );
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error al explorar el grafo de conocimiento: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::SessionEnd(args) => {
            let ctx = match build_context(cli.in_memory, cli.database_url, cli.embedding_url).await
            {
                Ok(c) => c,
                Err(err) => {
                    eprintln!("❌ {err}");
                    std::process::exit(1);
                }
            };

            let use_case = ExpireSessionUseCase::new(ctx.memory_repo);
            println!("🔄 Finalizando sesión de trabajo '{}'...", args.session_id);
            match use_case.execute(&args.session_id).await {
                Ok(count) => {
                    println!("✅ Sesión '{}' finalizada exitosamente.", args.session_id);
                    println!("   Recuerdos de trabajo archivados/expirados: {count}");
                }
                Err(e) => {
                    eprintln!("❌ Error al finalizar sesión: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::PurgeExpired => {
            let ctx = match build_context(cli.in_memory, cli.database_url, cli.embedding_url).await
            {
                Ok(c) => c,
                Err(err) => {
                    eprintln!("❌ {err}");
                    std::process::exit(1);
                }
            };

            let use_case = PurgeExpiredUseCase::new(ctx.memory_repo);
            println!("🔄 Purgando recuerdos volátiles expirados...");
            match use_case.execute().await {
                Ok(count) => {
                    println!("✅ Purga de recuerdos expirados completada exitosamente.");
                    println!("   Recuerdos purgados: {count}");
                }
                Err(e) => {
                    eprintln!("❌ Error al purgar recuerdos expirados: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Learn(args) => {
            let ctx = match build_context(cli.in_memory, cli.database_url, cli.embedding_url).await
            {
                Ok(c) => c,
                Err(err) => {
                    eprintln!("❌ {err}");
                    std::process::exit(1);
                }
            };

            let use_case =
                LearnUseCase::new(ctx.learning_repo).with_memory_repository(ctx.memory_repo);

            let mut cmd = if args.is_observation {
                LearnCommand::new_observation(args.statement)
            } else {
                LearnCommand::new_candidate(args.statement)
            };

            if let Some(ev) = args.evidence {
                cmd = cmd.with_evidence(ev, args.source_type.into());
            }
            if let Some(dom) = args.domain {
                cmd = cmd.with_domain(dom);
            }
            if let Some(ag) = args.agent {
                cmd = cmd.with_agent(ag);
            }
            cmd = cmd.with_supporting(!args.refutes);
            cmd = cmd.with_human_validated(args.human_validated);

            match use_case.execute(cmd).await {
                Ok(res) => {
                    println!("🧠 Aprendizaje registrado exitosamente en Local Brain");
                    println!("   Candidato ID:        {}", res.candidate_id);
                    println!("   Afirmación:          {}", res.statement);
                    println!(
                        "   Etapa:               {}",
                        res.stage.as_str().to_uppercase()
                    );
                    println!("   Confianza:           {:.2}", res.confidence);
                    println!("   Evidencias:          {}", res.evidence_count);
                    if let Some(ref dom) = res.domain {
                        println!("   Dominio:             {dom}");
                    }
                    println!(
                        "   Validación Humana:   {}",
                        if res.human_validated {
                            "Sí (Aprobado)"
                        } else {
                            "No"
                        }
                    );
                }
                Err(e) => {
                    eprintln!("❌ Error al registrar aprendizaje: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Explain(args) => {
            let ctx = match build_context(cli.in_memory, cli.database_url, cli.embedding_url).await
            {
                Ok(c) => c,
                Err(err) => {
                    eprintln!("❌ {err}");
                    std::process::exit(1);
                }
            };

            let use_case =
                ExplainUseCase::new(ctx.learning_repo).with_memory_repository(ctx.memory_repo);

            let mut query = ExplainQuery::new(args.query);
            if let Some(dom) = args.domain {
                query = query.with_domain(dom);
            }

            match use_case.execute(query).await {
                Ok(report) => {
                    if args.json {
                        match serde_json::to_string_pretty(&report) {
                            Ok(json_str) => println!("{json_str}"),
                            Err(e) => {
                                eprintln!("❌ Error al serializar explicación a JSON: {e}");
                                std::process::exit(1);
                            }
                        }
                    } else {
                        println!("🧠 Local Brain — Explicabilidad Cognitiva (SRS §57)");
                        println!("───────────────────────────────────────────────────");
                        println!("{}", report.formatted_explanation);
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error al generar explicación: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Reflect(args) => {
            let ctx = match build_context(cli.in_memory, cli.database_url, cli.embedding_url).await
            {
                Ok(c) => c,
                Err(err) => {
                    eprintln!("❌ {err}");
                    std::process::exit(1);
                }
            };

            let use_case = ReflectUseCase::new(
                ctx.memory_repo,
                ctx.conflict_repo,
                ctx.llm_reflection_client,
            )
            .with_learning_repository(Some(ctx.learning_repo))
            .with_graph_repository(Some(ctx.graph_repo));

            let mut cmd = ReflectCommand::new()
                .with_limit(args.limit)
                .with_threshold(args.threshold)
                .with_dry_run(args.dry_run);

            if let Some(proj) = args.project {
                cmd = cmd.with_project(proj);
            }

            match use_case.execute(cmd).await {
                Ok(report) => {
                    if args.json {
                        match serde_json::to_string_pretty(&report) {
                            Ok(json_str) => println!("{json_str}"),
                            Err(e) => {
                                eprintln!("❌ Error al serializar reporte a JSON: {e}");
                                std::process::exit(1);
                            }
                        }
                    } else {
                        println!("🧠 Local Brain — Sesión de Reflexión y Consolidación (SRS §16, §17, §18)");
                        println!(
                            "───────────────────────────────────────────────────────────────────"
                        );
                        println!("   Run ID:               {}", report.run_id);
                        println!("   Memorias analizadas:  {}", report.memories_analyzed);
                        println!("   Clusters formados:    {}", report.clusters_formed);
                        println!(
                            "   Patrones detectados:  {}",
                            report.patterns_detected.len()
                        );
                        println!("   Hipótesis generadas:  {}", report.hypotheses.len());
                        println!(
                            "   Conflictos señalados: {}",
                            report.conflicts_detected.len()
                        );
                        if args.dry_run {
                            println!(
                                "   Modo:                 SIMULACIÓN (dry-run, no persistido)"
                            );
                        }
                        println!("\n📊 Resumen:\n   {}", report.summary);

                        if !report.conflicts_detected.is_empty() {
                            println!("\n⚠️ Contradicciones detectadas:");
                            for conflict in &report.conflicts_detected {
                                println!(
                                    "   - Conflicto [{}] (Tipo: {})",
                                    conflict.id,
                                    conflict.conflict_type.as_str()
                                );
                                println!("     Razón: {}", conflict.reason);
                                if let Some(ref sugg) = conflict.suggested_resolution {
                                    println!("     Sugerencia: {}", sugg);
                                }
                            }
                            println!("   (Usa 'brain conflicts resolve <ID> --resolution <TEXT>' para resolverlos)");
                        }

                        if !report.hypotheses.is_empty() {
                            println!("\n💡 Hipótesis candidatas creadas:");
                            for hyp in &report.hypotheses {
                                println!(
                                    "   - {} (Confianza sugerida: {:.2})",
                                    hyp.statement, hyp.suggested_confidence
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error durante la reflexión: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Conflicts(args) => {
            let ctx = match build_context(cli.in_memory, cli.database_url, cli.embedding_url).await
            {
                Ok(c) => c,
                Err(err) => {
                    eprintln!("❌ {err}");
                    std::process::exit(1);
                }
            };

            match args.command {
                ConflictCommands::List(list_args) => {
                    let use_case = ListConflictsUseCase::new(ctx.conflict_repo);
                    let query = ListConflictsQuery {
                        project: list_args.project,
                    };

                    match use_case.execute(query).await {
                        Ok(conflicts) => {
                            if list_args.json {
                                match serde_json::to_string_pretty(&conflicts) {
                                    Ok(j) => println!("{j}"),
                                    Err(e) => {
                                        eprintln!("❌ Error serializando a JSON: {e}");
                                        std::process::exit(1);
                                    }
                                }
                            } else {
                                println!("🧠 Local Brain — Contradicciones Pendientes (SRS §18)");
                                println!("───────────────────────────────────────────────────");
                                if conflicts.is_empty() {
                                    println!("✅ No hay contradicciones pendientes.");
                                } else {
                                    for c in &conflicts {
                                        println!("⚡ Conflicto ID:         {}", c.id);
                                        println!(
                                            "   Tipo:                 {}",
                                            c.conflict_type.as_str()
                                        );
                                        println!("   Razón:                {}", c.reason);
                                        println!("   Memoria Origen:       {}", c.source_memory_id);
                                        println!(
                                            "   Memoria En Conflicto: {}",
                                            c.conflicting_memory_id
                                        );
                                        if let Some(ref sugg) = c.suggested_resolution {
                                            println!("   Resolución sugerida:  {}", sugg);
                                        }
                                        println!("   Detectado:            {}", c.detected_at);
                                        println!("   ─────────────────────────────────────────");
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ Error al listar conflictos: {e}");
                            std::process::exit(1);
                        }
                    }
                }

                ConflictCommands::Resolve(res_args) => {
                    let conflict_id = match ConflictId::from_str(&res_args.id) {
                        Ok(id) => id,
                        Err(e) => {
                            eprintln!("❌ Identificador de conflicto inválido: {e}");
                            std::process::exit(1);
                        }
                    };

                    let use_case = ResolveConflictUseCase::new(ctx.conflict_repo, ctx.memory_repo);
                    let cmd = ResolveConflictCommand {
                        conflict_id,
                        resolution_context: res_args.resolution,
                    };

                    match use_case.execute(cmd).await {
                        Ok(resolved) => {
                            println!("✅ Conflicto resuelto exitosamente");
                            println!("   Conflicto ID: {}", resolved.id);
                            println!(
                                "   Estado:       {}",
                                resolved.status.as_str().to_uppercase()
                            );
                            if let Some(ref ctx_text) = resolved.resolution_context {
                                println!("   Contexto:     {}", ctx_text);
                            }
                            println!("   Las memorias han sido restauradas al estado ACTIVE con contexto actualizado.");
                        }
                        Err(e) => {
                            eprintln!("❌ Error al resolver conflicto: {e}");
                            std::process::exit(1);
                        }
                    }
                }
            }
        }

        Commands::Retrieve(args) => {
            let ctx = match build_context(cli.in_memory, cli.database_url, cli.embedding_url).await
            {
                Ok(c) => c,
                Err(err) => {
                    eprintln!("❌ {err}");
                    std::process::exit(1);
                }
            };

            let config = if let Some(ref path) = args.config {
                match std::fs::read_to_string(path) {
                    Ok(content) => match RetrievalConfig::from_toml(&content) {
                        Ok(cfg) => cfg,
                        Err(e) => {
                            eprintln!("❌ Error en archivo de configuración '{path}': {e}");
                            std::process::exit(1);
                        }
                    },
                    Err(e) => {
                        eprintln!("❌ No se pudo leer archivo de configuración '{path}': {e}");
                        std::process::exit(1);
                    }
                }
            } else if std::path::Path::new("brain.toml").exists() {
                match std::fs::read_to_string("brain.toml") {
                    Ok(content) => RetrievalConfig::from_toml(&content).unwrap_or_default(),
                    Err(_) => RetrievalConfig::default(),
                }
            } else {
                RetrievalConfig::default()
            };

            let use_case = HybridRetrieveUseCase::new(
                ctx.memory_repo,
                ctx.vector_repo,
                ctx.embedding_provider,
                ctx.fts_repo,
                Some(ctx.graph_repo),
                config,
            );

            let mut query = HybridRetrieveQuery::new(args.query.clone())
                .with_limit(args.limit)
                .with_explain(args.explain);

            if let Some(proj) = args.project {
                query = query.with_project(proj);
            }
            if let Some(mtype) = args.memory_type {
                query = query.with_type(mtype.into());
            }
            if let Some(min_score) = args.min_score {
                query = query.with_min_score(min_score);
            }

            match use_case.execute(query).await {
                Ok(result) => {
                    println!("🧠 Local Brain — Recuperación Híbrida Avanzada (SRS §13, §14, §56)");
                    println!("─────────────────────────────────────────────────────────────────────────────");
                    println!("Consulta:           \"{}\"", args.query);
                    println!(
                        "Candidatos:         {} únicos (Vector: {}, FTS: {}, Grafo: {})",
                        result.metrics.merged_unique_count,
                        result.metrics.vector_candidates_count,
                        result.metrics.fts_candidates_count,
                        result.metrics.graph_candidates_count
                    );
                    println!("Recuerdos devueltos: {}", result.items.len());
                    println!("─────────────────────────────────────────────────────────────────────────────");

                    if result.items.is_empty() {
                        println!("ℹ️ No se encontraron recuerdos relevantes que superen los umbrales configurados.");
                    } else {
                        for (idx, item) in result.items.iter().enumerate() {
                            let rank = idx + 1;
                            println!(
                                "\n[{}] Score: {:.4} | ID: {} | Tipo: {:?}",
                                rank,
                                item.explanation.final_score,
                                item.memory.id,
                                item.memory.memory_type
                            );
                            if let Some(ref proj) = item.memory.project {
                                println!("    Proyecto:    {}", proj);
                            }
                            println!(
                                "    Factores:    Similitud: {:.2} | Importancia: {:.2} | Confianza: {:.2} | Recencia: {:.2}",
                                item.explanation.semantic_similarity,
                                item.explanation.importance,
                                item.explanation.confidence,
                                item.explanation.recency_factor
                            );
                            if args.explain {
                                println!(
                                    "    Explicación: {}",
                                    item.explanation.to_formatted_summary()
                                );
                                if !item.explanation.signals.is_empty() {
                                    println!(
                                        "    Señales:     {}",
                                        item.explanation.signals.join(", ")
                                    );
                                }
                            }
                            let preview = item.memory.content.text().trim();
                            let truncated = if preview.len() > 180 {
                                format!("{}...", &preview[..180])
                            } else {
                                preview.to_string()
                            };
                            println!("    Contenido:   {}", truncated);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error durante la recuperación híbrida: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::DecayPreview(args) => {
            let ctx = match build_context(cli.in_memory, cli.database_url, cli.embedding_url).await
            {
                Ok(c) => c,
                Err(err) => {
                    eprintln!("❌ {err}");
                    std::process::exit(1);
                }
            };

            let config = if let Some(ref path) = args.config {
                match std::fs::read_to_string(path) {
                    Ok(content) => match RetrievalConfig::from_toml(&content) {
                        Ok(cfg) => cfg,
                        Err(e) => {
                            eprintln!("❌ Error en archivo de configuración '{path}': {e}");
                            std::process::exit(1);
                        }
                    },
                    Err(e) => {
                        eprintln!("❌ No se pudo leer archivo de configuración '{path}': {e}");
                        std::process::exit(1);
                    }
                }
            } else if std::path::Path::new("brain.toml").exists() {
                match std::fs::read_to_string("brain.toml") {
                    Ok(content) => RetrievalConfig::from_toml(&content).unwrap_or_default(),
                    Err(_) => RetrievalConfig::default(),
                }
            } else {
                RetrievalConfig::default()
            };

            let memories = if let Some(ref proj) = args.project {
                ctx.memory_repo.find_by_project(proj, args.limit).await
            } else {
                ctx.memory_repo
                    .find_by_status(MemoryStatus::Active, args.limit)
                    .await
            };

            match memories {
                Ok(list) => {
                    let now = chrono::Utc::now();
                    println!(
                        "🧠 Local Brain — Auditoría de Decaimiento Temporal y Olvido (SRS §58)"
                    );
                    println!("─────────────────────────────────────────────────────────────────────────────");
                    println!(
                        "Estrategia:     {:?} (Vida media: {} días | Umbral protegido: >= {})",
                        config.decay.strategy,
                        config.decay.half_life_days,
                        config.decay.protected_importance_threshold
                    );
                    println!("Recuerdos evaluados: {}", list.len());
                    println!("─────────────────────────────────────────────────────────────────────────────");

                    if list.is_empty() {
                        println!("ℹ️ No hay recuerdos activos para evaluar.");
                    } else {
                        for m in &list {
                            let factor = calculate_decay_factor(
                                m.created_at,
                                m.last_retrieved_at,
                                now,
                                m.importance.value(),
                                m.confidence.value(),
                                m.utility.value(),
                                &config.decay,
                            );

                            let is_protected =
                                m.importance.value() >= config.decay.protected_importance_threshold;
                            let status_badge = if is_protected {
                                "🛡️ PROTEGIDO (Decisión clave)"
                            } else if factor < 0.5 {
                                "⏳ DECAÍDO (Baja recencia)"
                            } else {
                                "⚡ ACTIVO"
                            };

                            let last_interaction = m.last_retrieved_at.unwrap_or(m.created_at);
                            let days_inactive =
                                (now.signed_duration_since(last_interaction).num_seconds() as f32
                                    / 86400.0)
                                    .max(0.0);

                            println!(
                                "• ID: {} | Tipo: {:?} | {}",
                                m.id, m.memory_type, status_badge
                            );
                            println!(
                                "  Importancia: {:.2} | Confianza: {:.2} | Inactivo: {:.1} días | Factor Decaimiento: {:.3}",
                                m.importance.value(),
                                m.confidence.value(),
                                days_inactive,
                                factor
                            );
                            let snippet = m.content.text().trim();
                            let preview = if snippet.len() > 100 {
                                format!("{}...", &snippet[..100])
                            } else {
                                snippet.to_string()
                            };
                            println!("  Resumen:     {}", preview);
                            println!();
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error al cargar recuerdos para auditoría de decaimiento: {e}");
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}
