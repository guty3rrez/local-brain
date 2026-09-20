# ⚡ Quickstart Guide — Local Brain v1.0

> **Equip your AI agents with cognitive persistent memory in less than 3 minutes.**

🌐 **English** | [Español](QUICKSTART.es.md)

---

## 🎯 What Will You Achieve?
By the end of this guide, you will have:
1. **Local infrastructure running**: PostgreSQL 17 with `pgvector` and the `llama.cpp` embedding runtime operating 100% offline on your machine.
2. **Operational `brain` CLI**: Initialized with relational schemas, 768-dimensional HNSW indexes, and passing health diagnostics.
3. **AI agents connected via MCP**: Claude Code, Cursor, Claude Desktop, Antigravity, or Cline sharing memories, learnings, and knowledge graph relations.

---

## 📋 Minimum Prerequisites

- **Docker and Docker Compose** (v2+)
- **Operating System**: Linux (Ubuntu, Debian, Fedora, Arch), macOS (Apple Silicon or Intel), or Windows 10/11 (with WSL2).
- **Recommended Hardware**: 4 GB available RAM and ~2 GB disk space (for PostgreSQL, pgvector, and the `nomic-embed-text` model).

---

## 🚀 Method 1: Automated Onboarding Wizard (Recommended)

Run the interactive setup assistant from the root of the repository:

```bash
./scripts/quickstart.sh
```

The script automatically:
1. Downloads the quantized GGUF embedding model (~140 MB).
2. Starts the Docker containers in the background.
3. Waits for PostgreSQL and llama.cpp to report healthy status.
4. Applies database migrations (`brain init`).
5. Executes a full system health diagnostic (`brain doctor`).
6. Prints copy-pasteable MCP configuration blocks for your AI tools.

---

## 🛠️ Method 2: Manual Step-by-Step

### Step 1: Download the Pre-built Binary
Download the binary for your platform from [GitHub Releases](https://github.com/guty3rrez/local-brain/releases):

```bash
# Example for Linux x86_64:
curl -sL https://github.com/guty3rrez/local-brain/releases/latest/download/brain-linux-x86_64.tar.gz | tar xz
sudo install -m 755 brain /usr/local/bin/brain
```

*Or build directly from source using Cargo:*
```bash
cargo build --release --bin brain
sudo install -m 755 target/release/brain /usr/local/bin/brain
```

### Step 2: Start Local Services
From the repository root:
```bash
docker compose up -d
```
Verify the services are running:
```bash
docker compose ps
```
- `local-brain-postgres` on port `5433`
- `local-brain-embeddings` on port `8081`

### Step 3: Initialize the Database
Run migrations:
```bash
brain init
```
Expected output:
```text
🧠 Inicializando Local Brain...
✅ Base de datos inicializada exitosamente.
   Instancia:    PostgreSQL 17 + extensión pgvector
   Migraciones:  Aplicadas con éxito (memories, pgvector, HNSW 768 dim).
🚀 Local Brain está listo para operar con búsqueda semántica.
```

### Step 4: Health Diagnostics
Verify that your entire stack is operational:
```bash
brain doctor
```
```text
🧠 Local Brain Doctor — System Health Diagnostic
   Core: v1.0.0 | OS: linux
----------------------------------------------------------------------
 🟢 [System       ] Local Brain Version           Core v1.0.0 on platform linux
 🟢 [Configuration] brain.toml file               Loaded and validated successfully
 🟢 [Security     ] Offline Mode                  Standard local-first policy active
 🟢 [Persistence  ] PostgreSQL Connectivity       Connected (PostgreSQL 17.11 pgvector)
 🟢 [Persistence  ] Table Schemas                 All 7 cognitive tables migrated
 🟢 [Search       ] Full-Text Search (FTS)        tsvector column 'tsv' and GIN index active
 🟢 [Vectors      ] pgvector Extension            Installed v0.8.6 (cosine distance OK)
 🟢 [Embeddings   ] llama.cpp Runtime             Online (dimension: 768d nomic-embed-text)
 🟢 [Queues       ] Pending Embeddings            All memories vectorized and indexed
 🟢 [Queues       ] Working Memory TTL            No expired working memories pending
 🟢 [Knowledge    ] Conflicts & Contradictions    No unresolved cognitive contradictions
----------------------------------------------------------------------
   Summary: 11 OK | 0 Warnings | 0 Critical Failures
   Overall Status: 🟢 HEALTHY AND OPERATIONAL
```

---

## 🤖 Connect Your AI Agents via MCP

Local Brain implements the **Model Context Protocol (MCP)** over `stdio`. Select your client below:

### 1. Claude Desktop
Edit your `claude_desktop_config.json`:
- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Linux**: `~/.config/Claude/claude_desktop_config.json`
- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

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
Add the server with a single command:
```bash
claude mcp add local-brain brain -- --database-url postgres://localbrain:localbrain_secret@localhost:5433/local_brain --embedding-url http://127.0.0.1:8081/embedding mcp
```

### 3. Cursor & Windsurf
Add to `.cursor/mcp.json` or your MCP Settings:
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

### 4. Cline / Roo Code (VS Code Extension)
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

## 🧪 Terminal Quick Test

Test your memory layer directly from the CLI:

### 1. Store a decision or convention
```bash
brain remember \
  "In this backend service we use PASETO v4 tokens instead of JWT to mitigate algorithm 'none' vulnerabilities" \
  --type semantic \
  --project my-backend \
  --confidence 0.95
```

### 2. Semantic recall by meaning
```bash
brain recall -q "what token format do we use for authentication?" --project my-backend
```

### 3. Advanced hybrid retrieval with scoring breakdown
```bash
brain retrieve "authentication token security" --project my-backend --explain
```

### 4. Link concepts in the Knowledge Graph
```bash
brain relate PASETO JWT --type PREFERS -w 0.95 --context "Cryptographic security and defense against known vulnerabilities"
```

### 5. Inspect the Graph
```bash
brain graph PASETO --depth 1
```

---

## ❓ Frequently Asked Questions

#### Can I run Local Brain without a GPU?
**Yes.** The `nomic-embed-text-v1.5` Q8_0 model consumes ~140 MB of RAM and runs at under 15 ms per embedding on modern CPUs. If you have an NVIDIA GPU, CUDA can be used via Docker Compose for higher throughput.

#### Does Local Brain send data outside my machine?
**No.** Local Brain enforces a local-first policy (`--offline`). All persistence stays in your local PostgreSQL database, and all vectors are computed locally via `llama.cpp`. There is zero telemetry.

#### macOS Gatekeeper Warning
If you download the binary via a browser on macOS, Gatekeeper may flag it as an unverified developer. Simply run:
```bash
chmod +x brain
xattr -d com.apple.quarantine brain
```
