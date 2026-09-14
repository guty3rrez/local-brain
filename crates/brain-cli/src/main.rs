//! # Local Brain CLI (`brain`)
//!
//! Interfaz de línea de comandos para gestión y consulta de memoria cognitiva local (SRS §25).

use std::sync::Arc;

use clap::{Args, Parser, Subcommand, ValueEnum};

use brain_application::{RecallQuery, RecallUseCase, RememberCommand, RememberUseCase};
use brain_core::CORE_VERSION;
use brain_domain::model::{MemoryId, MemoryType};
use brain_domain::ports::InMemoryMemoryRepository;

#[derive(Parser, Debug)]
#[command(
    name = "brain",
    version = CORE_VERSION,
    about = "🧠 Local Brain — Memoria persistente local, agéntica y orientada a conocimiento",
    long_about = "Local Brain proporciona persistencia de memoria cognitiva local-first con tipos especializados (episódica, semántica, procedimental, etc.) para desarrolladores y agentes de IA."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Registra un nuevo recuerdo en el cerebro local
    Remember(RememberArgs),

    /// Recupera recuerdos almacenados por ID, proyecto o tipo
    Recall(RecallArgs),

    /// Muestra el estado del daemon y estadísticas del sistema
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
    /// Identificador UUID de un recuerdo específico
    #[arg(long)]
    id: Option<String>,

    /// Filtrar por proyecto
    #[arg(short, long)]
    project: Option<String>,

    /// Filtrar por tipo de memoria
    #[arg(short = 't', long = "type", value_enum)]
    memory_type: Option<CliMemoryType>,

    /// Límite máximo de resultados
    #[arg(short = 'n', long, default_value_t = 5)]
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Repositorio en memoria base para la CLI local
    let repo = Arc::new(InMemoryMemoryRepository::new());

    match cli.command {
        Commands::Remember(args) => {
            let use_case = RememberUseCase::new(repo);
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

            let memory = use_case.execute(cmd).await?;
            println!("✅ Recuerdo almacenado con éxito");
            println!("   ID:         {}", memory.id);
            println!("   Tipo:       {:?}", memory.memory_type);
            println!("   Versión:    v{}", memory.version.value());
            println!("   Hash:       {}", memory.content.hash());
            if let Some(proj) = memory.project {
                println!("   Proyecto:   {}", proj);
            }
        }
        Commands::Recall(args) => {
            let use_case = RecallUseCase::new(repo);
            let mut query = RecallQuery::default().with_limit(args.limit);

            if let Some(id_str) = args.id {
                let id: MemoryId = id_str.parse()?;
                query.id = Some(id);
            }
            if let Some(proj) = args.project {
                query.project = Some(proj);
            }
            if let Some(mtype) = args.memory_type {
                query.memory_type = Some(mtype.into());
            }

            match use_case.execute(query).await {
                Ok(memories) => {
                    if memories.is_empty() {
                        println!("ℹ️ No se encontraron recuerdos con los criterios especificados.");
                    } else {
                        println!("🔍 Encontrados {} recuerdo(s):", memories.len());
                        for m in memories {
                            println!("--------------------------------------------------");
                            println!("ID:       {}", m.id);
                            println!("Tipo:     {:?}", m.memory_type);
                            println!("Contenido: {}", m.content.text());
                            if let Some(p) = m.project {
                                println!("Proyecto: {}", p);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error al recuperar recuerdos: {e}");
                }
            }
        }
        Commands::Status => {
            println!("🧠 Local Brain — Estado del Sistema");
            println!("   Versión del Core: {}", CORE_VERSION);
            println!("   Arquitectura:     Hexagonal (6 crates configuradas)");
            println!("   Persistencia:     PostgreSQL 17 + pgvector (docker-compose: 5433)");
            println!("   Dominio:          Puro e independiente (RNF-006)");
        }
    }

    Ok(())
}
