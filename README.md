# 🧠 Local Brain

> **Structured long-term memory infrastructure for AI coding agents, delivered over the open Model Context Protocol (MCP). Think of it as a local cognitive memory layer.**

🌐 **English** | [Español](README.es.md)

[![Version: v1.0.0](https://img.shields.io/badge/Version-v1.0.0-blue.svg)](https://github.com/guty3rrez/local-brain/releases)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Rust: 1.85+](https://img.shields.io/badge/Rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![CI: Passing](https://img.shields.io/badge/CI-Passing-brightgreen.svg)](https://github.com/guty3rrez/local-brain/actions)
[![Protocol: Model Context Protocol (MCP)](https://img.shields.io/badge/Protocol-MCP_Standard-blueviolet.svg)](https://modelcontextprotocol.io/)
[![Architecture: Hexagonal](https://img.shields.io/badge/Architecture-Clean_%2F_Hexagonal-lightgrey.svg)](#-system-architecture)
[![Privacy: 100% Offline](https://img.shields.io/badge/Privacy-100%25_Offline-success.svg)](#-privacy--local-first-security)

---

## ⚡ The Problem: AI Agent Amnesia

Modern AI coding agents (Claude Code, Cursor, Antigravity, Codex, Windsurf, Cline) excel at solving complex problems within a single context window. But once that context window ends, so does everything they learned in it — by default, nothing carries over to the next session:
- **Agents repeat the exact same errors** they already spent hours fixing yesterday.
- **They ignore hard-earned architectural conventions** established in earlier turns.
- **They cannot explain why a choice was made** (*"why did we migrate from library A to library B last week?"*).

### Why Plain Vector RAG Falls Short for Agent Memory
Most existing tools try to solve persistence by dumping raw text or chat transcripts into a vector database:
```text
Raw text chunk → embedding → vector database → cosine similarity search
```
That pipeline treats all text as flat, undifferentiated chunks: it has no concept of causality, no way to distinguish an unverified hypothesis from an empirical fact, no model of step-by-step procedures, and no mechanism to flag when two retrieved chunks contradict each other. Local Brain addresses each of those gaps directly — see the comparison below.

---

## 💡 The Solution: Local Brain

**Local Brain** is an open-source, local-first memory infrastructure that equips AI agents with **structured long-term memory** through the open **Model Context Protocol (MCP)** standard, ensuring 100% privacy and full data sovereignty on the user's hardware.

Local Brain does not try to be another agent. It is infrastructure that any MCP-capable agent can consume: Claude Code, Cursor, Codex, a custom in-house agent, or even a traditional application talking to the `brain` CLI directly. One PostgreSQL-backed memory store, shared across whichever tools you point at it — see the [System Architecture](#-system-architecture) diagram below for how the pieces fit together.

### 🧭 The Golden Rule of Memory
> **Do not build a brain that simply remembers everything.**  
> Build a system that knows: **what to remember, why to remember it, when to recall it, how reliable it is, where it came from, how it is related, when it became invalid, and why it came to believe it.**

---

## 📊 Traditional Vector RAG vs. Local Brain

| Capability | Traditional Vector RAG | Local Brain v1.0 |
| :--- | :---: | :---: |
| **Data Model** | Flat unstructured text chunks | **5 Specialized Cognitive Types** (Working, Episodic, Semantic, Procedural, Associative) |
| **Relational Reasoning** | None (only distance in latent space) | **Typed Knowledge Graph** (10 canonical relations with cycle-safe DAGs) |
| **Causality & Experience** | Ignored | Formal episodic triad: **Context ➔ Action ➔ Outcome** |
| **Empirical Validity** | Assumes all stored text is true | **Empirical Learning Engine**: Fact vs. Belief with evidence-weighted confidence updates |
| **Contradiction Detection** | None (surfaces opposing text randomly) | **Reflection Engine**: Deterministic conflict clustering and resolution |
| **Retrieval Strategy** | Vector similarity only | **Hybrid Fusion**: Vectors (768d) + Lexical Full-Text Search + Graph + Recency Decay |
| **Explainability** | Black-box scores | **Mathematically Explainable** (`--explain` with explicit signal breakdown) |
| **Prompt Injection Defense** | Vulnerable (retrieved text can hijack prompt) | **Strict Isolation**: Wrapped in `<untrusted_memory_content>` delimiters |
| **Privacy & Sovereignty** | Often tied to proprietary cloud APIs | **100% Offline, Zero Telemetry**, local PostgreSQL + `llama.cpp` |

---

## 🚀 Quickstart (Zero to Hero in 2 Minutes)

Local Brain includes automated setup for all auxiliary local services (PostgreSQL 17 with `pgvector` and local `llama.cpp` server running the high-fidelity `nomic-embed-text-v1.5` 768-dimensional model).

### Option A: Automated Onboarding Wizard (Recommended)
```bash
# 1. Clone the repository
git clone https://github.com/guty3rrez/local-brain.git
cd local-brain

# 2. Run the interactive setup script
./scripts/quickstart.sh
```

The script automatically downloads the quantized GGUF model (~140 MB), spins up background Docker containers, applies database migrations (`brain init`), verifies system health (`brain doctor`), and outputs ready-to-use MCP configuration blocks.

### Option B: Manual Setup
```bash
# 1. Start local services
docker compose up -d

# 2. Build and install the CLI
cargo build --release --bin brain
sudo install -m 755 target/release/brain /usr/local/bin/brain

# 3. Initialize the database and run migrations
brain init

# 4. Verify system diagnostics
brain doctor
```

---

## 🤖 Connect Your AI Agents via MCP

Local Brain implements the **Model Context Protocol (MCP)** over `stdio`. Plug it into your favorite agent in seconds:

### 1. Claude Desktop
Add to your `claude_desktop_config.json`:
```json
{
  "mcpServers": {
    "local-brain": {
      "command": "brain",
      "args": [
        "--database-url", "postgres://localbrain:localbrain_secret@localhost:5433/local_brain",
        "--embedding-url", "http://127.0.0.1:8081/embedding",
        "mcp"
      ]
    }
  }
}
```

### 2. Claude Code CLI
Add the server with a single terminal command:
```bash
claude mcp add local-brain brain -- --database-url postgres://localbrain:localbrain_secret@localhost:5433/local_brain --embedding-url http://127.0.0.1:8081/embedding mcp
```

### 3. Cursor & Windsurf
In `.cursor/mcp.json` or your MCP Settings:
```json
{
  "mcpServers": {
    "local-brain": {
      "command": "brain",
      "args": [
        "--database-url", "postgres://localbrain:localbrain_secret@localhost:5433/local_brain",
        "--embedding-url", "http://127.0.0.1:8081/embedding",
        "mcp"
      ]
    }
  }
}
```

### 4. Cline / Roo Code (VS Code)
In `cline_mcp_settings.json`:
```json
{
  "mcpServers": {
    "local-brain": {
      "command": "brain",
      "args": [
        "--database-url", "postgres://localbrain:localbrain_secret@localhost:5433/local_brain",
        "--embedding-url", "http://127.0.0.1:8081/embedding",
        "mcp"
      ],
      "disabled": false,
      "autoApprove": [
        "brain_remember",
        "brain_recall",
        "brain_retrieve",
        "brain_relate",
        "brain_graph",
        "brain_learn",
        "brain_explain"
      ]
    }
  }
}
```

---

## 🧠 Core Features

### 1. Five Specialized Cognitive Memory Types
Memory is modeled with domain-level invariants in pure Rust:
1. **Working Memory (`working`)**: Ephemeral state scoped to an active session (`session_id`) with a TTL (`ttl_seconds`). Automatically purged or archived when the session concludes.
2. **Episodic Memory (`episodic`)**: Captures situated experience using the strict triad:
   - **Context**: Problem or situation encountered.
   - **Action**: Decision or command applied.
   - **Outcome**: Observed empirical result (error logs, latency, behavior).
3. **Semantic Memory (`semantic`)**: Generalized statements and principles with empirical confidence (`confidence` $\in [0.0, 1.0]$). If confidence $\ge 0.8$, it **strictly requires** supporting evidence UUIDs.
4. **Procedural Memory (`procedural`)**: Ordered sequences of technical steps and executable commands (`steps`) aligned to a goal (`goal`).
5. **Associative Memory (`associative`)**: Normalized conceptual triples (`source` ➔ `predicate` ➔ `target`) with associative link strength.

### 2. Cycle-Safe Knowledge Graph
Connects memories and entities through 10 canonical relations:
- `RELATED_TO`: General bidirectional conceptual link.
- `USED_IN`: Component utilized in a module or system.
- `CAUSED_BY`: Causal provenance.
- `SOLVES`: Proven technical resolution to an issue.
- `CONTRADICTS`: Incompatibility between architectural decisions.
- `SUPERSEDES`: Newer decision superseding an obsolete convention.
- `DERIVED_FROM`: Knowledge synthesized from prior experiences.
- `DEPENDS_ON`: Architectural or environment dependency.
- `PREFERS`: Explicit positive guideline (e.g., prefer PostgreSQL over SQLite).
- `AVOID`: Explicit anti-pattern guideline (e.g., avoid blocking I/O in async runtimes).

> **DAG Cycle Prevention**: Causal and dependency edges enforce real-time cycle detection, preventing circular reasoning loops in autonomous agents.

### 3. Empirical Learning & Evidence-Weighted Confidence
The system enforces a strict boundary between a **Factual Observation** and a **Generalized Belief**:
- Incoming evidence dynamically updates the confidence score of candidate beliefs through a heuristic evidence-accumulation model — not formal Bayesian inference.
- Supports typed evidence sources (`Human`, `ToolExecution`, `DirectObservation`, `AgentHypothesis`).
- Explicit human validation immediately elevates a belief to maximum certainty.

### 4. Asynchronous Reflection & Contradiction Resolution
The `brain reflect` engine clusters recent episodic experiences, identifies emerging patterns, and automatically flags cognitive contradictions (`CONFLICT`), allowing humans or agents to resolve discrepancies cleanly.

### 5. Multidimensional Hybrid Retrieval Engine
`brain_retrieve` fuses three parallel retrieval channels via **Reciprocal Rank Fusion (RRF)**:
1. **Dense Vector Channel**: 768 dimensions with cosine distance over HNSW indexes in `pgvector`.
2. **Lexical Full-Text Search**: Natural language queries parsed with PostgreSQL `tsvector` and GIN indexing.
3. **Knowledge Graph Channel**: Multi-hop recursive neighborhood expansion.

Final ranking is computed with **Multidimensional Scoring**:
$$Score = w_{sem} \cdot S_{sim} + w_{imp} \cdot S_{imp} + w_{conf} \cdot S_{conf} + w_{util} \cdot S_{util} - w_{pen} \cdot P$$
Modulated by an **Exponential Half-Life Recency Decay**:
$$D(t) = 2^{-t / T_{1/2}}$$
*(Memories with intrinsic importance $\ge 0.85$ are shielded from decay).*

---

## 🎬 Demo: See It Work

A minimal, reproducible round-trip against a real running instance — store two memories, then ask a question:

```bash
brain remember "PostgreSQL is our production database" --type semantic --project demo --confidence 0.9
brain remember "We migrated from SQLite because concurrent writes caused problems" --type episodic --project demo
brain retrieve "Why did we migrate to PostgreSQL?" --project demo --explain
```

Real output from `brain retrieve --explain` (CLI messages are currently Spanish-only regardless of `$LANG` — this is verbatim, not translated):

```text
🧠 Local Brain — Recuperación Híbrida Avanzada (SRS §13, §14, §56)
─────────────────────────────────────────────────────────────────────────────
Consulta:           "Why did we migrate to PostgreSQL?"
Candidatos:         40 únicos (Vector: 40, FTS: 0, Grafo: 0)
Recuerdos devueltos: 1
─────────────────────────────────────────────────────────────────────────────

[1] Score: 0.6169 | ID: 01a0bd17-fe1d-77d8-add1-a64b9d4a525e | Tipo: Semantic
    Proyecto:    demo
    Factores:    Similitud: 0.73 | Importancia: 0.50 | Confianza: 0.90 | Recencia: 1.00
    Explicación: Score: 0.6169 [Similitud: 0.73 | Importancia: 0.50 | Confianza: 0.90 | Utilidad: 0.50 | Recencia: 1.00]
    Señales:     Alta confianza empírica validada
    Contenido:   PostgreSQL is our production database
```

That single result is itself informative: of the two memories stored, only the `semantic` one about PostgreSQL scored high enough to surface for this query — the `episodic` SQLite migration memory ranked below it on vector similarity for this particular phrasing. The `--explain` breakdown shows exactly why (similarity, importance, confidence, recency), rather than returning an unexplained black-box score.

---

## 💻 CLI Command Reference (`brain`)

```bash
brain [FLAGS] <SUBCOMMAND>
```

| Subcommand | Purpose | Example |
| :--- | :--- | :--- |
| `init` | Initialize tables and apply SQLx migrations in PostgreSQL | `brain init` |
| `doctor` | Diagnose health of PostgreSQL, pgvector, and llama.cpp | `brain doctor --verbose` |
| `status` | View system metrics and memory distribution | `brain status` |
| `remember` | Store a memory with cognitive specialization | `brain remember "Context..." --type episodic --project app` |
| `recall` | Semantic vector search (768d) or ID lookup | `brain recall -q "database migration pattern"` |
| `retrieve` | Hybrid retrieval (Vector + FTS + Graph + Decay Scoring) | `brain retrieve "auth security" --explain` |
| `relate` | Create a typed edge in the Knowledge Graph | `brain relate Rust C++ --type PREFERS -w 0.9` |
| `graph` | Traverse recursive graph neighborhoods | `brain graph Rust --depth 2` |
| `learn` | Record an observation or belief with evidence | `brain learn "HNSW index reduces latency" -e "local bench"` |
| `explain` | Explain the empirical foundation of a belief | `brain explain "why do we use Rust?"` |
| `reflect` | Run offline consolidation and clustering | `brain reflect --project app` |
| `conflicts` | List and resolve cognitive contradictions | `brain conflicts list` |
| `backup` | Export a cryptographic backup in JSONL or SQL | `brain backup -o backup.jsonl` |
| `restore` | Restore memory state with checksum validation | `brain restore -i backup.jsonl` |
| `mcp` | Start Model Context Protocol server over stdio | `brain mcp` |

---

## 🛠️ MCP Tools Catalog for Agents

Local Brain exposes 11 standard MCP tools over `stdio`:

1. **`brain_remember`**: Store memories with rich metadata and cognitive specialization.
2. **`brain_recall`**: Rapid retrieval via vector similarity or attribute filters.
3. **`brain_search`**: Multi-criteria search with date ranges (ISO 8601) and importance filters.
4. **`brain_retrieve`**: Full hybrid retrieval (Vector + FTS + Graph) with decay scoring and explainability.
5. **`brain_relate`**: Link entities in the Knowledge Graph with 10 typed relations and cycle-safe DAGs.
6. **`brain_graph`**: Traverse and explore multi-hop graph neighborhoods.
7. **`brain_learn`**: Record observations vs. candidate beliefs with evidence sources.
8. **`brain_explain`**: Unpack why the system believes a statement with evidence history.
9. **`brain_consolidate`**: Trigger asynchronous reflection and hypothesis synthesis.
10. **`brain_forget`**: Safe logical soft-delete requiring explicit confirmation (`confirm: true`).
11. **`brain_session_end`**: Conclude an active session and expire transient working memories.

---

## 📦 Official Agent Skill (`local-brain`)

To help autonomous AI agents use Local Brain without prompt overhead, this repository includes an **Official Agent Skill**:

- Main Playbook: [`.agents/skills/local-brain/SKILL.md`](file:///home/guty_3rrez/Proyectos/local-brain/.agents/skills/local-brain/SKILL.md)
- Tools Catalog: [`mcp-tools.md`](file:///home/guty_3rrez/Proyectos/local-brain/.agents/skills/local-brain/references/mcp-tools.md)
- Cognitive Types Guide: [`memory-types.md`](file:///home/guty_3rrez/Proyectos/local-brain/.agents/skills/local-brain/references/memory-types.md)
- Interaction Patterns: [`workflow-patterns.md`](file:///home/guty_3rrez/Proyectos/local-brain/.agents/skills/local-brain/references/workflow-patterns.md)

To install globally for Antigravity or other agent runtimes:
```bash
mkdir -p ~/.agents/skills/local-brain
cp -r .agents/skills/local-brain/* ~/.agents/skills/local-brain/
```

---

## 🏛️ System Architecture

Built following strict **Clean Architecture / Hexagonal Architecture** principles:

```mermaid
flowchart TD
    subgraph Clients["Agents & Interfaces"]
        CC["Claude Code"]
        CU["Cursor / Windsurf"]
        AG["Antigravity / Codex"]
        CLI["brain CLI (clap)"]
    end

    subgraph Entrypoints["Primary Adapters (Inbound)"]
        MCP["brain-mcp (MCP Server stdio)"]
        CMD["brain-cli (Commands & Flags)"]
    end

    subgraph Application["Application Layer (brain-application)"]
        UC["Use Cases (Remember, Recall, Retrieve, Relate, Learn, Reflect)"]
    end

    subgraph Domain["Pure Domain Layer (brain-domain)"]
        DOM["Invariant Entities, Cognitive Types, Scoring Formulas, Traits"]
    end

    subgraph SpecializedCrates["Specialized Engines"]
        GRA["brain-graph (Graph & DAG)"]
        LEA["brain-learning (Evidence & Confidence)"]
        CON["brain-consolidation (Reflection & Conflicts)"]
        RET["brain-retrieval (FTS + Vector + Graph + Decay)"]
    end

    subgraph Infrastructure["Secondary Adapters (brain-infrastructure)"]
        PG[("PostgreSQL 17 + pgvector (SQLx)")]
        LLM["llama.cpp (Local 768d Inference)"]
    end

    Clients --> Entrypoints
    Entrypoints --> Application
    Application --> Domain
    Application --> SpecializedCrates
    SpecializedCrates --> Domain
    Infrastructure -.->|Implements ports| Domain
```

- **Pure Domain**: `brain-domain` compiles in milliseconds with **zero dependencies** on databases, network, GPU, or external frameworks.

---

## 💻 Hardware Requirements & Benchmarks

Local Brain is optimized to deliver sub-millisecond core logic latencies on commodity consumer hardware:

| Component | Reference Benchmark Spec |
| :--- | :--- |
| **CPU** | AMD Ryzen 7 6800H (8 cores, 16 threads) or equivalent x86_64 / ARM64 |
| **RAM** | 16 GB DDR4/DDR5 |
| **GPU / VRAM** | NVIDIA GeForce RTX 3050 Laptop (4 GB VRAM) or pure CPU |
| **Storage** | NVMe SSD |
| **Embedding Model** | `nomic-embed-text-v1.5` Q8_0 (768 dimensions, ~140 MB) |

### Latencies

Full methodology and reproduction commands: [`docs/BENCHMARKS.md`](file:///home/guty_3rrez/Proyectos/local-brain/docs/BENCHMARKS.md).

| Operation | Latency | Status |
| :--- | :---: | :--- |
| Domain Invariant Validation & SHA-256 | `< 3 µs` | ✅ Benchmarked (`cargo bench --bench hardware_reference`) |
| In-Memory Cosine Similarity | `< 1 µs` | ✅ Benchmarked |
| Multidimensional Scoring with Decay | `< 500 ns` | ✅ Benchmarked |
| Graph Neighborhood Traversal (3 hops, in-memory) | `< 100 µs` | ✅ Benchmarked |
| PostgreSQL HNSW Vector Search | `< 5 ms` | 🎯 Design target — no automated end-to-end benchmark yet |
| End-to-End Hybrid Retrieval Pipeline | `< 40 ms` | 🎯 Design target — no automated end-to-end benchmark yet |

The ✅ rows are measured by the criterion suite in `crates/brain-retrieval/benches/hardware_reference.rs`, which exercises pure in-memory logic (no PostgreSQL, pgvector, or llama.cpp round-trip). The 🎯 rows describe what the architecture is designed to sustain, but aren't yet covered by a reproducible end-to-end benchmark against a real database and embedding server — that's tracked as follow-up work, not a claim you can currently verify yourself.

---

## 🛡️ Privacy & Local-First Security

- **100% Offline by Design**: Local Brain rejects outbound external internet connections (`--offline`).
- **Zero Telemetry**: No project details or memory content ever leave your machine.
- **Prompt Injection Defense**: Retrieved memories are wrapped in strict untrusted data boundaries (`<untrusted_memory_content>`), preventing malicious historical text from hijacking agent instructions.
- **Safe Soft Deletes**: Memories are marked logically and require explicit confirmation (`confirm: true`) to delete.
- **Cryptographic Portability**: Export your entire brain to open JSONL or SQL backups with SHA-256 verification.

---

## 🤝 Contributing

We warmly welcome contributions from human engineers and autonomous AI agents:

1. **Review the Guidelines**:
   - For human developers: [`CONTRIBUTING.md`](file:///home/guty_3rrez/Proyectos/local-brain/CONTRIBUTING.md) ([Español](file:///home/guty_3rrez/Proyectos/local-brain/CONTRIBUTING.es.md)).
   - For AI agents: [`AGENTS.md`](file:///home/guty_3rrez/Proyectos/local-brain/AGENTS.md) ([Español](file:///home/guty_3rrez/Proyectos/local-brain/AGENTS.es.md)).
   - For Claude Code: [`CLAUDE.md`](file:///home/guty_3rrez/Proyectos/local-brain/CLAUDE.md).
2. **Explore Open Issues**: Check [GitHub Issues & Milestones](https://github.com/guty3rrez/local-brain/issues).
3. **Architecture Decision Records**: Read technical rationale in [`docs/adr/`](file:///home/guty_3rrez/Proyectos/local-brain/docs/adr/).

---

## 📄 License

This project is licensed under the **GNU Affero General Public License v3.0 (AGPL-3.0-or-later)**.

> [!NOTE]
> AGPLv3 is a **strong copyleft** license ensuring that Local Brain and any derivative works or network-hosted integrations remain **permanently free and open-source software**. See [`LICENSE`](file:///home/guty_3rrez/Proyectos/local-brain/LICENSE) for complete terms.
