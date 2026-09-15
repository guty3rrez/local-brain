//! Adaptadores de proveedores de embeddings locales e inferencia (SRS §12).

pub mod llama_cpp;

pub use llama_cpp::{LlamaCppConfig, LlamaCppEmbeddingProvider};
