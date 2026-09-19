//! # Local Brain — Retrieval Engine (`brain-retrieval`)
//!
//! Dominio puro para recuperación híbrida avanzada, scoring multidimensional,
//! decaimiento temporal y fusión de candidatos (SRS §13, §14, §56, §58).

pub mod config;
pub mod decay;
pub mod errors;
pub mod in_memory;
pub mod pipeline;
pub mod ports;
pub mod scoring;

pub use config::{
    DecayConfig, DecayStrategy, FusionWeights, RetrievalConfig, RetrievalLimits, ScoringWeights,
};
pub use decay::calculate_decay_factor;
pub use errors::RetrievalError;
pub use in_memory::InMemoryFullTextSearchRepository;
pub use pipeline::{CandidateFusionScore, HybridRetrievalPipeline, RetrievalMetrics, ScoredMemory};
pub use ports::FullTextSearchRepository;
pub use scoring::{MultidimensionalScorer, ScoreExplanation};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retrieval_module_exports_healthy() {
        let config = RetrievalConfig::default();
        assert!(config.limits.default_limit > 0);
        let pipeline = HybridRetrievalPipeline::new(config);
        assert_eq!(pipeline.config().limits.default_limit, 10);
    }
}
