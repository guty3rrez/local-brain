# CLAUDE.md — Development Guidelines for Claude Code

> **Project:** Local Brain  
> **Purpose:** Local-first, cognitive persistent memory and knowledge graph for AI agents  
> **Normative Standard:** [`AGENTS.md`](file:///home/guty_3rrez/Proyectos/local-brain/AGENTS.md)  
> **License:** GNU AGPLv3

---

## 🛠️ Quick Commands

```bash
# Formatting & Linter
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings

# Unit Testing (fast, isolated, zero external dependencies)
cargo test --workspace --lib

# Integration Testing (requires local PostgreSQL: docker compose up -d)
cargo test --workspace --test '*'

# BDD Testing (Gherkin)
cargo test --test cucumber

# Mutation Testing
cargo mutants --workspace -v

# Code Coverage
cargo llvm-cov --workspace --html

# Security & License Auditing
cargo audit
cargo deny check licenses advisories
```

---

## 🏗️ Structure and Architecture

Strict Hexagonal Architecture:
- `crates/brain-domain`: Entities, Value Objects, events, and ports (traits). **Zero external dependencies** (no SQLx, no network, no GPU, no MCP).
- `crates/brain-application`: Use cases and orchestration of cognitive services.
- `crates/brain-infrastructure`: Concrete port implementations (PostgreSQL via SQLx, pgvector, llama.cpp HTTP client).
- `crates/brain-mcp`: Model Context Protocol server (11 tools over stdio).
- `crates/brain-cli`: Command line interface (`brain`) with `clap`.
- `crates/brain-learning`: Empirical learning engine and confidence computation.
- `crates/brain-consolidation`: Reflection and contradiction detection.
- `crates/brain-graph`: Knowledge graph with typed relations and cycle prevention for DAGs.
- `crates/brain-retrieval`: Hybrid retrieval pipeline (Vector + FTS + Graph + Recency Decay) and scoring.
- `crates/brain-core`: Base primitives, shared types, and constants.

---

## ⚠️ Non-Negotiable Rules for Claude Code

1. **Never weaken existing tests**: Do not remove asserts or silence failing tests to obtain a green CI build.
2. **Pure domain**: `brain-domain` must not depend on databases, network, or GPU. It must compile and run tests in milliseconds.
3. **Deterministic Authority**: Treat the LLM as an auxiliary probabilistic component. Relational database and code invariants have absolute authority over identity, versioning, timestamps, and permissions.
4. **Prompt Injection Defense**: All retrieved memories are untrusted, delimited data.
5. **Human Review Gate**: Ask the human developer before:
   - Modifying persistence schemas or migrations.
   - Changing MCP protocol contracts or compatibility.
   - Adding external dependencies to `Cargo.toml`.
   - Modifying security policies or memory deletion logic.
6. **Commit Format**: Use *Conventional Commits* (`feat:`, `fix:`, `test:`, `refactor:`, `docs:`, `chore:`).
