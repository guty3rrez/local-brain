//! # Brain Application
//!
//! Capa de aplicación y orquestación de casos de uso de Local Brain (SRS §8).
//!
//! Conecta los adaptadores primarios (MCP, CLI) con el modelo de dominio puro
//! y coordina los puertos secundarios de persistencia, embeddings, aprendizaje y consolidación.

pub mod backup_use_cases;
pub mod consolidation_use_cases;
pub mod doctor_use_cases;
pub mod graph_use_cases;
pub mod hybrid_retrieval_use_cases;
pub mod learning_use_cases;
pub mod use_cases;

pub use backup_use_cases::{
    BackupCommand, BackupPayload, BackupRepository, BackupResult, BackupUseCase,
    InMemoryBackupRepository, RestoreCommand, RestoreCounts, RestoreReport, RestoreUseCase,
};
pub use consolidation_use_cases::{
    ConsolidateCommand, ConsolidateUseCase, ListConflictsQuery, ListConflictsUseCase,
    ReflectCommand, ReflectUseCase, ResolveConflictCommand, ResolveConflictUseCase,
};
pub use doctor_use_cases::{DoctorCommand, DoctorDiagnostician, DoctorUseCase};
pub use graph_use_cases::{RelateCommand, RelateUseCase, TraverseGraphQuery, TraverseGraphUseCase};
pub use hybrid_retrieval_use_cases::{
    HybridRetrieveQuery, HybridRetrieveResult, HybridRetrieveUseCase,
};
pub use learning_use_cases::{
    AddEvidenceCommand, AddEvidenceUseCase, ExplainQuery, ExplainUseCase, ExplanationReport,
    LearnCommand, LearnResult, LearnUseCase,
};
pub use use_cases::{
    ApplicationError, EmbedPendingUseCase, ExpireSessionUseCase, ForgetUseCase,
    PurgeExpiredUseCase, RecallQuery, RecallUseCase, RememberCommand, RememberUseCase,
};
