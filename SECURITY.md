# Security Policy & Threat Model — Local Brain

🌐 **English** | [Español](SECURITY.es.md)

---

## 🔒 Responsible Disclosure Policy

Security and data integrity are paramount. If you discover a security vulnerability in Local Brain, please **DO NOT open a public issue**.

Instead, report vulnerabilities privately by opening a [Security Advisory on GitHub](https://github.com/guty3rrez/local-brain/security/advisories/new) or contacting the maintainers directly.

Please include:
- Technical description of the vulnerability.
- Step-by-step reproduction guide or Proof of Concept (PoC).
- Potential impact on memory confidentiality, integrity, or availability.

We commit to acknowledging reports within 48 hours and coordinating a security fix prior to public disclosure.

---

## 🛡️ Threat Model & Security Controls

Local Brain implements controls mitigating threats specific to AI agents and cognitive memory systems:

| ID | Threat | Description | Mitigation in Local Brain |
| :--- | :--- | :--- | :--- |
| **T-001** | **Prompt Injection** | Memories containing adversarial text attempting to override agent system instructions upon retrieval. | Retrieved memory content is strictly framed inside **untrusted data delimiters** (`<untrusted_memory_content>`), never as system prompts. |
| **T-002** | **Malicious MCP Client** | An unauthorized client attempting privileged destructive commands. | Strict MCP permission tiering (READ, WRITE, MODIFY, DELETE, ADMIN) and stdio process containment. |
| **T-003** | **Memory Poisoning** | Adversarial agents storing falsified memories to skew future agent decisions. | Comprehensive provenance tracking, Bayesian-like empirical confidence scoring, and candidate belief stages. |
| **T-004** | **Unauthorized Deletion** | Accidental or malicious wiping of accumulated knowledge. | Soft deletes by default, immutable audit logs, and explicit confirmation (`confirm: true`) for destructive commands. |
| **T-005** | **Credential Leakage** | Inadvertent persistence of API keys, tokens, or passwords in memory. | Automated regex secret scanning filters before persisting any memory payload. |
| **T-006** | **Malicious Dependency** | Compromised supply chain crate introduced in workspace. | Enforced `cargo audit` and `cargo deny check` in continuous integration. |
| **T-007** | **Database Compromise** | Unauthorized access to PostgreSQL or pgvector data. | Least-privilege database connections, encryption at rest support, and cryptographic JSONL/SQL backups. |
| **T-008** | **Supply-Chain Model Attack** | Tampered model weights or malicious runtime binaries. | SHA-256 hash verification of quantized GGUF weights and reproducible builds. |
| **T-009** | **Agent Hallucination** | LLM-generated false claims treated as ground truth. | Strict separation of **Factual Observations** and **Candidate Beliefs**; LLMs lack direct authority to establish unverified facts. |
| **T-010** | **Knowledge Poisoning** | Subtle drift or malicious alteration of the knowledge graph. | Immutable versioning of knowledge entities with revision history and author attribution. |
