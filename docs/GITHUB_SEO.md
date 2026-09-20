# 📈 Guía de SEO, Tracción y Posicionamiento — Local Brain

> **Estrategia integral de visibilidad orgánica y tracción en GitHub para el lanzamiento de Local Brain v1.0.**  
> Esta guía maximiza el descubrimiento por parte de desarrolladores e investigadores sin caer en "vender humo", destacando el rigor técnico, la arquitectura determinista y la solución real al problema de la amnesia en agentes de IA.

---

## 🎯 1. Configuración de Metadatos del Repositorio en GitHub

Para optimizar la indexación en el buscador interno de GitHub y en Google/Bing, configura los siguientes campos en la página principal del repositorio (`Settings` o botón `⚙️ Edit repository details`):

### 1.1 Descripción Oficial (About)
```text
Local-first, cognitive persistent memory & knowledge graph for AI coding agents (Claude, Cursor, Antigravity) via Model Context Protocol (MCP). 100% offline, privacy-first.
```
- **Longitud**: ~175 caracteres (dentro del límite de 350 caracteres de GitHub).
- **Palabras clave indexadas**: `local-first`, `cognitive persistent memory`, `knowledge graph`, `AI coding agents`, `Claude`, `Cursor`, `Antigravity`, `Model Context Protocol`, `MCP`, `offline`, `privacy-first`.

### 1.2 URL del Proyecto (Website)
```text
https://github.com/guty3rrez/local-brain#readme
```

### 1.3 Tópicos Estratégicos (Topics / Tags)
Agrega los siguientes **15 tópicos** en la sección *Topics* del repositorio:
1. `model-context-protocol`
2. `mcp`
3. `ai-agents`
4. `persistent-memory`
5. `knowledge-graph`
6. `local-first`
7. `rust`
8. `pgvector`
9. `llama-cpp`
10. `embeddings`
11. `hybrid-search`
12. `semantic-memory`
13. `claude-code`
14. `cursor-ai`
15. `privacy-first`

---

## 🖼️ 2. Imagen de Vista Previa Social (Open Graph Card)

GitHub muestra una tarjeta gráfica cuando el repositorio se comparte en Twitter/X, LinkedIn, Discord, Slack o Reddit.

- **Dimensiones recomendadas**: `1280 × 640 px` (formato PNG o WebP, relación de aspecto 2:1).
- **Ubicación en GitHub**: `Settings` → `General` → `Social preview` → `Edit` → `Upload an image`.
- **Composición Visual Sugerida**:
  - **Fondo**: Azul marino oscuro / carbón (`#0B0F19` a `#1E293B`).
  - **Elemento Central**: Isotipo de un cerebro vectorizado con nodos interconectados (representando el Grafo de Conocimiento).
  - **Tipografía Principal**:
    - **LOCAL BRAIN** (Sans-serif bold, blanco).
    - *Persistent Cognitive Memory for AI Agents* (gris claro `#94A3B8`).
  - **Insignias Visuales en las Esquinas**:
    - `🦀 Built in Rust`
    - `⚡ Model Context Protocol (MCP)`
    - `🛡️ 100% Offline & Local-First`
    - `🧠 768-dim Vectors + Knowledge Graph`

---

## 💎 3. La Propuesta de Valor Central (Sin Vender Humo)

Para generar tracción genuina y duradera en la comunidad de desarrolladores de software y ML, la narrativa no debe basarse en promesas vacías de "IA mágica", sino en **ingeniería de sistemas sólida y contrastada**:

### El Problema Real
Los agentes de IA actuales son brillantes resolviendo problemas aislados en un turno de conversación, pero **sufren de amnesia total**:
1. Olvidan por qué se tomó una decisión arquitectónica hace dos días.
2. Repiten los mismos errores de compilación o bugs en sesiones sucesivas.
3. El RAG tradicional es demasiado primitivo: arrojar fragmentos desordenados a una base de vectores produce ruido y alucinaciones.

### La Solución de Local Brain
Local Brain trata la memoria como un **sistema cognitivo estructurado**:
- **5 tipos de memoria con invariantes de dominio**: No todo es texto. Hay recuerdos situacionales (*Episódicos*), reglas y hechos (*Semánticos*), secuencias de comandos (*Procedimentales*), ontologías (*Asociativos*) y estado transitorio con TTL (*Working Memory*).
- **Grafo de conocimiento**: Relaciones causales (`SOLVES`, `PREFERS`, `AVOID`, `CAUSED_BY`, `DEPENDS_ON`) con prevención estricta de ciclos en DAGs.
- **Autoridad determinista**: La base de datos relacional y las invariantes de código mandan sobre el LLM. El LLM sugiere; el motor determinista valida.
- **Aprendizaje empírico**: Diferenciación formal entre observaciones factuales y creencias candidatas, con actualización bayesiana de confianza.
- **Recuperación híbrida con decaimiento**: Búsqueda vectorial HNSW + búsqueda léxica FTS + expansión en grafo + función de decaimiento temporal por media vida ($T_{1/2}$).
- **Seguridad nativa**: 100% offline, cero telemetría, mitigación rigurosa de prompt injection mediante encapsulado con delimitadores no confiables.

