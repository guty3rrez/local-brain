# AGENTS.md — Protocol and Standards for AI Agents

> **Normative document for AI coding agents (Antigravity, Claude Code, Codex, Cursor, Cline, Windsurf, Roo Code, etc.) collaborating on the Local Brain repository.**  
> Every agent reading, generating, modifying, or auditing code in this repository must strictly adhere to the guidelines and architectural principles established here.

🌐 **English** | [Español](AGENTS.es.md)

---

## 🧭 1. Fundamental System Principles

### 1.1 The Golden Rule of Memory
> **Do not build a brain that simply remembers everything.**  
> Build a system that knows: **what to remember, why to remember it, when to recall it, how reliable it is, where it came from, how it is related, when it became invalid, and why it came to believe it.**

### 1.2 Local-First & Provider Independence
- The core of Local Brain does not depend on any commercial cloud AI vendor (OpenAI, Anthropic, Google, etc.).
- All critical functionality must run 100% offline with local models (`llama.cpp`) and local relational/vector persistence (`PostgreSQL + pgvector`).
- Introducing outbound external network calls or collecting telemetry without an explicit configuration directive is strictly prohibited.

### 1.3 Do Not Blindly Trust the LLM (Deterministic Authority)
- The LLM is a **probabilistic**, auxiliary component.
- The relational database and deterministic code rules maintain **absolute authority** over:
  - Identity and unique identifiers (monotonic UUIDv7).
  - Relations and referential integrity in the Knowledge Graph.
  - History, timestamps, and data provenance.
  - Memory states, expiration policies, and access permissions.
- Retrieved memory content must always be treated as **untrusted data**, never as privileged system instructions (prompt injection mitigation via isolation delimiters).

### 1.4 Hexagonal Architecture and Pure Domain
```text
Domain (brain-domain)
   ↑ (implements ports)
Application (brain-application)
   ↑ (invokes use cases)
Adapters / Infrastructure (brain-infrastructure, brain-mcp, brain-cli)
```
- **Unbreakable Rule**: Domain logic in `brain-domain` **must have zero dependencies on PostgreSQL, SQLx, llama.cpp, HTTP, GPU, or MCP**. The domain must compile and test purely in milliseconds without external infrastructure.

---

## 🚫 2. Strict Prohibitions for Agents

AI agents are **STRICTLY PROHIBITED** from:
1. **Modifying or disabling existing tests** to force green CI builds. If a test fails, fix the implementation or explicitly justify a change in the test requirements.
2. **Introducing new dependencies in `Cargo.toml`** without technical justification documented in the PR or commit.
3. **Modifying database schemas** destructively or without a versioned, idempotent SQLx migration.
4. **Weakening or removing security controls**, input validations, memory limits, or prompt injection sanitization.
5. **Hardcoding secrets**, static production URLs, or credentials in source code.
6. **Sending telemetry or memory data** outside the local environment without explicit configuration.

---

## 🚦 3. Human Review Gate

The following actions **require pausing execution and requesting explicit confirmation from the human developer**:
- [ ] Changes to crate architecture or Clean Architecture boundaries.
- [ ] Modifications to the MCP permission model or security controls.
- [ ] Changes to project licensing or distribution terms (AGPL-3.0).
- [ ] Structural modifications to PostgreSQL / pgvector tables or extensions.
- [ ] Breaking changes to MCP protocol compatibility with existing clients.
- [ ] Introducing critical dependencies or licenses incompatible with AGPL-3.0.
- [ ] Bulk deletion or non-reversible purging of memory records.

---

## 🧪 4. Mandatory Testing Arsenal for Agents

Before declaring a task or PR complete, the agent must locally run and report:

| Layer | Command | Goal |
| :--- | :--- | :--- |
| **Format** | `cargo fmt --all -- --check` | Strict compliance with official Rust style |
| **Linter** | `cargo clippy --workspace --all-targets -- -D warnings` | Zero warnings permitted |
| **Unit Tests** | `cargo test --workspace --lib` | Pure domain logic and edge cases |
| **BDD** | `cargo test --test cucumber` | Living specifications in Gherkin (`tests/features/`) |
| **Integration** | `cargo test --workspace --test '*'` | Verification with PostgreSQL + pgvector and MCP |
| **Security** | `cargo audit` | Zero known vulnerabilities in dependencies |
| **Licenses** | `cargo deny check licenses advisories` | Strict compliance with AGPLv3 |

---

## 🔄 5. Git Workflow and Delivery Format

### 5.1 Branch Naming
- Features: `feature/<descriptive-name>`
- Fixes: `fix/<descriptive-name>`
- Refactors: `refactor/<descriptive-name>`
- Security: `security/<descriptive-name>`
- Docs: `docs/<descriptive-name>`

### 5.2 Commit Format (Conventional Commits)
```text
feat(domain): add MemoryContent value object with invariant validation
fix(mcp): prevent prompt injection via delimiter wrapping
test(retrieval): add mutation-tested edge cases for score calculation
```

### 5.3 Mandatory PR / Changes Description
```markdown
### Purpose
[What problem or requirement does this solve?]

### Changes Made
[Concise breakdown of modified modules]

### Risks & Mitigations
[Potential impact on performance, concurrency, or compatibility]

### Testing Arsenal Executed
- [x] cargo fmt & clippy
- [x] cargo test (unit & integration)
- [x] BDD scenarios

### Human Review Gate
[Requires human review? Yes/No and rationale]
```
