# Agent Workflow Patterns — Local Brain

Recommended interaction patterns for autonomous AI agents integrating persistent cognitive memory.

---

## Pattern 1: Session Start & Context Ingestion
Upon starting work in a repository or feature, query historical knowledge to avoid repeating past mistakes or violating conventions.

```json
// Step 1: Hybrid retrieval for project conventions
{
  "name": "brain_retrieve",
  "arguments": {
    "query": "architectural conventions decisions and troubleshooting patterns",
    "project": "local-brain",
    "limit": 5,
    "min_score": 0.4
  }
}
```

If key technologies appear in retrieved memories, inspect the Knowledge Graph for explicit preference rules:
```json
// Step 2: Query relationships for a specific technology
{
  "name": "brain_graph",
  "arguments": {
    "node": "PostgreSQL",
    "depth": 1,
    "direction": "both"
  }
}
```

---

## Pattern 2: Logging a Technical Solution (Episodic Memory)
When resolving an intricate bug or discovering a non-trivial fix, record the episodic experience so that other agents (or yourself in future sessions) can benefit:

```json
{
  "name": "brain_remember",
  "arguments": {
    "project": "local-brain",
    "agent": "claude-code",
    "memory_type": "episodic",
    "importance": 0.85,
    "confidence": 0.90,
    "context": "CI compilation failed in minimal Alpine container due to missing glibc symbols",
    "action": "Migrated build target to x86_64-unknown-linux-musl and installed musl-tools on the runner",
    "outcome": "Binary compiled completely static with zero dynamic runtime dependencies, passing all CI checks"
  }
}
```

Link the resolution explicitly in the Knowledge Graph:
```json
{
  "name": "brain_relate",
  "arguments": {
    "source": "x86_64-unknown-linux-musl",
    "target": "Alpine-Linux-CI",
    "relation": "SOLVES",
    "weight": 0.95,
    "context": "Resolves glibc dynamic symbol incompatibility on minimal Alpine environments"
  }
}
```

---

## Pattern 3: Empirical Learning Cycle (Observation vs. Belief)
When discovering a performance pattern that is not yet an established ground truth, log it as an empirical observation with moderate confidence:

```json
{
  "name": "brain_learn",
  "arguments": {
    "statement": "HNSW indexing with m=16 and ef_construction=64 decreases RAM usage by 35% without degrading recall in local benchmarks",
    "evidence": "Benchmark execution with 50,000 synthetic vectors on AMD Ryzen 7 hardware",
    "source_type": "tool_execution",
    "domain": "vector-search",
    "agent": "antigravity",
    "is_supporting": true
  }
}
```

Later, when asking why a specific configuration was chosen:
```json
{
  "name": "brain_explain",
  "arguments": {
    "query": "Why do we use m=16 in HNSW?",
    "domain": "vector-search"
  }
}
```

---

## Pattern 4: Registering an Operational Procedure
For recurring build, test, or deployment recipes:

```json
{
  "name": "brain_remember",
  "arguments": {
    "project": "local-brain",
    "memory_type": "procedural",
    "content": "Pre-PR local verification protocol",
    "goal": "Guarantee zero linter, format, and test regressions prior to submitting changes",
    "steps": [
      {
        "step_number": 1,
        "description": "Verify official Rust formatting",
        "command": "cargo fmt --all -- --check"
      },
      {
        "step_number": 2,
        "description": "Execute static analysis with zero warnings permitted",
        "command": "cargo clippy --workspace --all-targets -- -D warnings"
      },
      {
        "step_number": 3,
        "description": "Run pure domain unit tests",
        "command": "cargo test --workspace --lib"
      }
    ]
  }
}
```

---

## Pattern 5: Managing Ephemeral Session State (Working Memory)
For ongoing multi-turn tasks that should not pollute long-term history:

```json
// Step 1: Store working memory with TTL (30 minutes = 1800s)
{
  "name": "brain_remember",
  "arguments": {
    "memory_type": "working",
    "session_id": "session-refactor-retrieval-42",
    "content": "Refactoring decay scoring: analyzing impact on pipeline integration tests",
    "ttl_seconds": 1800
  }
}

// Step 2: Clean up at session end
{
  "name": "brain_session_end",
  "arguments": {
    "session_id": "session-refactor-retrieval-42"
  }
}
```
