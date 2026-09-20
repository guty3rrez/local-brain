# 🚀 Local Brain — Hardware Reference Benchmarks

> **Reproducible performance benchmarking specification.**  
> This document details benchmark methodology, latency targets, and measured results across our reference commodity consumer hardware profile.

---

## 💻 1. Reference Hardware Profile

Local Brain is engineered to guarantee fluid, sub-millisecond core logic latencies on commodity hardware:

| Component | Reference Hardware Specification |
| :--- | :--- |
| **CPU** | AMD Ryzen 7 6800H (8 cores, 16 threads @ 3.2 GHz - 4.7 GHz) or equivalent |
| **RAM** | 16 GB DDR5 / DDR4 Dual-Channel |
| **GPU / VRAM** | NVIDIA GeForce RTX 3050 Laptop (4 GB dedicated VRAM) or pure CPU |
| **Storage** | NVMe SSD (PCIe 3.0 / 4.0) |
| **Operating System**| Linux (Kernel 6.x+, Ubuntu / Debian / Arch / Fedora) |
| **Embedding Model** | `nomic-embed-text-v1.5` GGUF (768 dimensions) via local `llama.cpp` |

---

## 🎯 2. Latency Targets & Service Level Objectives (SLOs)

| Operation | Target p50 | Target p95 | Scope / Layer | Status |
| :--- | :---: | :---: | :--- | :--- |
| **Creation & SHA-256 Hash** | `< 2 µs` | `< 10 µs` | Pure in-memory domain | ✅ Benchmarked |
| **Cosine Similarity (768d)** | `< 1 µs` | `< 5 µs` | Pure vector arithmetic | ✅ Benchmarked |
| **Graph Traversal (3 hops)** | `< 50 µs` | `< 500 µs` | Graph traversal (100-node DAG, in-memory) | ✅ Benchmarked |
| **Multidimensional Scoring** | `< 100 ns` | `< 500 ns` | 5-signal weighted fusion | ✅ Benchmarked |
| **Recency Decay Computation** | `< 50 ns` | `< 200 ns` | Half-life / exponential model | ✅ Benchmarked |
| **RRF Fusion (50 candidates)** | `< 15 µs` | `< 50 µs` | Hybrid retrieval pipeline (in-memory) | ✅ Benchmarked |
| **HNSW Vector Search** | `< 5 ms` | `< 10 ms` | PostgreSQL 17 + `pgvector` HNSW | 🎯 Design target — not yet benchmarked |
| **End-to-End Hybrid Pipeline** | `< 40 ms` | `< 100 ms` | FTS + Vectors + Graph + Rerank | 🎯 Design target — not yet benchmarked |

"✅ Benchmarked" rows are covered by `cargo bench --bench hardware_reference` (see §3 below) and run entirely in-memory — no PostgreSQL, pgvector, or llama.cpp round-trip. "🎯 Design target" rows describe what the architecture is intended to sustain, but currently have no automated benchmark exercising the real database and embedding server end-to-end; treat those two numbers as unverified until that harness exists.

---

## 🧪 3. Reproducible Benchmark Execution

The automated suite uses the statistical framework [`criterion`](https://github.com/bheisler/criterion.rs):

### 3.1 Run All Benchmarks
```bash
cargo bench --bench hardware_reference
```

### 3.2 Run a Specific Benchmark
```bash
cargo bench --bench hardware_reference -- rrf
cargo bench --bench hardware_reference -- vector_cosine
```

### 3.3 Fast Verification Test
```bash
cargo bench --bench hardware_reference -- --test
```

---

## 📊 4. Measured Results & Architectural Takeaways

1. **Pure Domain Isolation**: Memory record creation with invariant enforcement and SHA-256 cryptographic hashing takes less than 3 microseconds per record.
2. **Dense Semantic Fidelity (768d)**: Cosine similarity computation in memory executes in sub-microseconds, enabling in-memory reranking of hundreds of candidates before interacting with the persistent database.
3. **Graph Scalability**: Recursive traversal over typed conceptual knowledge graphs scales linearly with edge count, completing 3-hop neighborhood discovery in under 100 microseconds.
4. **Zero Heap Overhead**: Scoring and decay computations operate in-place without heap allocations, guaranteeing zero GC/deallocation pause on critical retrieval paths.

> **Note on HNSW and End-to-End numbers**: the `< 5 ms` (HNSW) and `< 40 ms` (end-to-end pipeline) rows in §2 are roadmap targets, not results from this suite — `hardware_reference.rs` never opens a PostgreSQL connection or calls the embedding server. A real end-to-end benchmark harness (Postgres + pgvector + llama.cpp in the loop) is tracked as future work.
