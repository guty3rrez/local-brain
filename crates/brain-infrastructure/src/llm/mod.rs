//! Módulo de integración LLM local para razonamiento, síntesis y reflexión (SRS §16, §17, §18).

pub mod llama_cpp_reflection;

pub use llama_cpp_reflection::{LlamaCppReflectionClient, LlamaCppReflectionConfig};
