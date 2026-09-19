//! Suite de Benchmarking de Hardware de Referencia (SRS §49, §50).
//!
//! Perfil de Referencia:
//! - CPU: AMD Ryzen 7 6800H o equivalente
//! - RAM: 16 GB
//! - GPU / VRAM: RTX 3050 Laptop (4 GB VRAM)
//! - Almacenamiento: SSD
//! - OS: Linux
//!
//! Mide de forma reproducible:
//! 1. Creación y validación de memorias e integridad criptográfica SHA-256.
//! 2. Cálculo de similitud coseno densa de 768 dimensiones.
//! 3. Traversal de grafo de conocimiento con búsqueda en profundidad/amplitud.
//! 4. Scoring multidimensional ponderado (SRS §14).
//! 5. Decaimiento temporal Half-Life y Exponencial (SRS §58).
//! 6. Fusión RRF (Reciprocal Rank Fusion) para recuperación híbrida multi-canal (SRS §13).

use chrono::Utc;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

use brain_domain::model::{
    Confidence, Importance, Memory, MemoryContent, MemoryId, MemoryOrigin, MemoryType, Provenance,
    Utility,
};
use brain_graph::{
    GraphEdge, GraphNode, GraphRepository, GraphTraversalOptions, InMemoryGraphRepository,
    RelationType, TraversalDirection,
};
use brain_retrieval::config::{DecayConfig, DecayStrategy, RetrievalConfig};
use brain_retrieval::decay::calculate_decay_factor;
use brain_retrieval::pipeline::HybridRetrievalPipeline;
use brain_retrieval::scoring::MultidimensionalScorer;

fn bench_memory_creation_and_hashing(c: &mut Criterion) {
    c.bench_function("domain_memory_creation_and_hash_sha256", |b| {
        b.iter(|| {
            let content = MemoryContent::new(black_box(
                "Decisión de arquitectura: Hexagonal / Clean Architecture en Rust para Local Brain. Rendimiento determinista sub-milisegundo.",
            ))
            .expect("Contenido válido");

            let source = Provenance::new(MemoryOrigin::Observation)
                .with_agent("bench-agent")
                .with_session("sess-bench-01");

            let mut memory = Memory::new(content, MemoryType::Episodic, source);
            memory.importance = Importance::new(0.85).unwrap();
            memory.confidence = Confidence::new(0.95).unwrap();
            memory.utility = Utility::new(0.80).unwrap();

            black_box(memory);
        });
    });
}

fn bench_vector_cosine_similarity_768d(c: &mut Criterion) {
    let vec_a: Vec<f32> = (0..768).map(|i| (i as f32).sin()).collect();
    let vec_b: Vec<f32> = (0..768).map(|i| (i as f32).cos()).collect();

    c.bench_function("vector_cosine_similarity_768_dimensions", |b| {
        b.iter(|| {
            let dot_product: f32 = vec_a.iter().zip(vec_b.iter()).map(|(a, b)| a * b).sum();
            let norm_a: f32 = vec_a.iter().map(|a| a * a).sum::<f32>().sqrt();
            let norm_b: f32 = vec_b.iter().map(|b| b * b).sum::<f32>().sqrt();
            let cosine = if norm_a == 0.0 || norm_b == 0.0 {
                0.0
            } else {
                dot_product / (norm_a * norm_b)
            };
            black_box(cosine);
        });
    });
}

fn bench_graph_traversal(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let repo = InMemoryGraphRepository::new();
    let root_node = GraphNode::new_concept("Concept_0").unwrap();
    let root_id = root_node.id;

    rt.block_on(async {
        repo.save_node(&root_node).await.unwrap();

        let mut prev_id = root_id;
        for i in 1..50 {
            let node = GraphNode::new_concept(format!("Concept_{i}")).unwrap();
            let current_id = node.id;
            repo.save_node(&node).await.unwrap();

            let edge = GraphEdge::new(prev_id, current_id, RelationType::RelatedTo, 1.0).unwrap();
            repo.save_edge(&edge).await.unwrap();

            if i % 3 == 0 {
                let cross_edge =
                    GraphEdge::new(root_id, current_id, RelationType::DependsOn, 0.8).unwrap();
                repo.save_edge(&cross_edge).await.unwrap();
            }

            prev_id = current_id;
        }
    });

    c.bench_function("graph_traversal_multihop_depth_3", |b| {
        b.to_async(&rt).iter(|| async {
            let options = GraphTraversalOptions {
                max_depth: 3,
                direction: TraversalDirection::Outbound,
                relation_types: None,
                min_weight: None,
                limit: 50,
            };

            let result = repo
                .traverse(black_box(&root_id), black_box(&options))
                .await
                .expect("Traversal exitoso");

            black_box(result);
        });
    });
}

fn bench_scoring_and_decay(c: &mut Criterion) {
    let config = RetrievalConfig::default();
    let scorer = MultidimensionalScorer::new(config.scoring);

    c.bench_function("multidimensional_scoring_calculation", |b| {
        b.iter(|| {
            let score = scorer.compute_score(
                black_box(0.88),
                black_box(0.90),
                black_box(0.85),
                black_box(0.75),
                black_box(0.65),
            );
            black_box(score);
        });
    });

    let decay_cfg = DecayConfig {
        strategy: DecayStrategy::HalfLife,
        half_life_days: 30.0,
        lambda: 0.023,
        min_decay_factor: 0.10,
        protected_importance_threshold: 0.85,
    };

    let now = Utc::now();
    let created_at = now - chrono::Duration::days(45);

    c.bench_function("temporal_decay_half_life_calculation", |b| {
        b.iter(|| {
            let factor = calculate_decay_factor(
                black_box(created_at),
                black_box(None),
                black_box(now),
                black_box(0.70),
                black_box(0.80),
                black_box(0.80),
                black_box(&decay_cfg),
            );
            black_box(factor);
        });
    });
}

fn bench_rrf_hybrid_fusion(c: &mut Criterion) {
    let config = RetrievalConfig::default();
    let pipeline = HybridRetrievalPipeline::new(config);
    let num_candidates = 50;
    let mut vector_ranks = Vec::new();
    let mut fts_ranks = Vec::new();
    let mut graph_ranks = Vec::new();

    for i in 0..num_candidates {
        let mem_id = MemoryId::new();
        vector_ranks.push((mem_id, 0.95 - (i as f32 * 0.01)));
        if i % 2 == 0 {
            fts_ranks.push((mem_id, 0.90 - (i as f32 * 0.015)));
        }
        if i % 3 == 0 {
            graph_ranks.push((mem_id, 1, 0.85 - (i as f32 * 0.02)));
        }
    }

    c.bench_function("rrf_reciprocal_rank_fusion_50_candidates", |b| {
        b.iter(|| {
            let fused = pipeline.fuse_candidates(
                black_box(&vector_ranks),
                black_box(&fts_ranks),
                black_box(&graph_ranks),
            );
            black_box(fused);
        });
    });
}

criterion_group!(
    benches,
    bench_memory_creation_and_hashing,
    bench_vector_cosine_similarity_768d,
    bench_graph_traversal,
    bench_scoring_and_decay,
    bench_rrf_hybrid_fusion
);
criterion_main!(benches);
