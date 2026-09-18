//! # Brain Application
//!
//! Capa de aplicación y orquestación de casos de uso de Local Brain (SRS §8).
//!
//! Conecta los adaptadores primarios (MCP, CLI) con el modelo de dominio puro
//! y coordina los puertos secundarios de persistencia y embeddings.

pub mod graph_use_cases;
pub mod use_cases;

pub use graph_use_cases::{RelateCommand, RelateUseCase, TraverseGraphQuery, TraverseGraphUseCase};
pub use use_cases::{
    ApplicationError, EmbedPendingUseCase, ExpireSessionUseCase, ForgetUseCase,
    PurgeExpiredUseCase, RecallQuery, RecallUseCase, RememberCommand, RememberUseCase,
};
