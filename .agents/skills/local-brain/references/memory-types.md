# Cognitive Memory Types — Local Brain

Local Brain rejects the naive approach of treating all memory as flat unstructured text. The system models **five specialized cognitive types**, each with distinct lifecycles, decay models, and pure domain invariants.

---

## 🧠 Comparison Matrix

| Memory Type | Nature | Persistence | Key Structure | Typical Use Case |
| :--- | :--- | :--- | :--- | :--- |
| **`working`** | Active session state | Ephemeral (TTL) | `session_id`, `ttl_seconds`, `goal` | Immediate context of active task, transient scratch variables. |
| **`episodic`** | Situated experiences | Permanent | `context`, `action`, `outcome` | What was attempted, what was done, and what happened. |
| **`semantic`** | Facts & guidelines | Permanent | `statement`, `confidence`, `evidence_ids` | Architectural conventions, validated preferences, core rules. |
| **`procedural`** | Ordered technical steps | Permanent | `goal`, `steps: [step_number, cmd, action]` | Operational runbooks, compilation sequences, deployment steps. |
| **`associative`** | Ontological connections | Permanent | `source_concept`, `target_concept`, `predicate`, `strength` | Direct concept-to-concept triples linking technologies and patterns. |

---

## 1. Working Memory (`working`)
- **Purpose**: Maintain immediate scratchpad state without polluting long-term memory.
- **Invariants**:
  - Requires non-empty `session_id`.
  - Automatically expires after `ttl_seconds` or when `brain_session_end` is invoked.
  - Does not undergo gradual recency decay; it is cleanly archived or purged.

---

## 2. Episodic Memory (`episodic`)
- **Purpose**: Capture empirical trial-and-error experiences of agents over time.
- **Invariants**:
  - Structured under the strict triad:
    - **Context (`context`)**: Situation or problem motivating the action.
    - **Action (`action`)**: Technical command or decision executed.
    - **Outcome (`outcome`)**: Empirical consequence (error traces, benchmark latencies, success).
  - Supports intrinsic importance (`importance` $\in [0.0, 1.0]$).
  - Primary input for offline **reflection and consolidation**, where repetitive episodes yield generalized semantic beliefs.

---

## 3. Semantic Memory (`semantic`)
- **Purpose**: Store architectural principles, design guidelines, and verified facts.
- **Invariants**:
  - Contains a non-empty `statement`.
  - Measures empirical confidence (`confidence` $\in [0.0, 1.0]$).
  - **Strict Evidence Invariant**: If `confidence >= 0.8`, the memory **mandates** at least one supporting evidence UUID in `evidence_ids`. Agents cannot assert absolute truths without empirical backing.

---

## 4. Procedural Memory (`procedural`)
- **Purpose**: Encode step-by-step technical execution recipes.
- **Invariants**:
  - Defines a technical goal (`goal`) and title.
  - Contains an ordered list of `steps`, each with:
    - `step_number`: Positive ordinal integer (1, 2, 3...).
    - `description`: Action description.
    - `command` *(optional)*: Shell or CLI command.
    - `action_type` *(optional)*: Action category.
  - Step numbers must be strictly sequential with zero gaps.

---

## 5. Associative Memory (`associative`)
- **Purpose**: Establish semantic concept triples between entities.
- **Invariants**:
  - Canonical triple: `source_concept` → `predicate` → `target_concept`.
  - Normalized associative strength (`strength` $\in [0.0, 1.0]$).
  - `source_concept` and `target_concept` cannot be identical (self-loops are prohibited).
