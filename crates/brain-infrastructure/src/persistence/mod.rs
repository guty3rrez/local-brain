//! Módulo de persistencia secundaria para Local Brain.

pub mod postgres_backup_repository;
pub mod postgres_conflict_repository;
pub mod postgres_doctor_diagnostician;
pub mod postgres_graph_repository;
pub mod postgres_learning_repository;
pub mod postgres_repository;

pub use postgres_backup_repository::PostgresBackupRepository;
pub use postgres_conflict_repository::PostgresConflictRepository;
pub use postgres_doctor_diagnostician::PostgresDoctorDiagnostician;
pub use postgres_graph_repository::PostgresGraphRepository;
pub use postgres_learning_repository::PostgresLearningRepository;
pub use postgres_repository::PostgresMemoryRepository;
