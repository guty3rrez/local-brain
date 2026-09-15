//! Módulo de persistencia secundaria para Local Brain.

pub mod postgres_repository;

pub use postgres_repository::PostgresMemoryRepository;