---

## 📢 4. Plantillas de Lanzamiento para Comunidades

### 4.1 Show HN (Hacker News)
```text
Title: Show HN: Local Brain – Local-first cognitive memory and knowledge graph for AI agents (Rust + MCP)

Hey HN,

We built Local Brain because AI coding agents (Claude Code, Cursor, Antigravity) are great at immediate reasoning, but completely forget context, decisions, and hard-earned lessons between sessions.

Most "agent memory" tools today just dump raw chat transcripts into a vector DB and do similarity searches. In practice, that creates high hallucination rates and zero understanding of causality ("why did we pick X over Y last week?").

Local Brain approaches agent memory as a structured cognitive system:
1. Five specialized memory types with domain invariants: Working (ephemeral with TTL), Episodic (Context + Action + Outcome), Semantic (Beliefs + empirical evidence), Procedural (technical step sequences), and Associative (concept triples).
2. A typed Knowledge Graph (10 relations like PREFERS, AVOID, SOLVES, DEPENDS_ON) with cycle-prevention on DAGs.
3. Advanced hybrid retrieval: Combining 768-dimensional dense vectors (pgvector HNSW), PostgreSQL Full-Text Search, and graph traversal with reciprocal rank fusion and recency decay scoring.
4. Model Context Protocol (MCP) server: Exposes 11 tools over stdio, seamlessly pluggable into Claude Desktop, Claude Code, Cursor, Windsurf, or custom agents.
5. Strict local-first & deterministic authority: Runs 100% offline using PostgreSQL 17 + pgvector and llama.cpp (nomic-embed-text-v1.5). Zero telemetry. Domain logic is pure Rust with zero DB/GPU coupling.

Everything is open source under AGPLv3. Prebuilt binaries and a 2-minute automated docker quickstart are available.

Code: https://github.com/guty3rrez/local-brain

Feedback and questions on the cognitive architecture or MCP integration are warmly welcome!
```

### 4.2 Reddit (r/LocalLLaMA & r/rust)
```text
Title: We built Local Brain: 100% offline, cognitive persistent memory & knowledge graph for AI coding agents (Rust, pgvector, llama.cpp, MCP)

Hey everyone!

One of the biggest pain points when coding with AI agents (Claude Code, Cursor, local LLM harnesses) is context amnesia. When a session ends, all the troubleshooting, architecture choices, and lessons learned vanish.

Instead of naive vector RAG (which treats all memory as flat embeddings), we built Local Brain in pure Rust:
- Specialized memory types: Episodic (What happened?), Semantic (What do we believe?), Procedural (How to do it?), Associative (How are concepts linked?), and Working (session-scoped TTL).
- Knowledge Graph: Cycle-safe typed edges so agents know what tools to AVOID and what patterns to PREFER.
- 100% Local & Privacy-first: Uses PostgreSQL 17 with pgvector and local llama.cpp embeddings. No cloud APIs, no external telemetry.
- Standard MCP protocol: Compatible with any MCP client over stdio.

Quickstart with Docker Compose:
`./scripts/quickstart.sh`

Repo: https://github.com/guty3rrez/local-brain

Would love to hear your thoughts on cognitive memory modeling for autonomous agents!
```

### 4.3 Twitter / X Tech Thread
```text
🧵 1/6 Agents have an amnesia problem. They debug an obscure race condition for 2 hours today, but tomorrow in a new session, they make the exact same mistake.

Vector RAG isn't enough. We need structured cognitive memory.

Introducing Local Brain v1.0 🧠👇
https://github.com/guty3rrez/local-brain

2/6 Why does naive RAG fail for agents?
Because human/agent experience isn't just flat text chunks.
Local Brain structures memory into 5 distinct cognitive types:
• Episodic (Context ➔ Action ➔ Outcome)
• Semantic (Beliefs backed by evidence)
• Procedural (Step sequences)
• Working (Session TTL)
• Associative

3/6 It also includes a Knowledge Graph with 10 typed relations (PREFERS, AVOID, SOLVES, DEPENDS_ON) with cycle-prevention for DAGs.
If your agent learned that Library X broke in Alpine, it won't suggest Library X on Alpine again.

4/6 Retrieval is truly hybrid:
Dense Vectors (768d HNSW via pgvector) + Full-Text Lexical Search (PostgreSQL) + Graph Expansion + Half-life Recency Decay.
And every score can be mathematically explained (`--explain`).

5/6 100% Local-First & Privacy Guaranteed:
• Written in pure Rust (Clean Hexagonal Architecture)
• Local llama.cpp embeddings (nomic-embed-text-v1.5)
• Zero cloud dependencies, zero telemetry
• Model Context Protocol (MCP) server for instant setup in Claude, Cursor, and Windsurf.

6/6 Fully open source under AGPLv3.
Try it in 2 minutes:
`git clone https://github.com/guty3rrez/local-brain && ./scripts/quickstart.sh`

GitHub: https://github.com/guty3rrez/local-brain
⭐ Star the repo if you believe local agents need persistent brains!
```
