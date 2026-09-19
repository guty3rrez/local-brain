//! Algoritmos deterministas de agrupamiento (Clustering) de experiencias (SRS §16, §17).

use std::collections::HashMap;

use brain_domain::model::MemoryId;

use crate::errors::ConsolidationError;
use crate::model::{ClusterableMemory, MemoryCluster};

/// Calcula la similitud coseno entre dos vectores normalizados o no normalizados.
pub fn cosine_similarity(v1: &[f32], v2: &[f32]) -> f32 {
    if v1.is_empty() || v1.len() != v2.len() {
        return 0.0;
    }

    let mut dot = 0.0f32;
    let mut norm1 = 0.0f32;
    let mut norm2 = 0.0f32;

    for (a, b) in v1.iter().zip(v2.iter()) {
        dot += a * b;
        norm1 += a * a;
        norm2 += b * b;
    }

    if norm1 <= 0.0 || norm2 <= 0.0 {
        return 0.0;
    }

    dot / (norm1.sqrt() * norm2.sqrt())
}

/// Agrupa memorias afines utilizando similitud vectorial coseno (o heurística contextual si no hay embeddings).
pub fn cluster_memories(
    memories: &[ClusterableMemory],
    threshold: f32,
    min_size: usize,
) -> Result<Vec<MemoryCluster>, ConsolidationError> {
    if !(0.0..=1.0).contains(&threshold) {
        return Err(ConsolidationError::InvalidSimilarityThreshold(threshold));
    }

    if memories.is_empty() {
        return Ok(Vec::new());
    }

    // Particionar primero por proyecto para evitar mezclar dominios no relacionados
    let mut by_project: HashMap<Option<String>, Vec<&ClusterableMemory>> = HashMap::new();
    for mem in memories {
        by_project.entry(mem.project.clone()).or_default().push(mem);
    }

    let mut final_clusters = Vec::new();

    for (project, group) in by_project {
        // Separar memorias con embedding de las que no tienen
        let with_embedding: Vec<&ClusterableMemory> = group
            .iter()
            .filter(|m| m.embedding.is_some())
            .copied()
            .collect();
        let without_embedding: Vec<&ClusterableMemory> = group
            .iter()
            .filter(|m| m.embedding.is_none())
            .copied()
            .collect();

        // 1. Leader-clustering vectorial para memorias con embeddings
        let mut vector_clusters: Vec<(Vec<f32>, Vec<MemoryId>)> = Vec::new();

        for mem in with_embedding {
            let emb = mem.embedding.as_ref().unwrap();
            let mut matched = false;

            for (centroid, cluster_ids) in vector_clusters.iter_mut() {
                let sim = cosine_similarity(centroid, emb);
                if sim >= threshold {
                    cluster_ids.push(mem.id);
                    // Actualizar centroide promediado
                    for (c, e) in centroid.iter_mut().zip(emb.iter()) {
                        *c = (*c + *e) / 2.0;
                    }
                    matched = true;
                    break;
                }
            }

            if !matched {
                vector_clusters.push((emb.clone(), vec![mem.id]));
            }
        }

        // Agregar clusters vectoriales que alcancen el tamaño mínimo
        for (_centroid, ids) in vector_clusters {
            if ids.len() >= min_size {
                final_clusters.push(MemoryCluster::new(project.clone(), ids));
            }
        }

        // 2. Heurística contextual para memorias sin embeddings (por proyecto / palabras clave)
        if !without_embedding.is_empty() {
            let fallback_ids: Vec<MemoryId> = without_embedding.iter().map(|m| m.id).collect();
            if fallback_ids.len() >= min_size {
                final_clusters.push(MemoryCluster::new(project.clone(), fallback_ids));
            }
        }
    }

    Ok(final_clusters)
}
