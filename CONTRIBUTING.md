# Contribution Guidelines for Local Brain

Thank you for your interest in contributing to **Local Brain**!  
This project is open-source under the [GNU AGPLv3](LICENSE) license and is built in public.

🌐 **English** | [Español](CONTRIBUTING.es.md)

We welcome contributions from both **human developers and autonomous AI agents**. To maintain technical rigor, security, and architectural coherence, please follow the guidelines below.

---

## 🧭 How Can I Help?

1. **Implementing Open Issues**: Check [GitHub Issues and Milestones](https://github.com/local-brain/local-brain/issues) for available tasks. Comment on an issue before starting work to avoid duplicate effort.
2. **Writing BDD Scenarios & Test Cases**: Add Gherkin scenarios in `tests/features/` capturing real agent workflows, or expand mutation testing suites with `cargo-mutants`.
3. **Proposing Architectural Decision Records (ADRs)**: Open an Issue using the RFC/ADR template to discuss technology choices, schema designs, or embedding strategies.
4. **Running Hardware Benchmarks**: Help us benchmark across diverse hardware (Apple Silicon, AMD GPUs, NVIDIA GPUs) and report latency and memory footprints.
5. **Reporting Bugs**: Use our bug report issue template if you identify unexpected behavior in retrieval, persistence, or MCP servers.

---

## 🌿 Git Workflow

### 1. Branch Naming
Create branches from `main` using standard conventions:
- `feature/<short-name>` for new features.
- `fix/<short-name>` for bug fixes.
- `refactor/<short-name>` for code improvements without functional changes.
- `docs/<short-name>` for documentation updates.
- `test/<short-name>` for test additions or benchmarks.
- `security/<short-name>` for security patches.

### 2. Commit Messages (Conventional Commits)
We enforce the *Conventional Commits* standard:
```text
<type>(<optional scope>): <concise imperative description>

[optional body explaining the rationale]

[optional footer with issue references or BREAKING CHANGE]
```
Examples:
- `feat(domain): implement memory retention policy and decay logic`
- `fix(postgres): handle connection pool timeout gracefully`
- `test(bdd): add gherkin scenario for contradiction detection`

---

## 🤖 AI-Assisted Contribution Policy

Local Brain is developed in close human-agent collaboration. We actively encourage AI-assisted contributions under the following conditions:

1. **Mandatory Transparency**: If a Pull Request is generated or assisted by an AI agent (Antigravity, Claude Code, Cursor, Codex, etc.), declare it clearly in the PR description.
2. **Compliance with [`AGENTS.md`](AGENTS.md)**: Agent-generated code must adhere strictly to pure domain isolation, zero test weakening, and the Human Review Gate.
3. **Accountability**: The human author of the GitHub account submitting the PR holds ultimate responsibility for the code.

---

## 🧪 Pre-PR Verification Checklist

Before submitting a Pull Request, ensure all the following commands pass locally:

```bash
# 1. Official formatting
cargo fmt --all -- --check

# 2. Static analysis (zero warnings permitted)
cargo clippy --workspace --all-targets -- -D warnings

# 3. Workspace unit tests
cargo test --workspace --lib

# 4. Integration tests
cargo test --workspace --test '*'

# 5. BDD living specifications
cargo test --test cucumber

# 6. Dependency vulnerability audit
cargo audit

# 7. License compatibility check (AGPLv3)
cargo deny check licenses
```
