# 🧠 Local Brain

> **Cerebro cognitivo local-first, memoria persistente y grafo de conocimiento para agentes de Inteligencia Artificial**

[English](README.md) | 🌐 **Español**

[![Version: v1.0.0](https://img.shields.io/badge/Version-v1.0.0-blue.svg)](https://github.com/local-brain/local-brain/releases)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Rust: 1.85+](https://img.shields.io/badge/Rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![CI: Passing](https://img.shields.io/badge/CI-Passing-brightgreen.svg)](https://github.com/local-brain/local-brain/actions)
[![Protocol: Model Context Protocol (MCP)](https://img.shields.io/badge/Protocol-MCP_Standard-blueviolet.svg)](https://modelcontextprotocol.io/)
[![Architecture: Hexagonal](https://img.shields.io/badge/Architecture-Clean_%2F_Hexagonal-lightgrey.svg)](#-arquitectura-del-sistema)
[![Privacy: 100% Offline](https://img.shields.io/badge/Privacy-100%25_Offline-success.svg)](#-privacidad-y-seguridad-local-first)

---

## ⚡ El Problema: La Amnesia de los Agentes de IA

Los agentes de programación modernos (Claude Code, Cursor, Antigravity, Codex, Windsurf, Cline) razonan con gran destreza dentro de una ventana de contexto, pero **pierden toda su experiencia al cerrar la sesión**. 

Cada nueva sesión comienza desde cero:
- **Repiten los mismos errores** que ya resolvieron ayer.
- **Ignoran las convenciones técnicas** y decisiones acordadas en el proyecto.
- **Desconocen las causas y consecuencias** de decisiones pasadas.

### ¿Por qué el RAG tradicional no resuelve esto?
La mayoría de las herramientas intentan resolver la persistencia arrojando texto plano a una base de datos vectorial:
```text
Documento plano → embedding → vector database → similitud de cosenos
```
El RAG tradicional es un buscador de texto, **no un cerebro cognitivo**. No comprende causalidad (*"¿por qué preferimos la solución A sobre la B?"*), no distingue una hipótesis no probada de un hecho verificado, no modela secuencias de pasos técnicos y sufre de alucinaciones masivas cuando se acumulan contradicciones.

---

## 💡 La Solución: Local Brain

**Local Brain** es una infraestructura cognitiva local-first que dota a los agentes de IA de una **memoria estructurada a largo plazo** a través del estándar abierto **Model Context Protocol (MCP)**, garantizando que el usuario conserve la soberanía absoluta de sus datos en su propio hardware.

### 🧭 La Regla de Oro Cognitiva
> **No construir un cerebro que simplemente recuerde todo.**  
> Construir un sistema que sepa: **qué recordar, por qué recordarlo, cuándo recordarlo, qué tan confiable es, de dónde proviene, con qué está relacionado, cuándo dejó de ser válido y por qué llegó a creerlo.**

---

## 📊 RAG Tradicional vs. Local Brain

| Característica | RAG Vectorial Tradicional | Local Brain v1.0 |
| :--- | :---: | :---: |
| **Modelo de datos** | Texto plano desestructurado | **5 tipos cognitivos** (Working, Episodic, Semantic, Procedural, Associative) |
| **Relaciones entre conceptos** | Inexistentes (solo cercanía en espacio latente) | **Grafo de Conocimiento tipado** (10 relaciones canónicas con DAG sin ciclos) |
| **Causalidad y experiencia** | Ignorada | Tríada episódica formal: **Contexto ➔ Acción ➔ Resultado** |
| **Validez y certeza** | Asume que todo texto almacenado es verdad | **Motor de Aprendizaje Empírico**: Observaciones vs Creencias con confianza bayesiana |
| **Detección de contradicciones** | Ninguna (devuelve información contradictoria al azar) | **Motor de Reflexión y Consolidación**: Detección determinista de conflictos cognitivos |
| **Estrategia de búsqueda** | Similitud de vectores aislada | **Recuperación Híbrida**: Vectores (768d) + Full-Text Search + Grafo + Recency Decay |
| **Explicabilidad** | Caja negra | **Matemáticamente explicable** (`--explain` con desglose de señales) |
| **Protección Prompt Injection** | Vulnerable (el texto recuperado puede hackear al agente) | **Aislamiento estricto**: Envoltorio delimitador de datos no confiables |
| **Privacidad e Independencia** | Frecuentemente atado a APIs comerciales en la nube | **100% Offline, cero telemetría**, local en PostgreSQL + `llama.cpp` |

---

## 🚀 Inicio Rápido (En 2 Minutos)

Local Brain incluye automatización completa para levantar sus servicios auxiliares (PostgreSQL 17 con `pgvector` y runtime local de embeddings `llama.cpp` con el modelo de alta fidelidad `nomic-embed-text-v1.5` de 768 dimensiones).

### Opción A: Asistente Automatizado (Recomendado)
```bash
# 1. Clonar el repositorio
git clone https://github.com/local-brain/local-brain.git
cd local-brain

# 2. Ejecutar el asistente de onboarding
./scripts/quickstart.sh
```

El script se encarga de descargar el modelo GGUF cuantizado (~140 MB), levantar los contenedores Docker en segundo plano, ejecutar las migraciones de base de datos (`brain init`), verificar el diagnóstico de salud (`brain doctor`) y mostrar los bloques de configuración para tu cliente MCP.

### Opción B: Instalación Manual
```bash
# 1. Iniciar servicios con Docker Compose
docker compose up -d

# 2. Compilar e instalar la CLI
cargo build --release --bin brain
sudo install -m 755 target/release/brain /usr/local/bin/brain

# 3. Inicializar la persistencia y aplicar migraciones
brain init

# 4. Verificar salud del sistema
brain doctor
```

---

## 🤖 Conectar tus Agentes de IA vía MCP

Local Brain implementa el estándar **Model Context Protocol (MCP)** sobre `stdio`. Agrega la siguiente configuración en tu cliente favorito:

### 1. Claude Desktop
Añade a tu archivo `claude_desktop_config.json`:
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
Añade el servidor en un solo comando:
```bash
claude mcp add local-brain brain -- --database-url postgres://localbrain:localbrain_secret@localhost:5433/local_brain --embedding-url http://127.0.0.1:8081/embedding mcp
```

### 3. Cursor & Windsurf
En `.cursor/mcp.json` o en los ajustes de MCP:
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
En `cline_mcp_settings.json`:
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

## 🧠 Características Principales de Local Brain

### 1. Especialización en 5 Tipos Cognitivos de Memoria
El cerebro modela la memoria respetando invariantes formales de dominio:
1. **Memoria de Trabajo (`working`)**: Estado temporal ligado a una sesión activa (`session_id`) con tiempo de vida en segundos (`ttl_seconds`). Se purga o archiva automáticamente al concluir la sesión.
2. **Memoria Episódica (`episodic`)**: Captura la experiencia situada mediante la tríada rigurosa:
   - **Contexto**: Situación o problema inicial.
   - **Acción**: Decisión técnica o comando adoptado.
   - **Resultado**: Consecuencia empírica real (logs, métricas, latencias).
3. **Memoria Semántica (`semantic`)**: Hechos y principios generalizados con grado de certidumbre empírica (`confidence` $\in [0.0, 1.0]$). Si la confianza es $\ge 0.8$, **exige obligatoriamente** enlaces a UUIDs de evidencias que respalden la afirmación.
4. **Memoria Procedimental (`procedural`)**: Secuencias ordenadas de pasos y comandos ejecutables (`steps`) orientados a un objetivo técnico claro (`goal`).
5. **Memoria Asociativa (`associative`)**: Triplas conceptuales normalizadas (`source` ➔ `predicate` ➔ `target`) con fuerza de enlace.

### 2. Grafo de Conocimiento con Prevención de Ciclos
Conecta recuerdos y entidades técnicas mediante 10 relaciones semánticas canónicas:
- `RELATED_TO`: Relación asociativa general bidireccional.
- `USED_IN`: Componente utilizado en un módulo o sistema.
- `CAUSED_BY`: Relación causal de origen.
- `SOLVES`: Solución técnica demostrada para un error o limitación.
- `CONTRADICTS`: Incompatibilidad detectada entre decisiones o preferencias.
- `SUPERSEDES`: Una decisión más reciente o superior reemplaza a una anterior.
- `DERIVED_FROM`: Conocimiento sintetizado a partir de evidencias previas.
- `DEPENDS_ON`: Dependencia arquitectónica o de entorno.
- `PREFERS`: Directriz de preferencia explícita (ej. preferir PostgreSQL sobre bases embebidas).
- `AVOID`: Directriz de exclusión explícita (ej. evitar dependencias con licencias restrictivas).

> **Garantía DAG**: Las relaciones de dependencia, causalidad y reemplazo implementan algoritmos deterministas de prevención de ciclos en tiempo real, impidiendo bucles infinitos en el razonamiento de los agentes.

### 3. Motor de Aprendizaje Empírico & Confianza
El sistema diferencia de forma estricta entre una **Observación Factual** puntual y una **Creencia Generalizada Candidata**:
- Cada nueva evidencia de soporte o refutación actualiza dinámicamente la puntuación de confianza del conocimiento.
- Soporta fuentes de evidencia tipadas (`Human`, `ToolExecution`, `DirectObservation`, `AgentHypothesis`).
- Las validaciones explícitas de un usuario humano elevan la creencia al grado de máxima certeza.

### 4. Reflexión y Consolidación Asíncrona
El subcomando `brain reflect` agrupa recuerdos episódicos recientes mediante clustering semántico, sintetiza patrones recurrentes y detecta automáticamente colisiones o contradicciones cognitivas (`CONFLICT`), permitiendo al desarrollador o al agente resolver la discrepancia de forma transparente.

### 5. Motor de Recuperación Híbrida Multidimensional
La herramienta `brain_retrieve` (y `brain retrieve` en la CLI) fusiona tres canales de recuperación simultáneos mediante **Reciprocal Rank Fusion (RRF)**:
1. **Canal Vectorial Denso**: 768 dimensiones con distancia de coseno sobre índices HNSW en `pgvector`.
2. **Canal Léxico Full-Text**: Consultas en lenguaje natural procesadas con `tsvector` y diccionarios de PostgreSQL.
3. **Canal de Grafo**: Expansión recursiva multi-salto sobre conceptos y dependencias asociadas.

El ranking final aplica una función de **Scoring Multidimensional**:
$$Score = w_{sem} \cdot S_{sim} + w_{imp} \cdot S_{imp} + w_{conf} \cdot S_{conf} + w_{util} \cdot S_{util} - w_{pen} \cdot P$$
Multiplicado por un factor de **Decaimiento Temporal Exponencial**:
$$D(t) = 2^{-t / T_{1/2}}$$
*(Donde recuerdos con importancia intrínseca $\ge 0.85$ quedan protegidos contra el olvido).*

---

## 💻 Manual de Comandos de la CLI (`brain`)

```bash
brain [FLAGS] <SUBCOMANDO>
```

| Subcomando | Descripción | Ejemplo de Uso |
| :--- | :--- | :--- |
| `init` | Inicializa esquemas y aplica migraciones SQLx en PostgreSQL | `brain init` |
| `doctor` | Diagnostica salud de base de datos, pgvector y llama.cpp | `brain doctor --verbose` |
| `status` | Muestra métricas del sistema y recuerdos almacenados | `brain status` |
| `remember` | Almacena un recuerdo con tipo cognitivo especializado | `brain remember "Contexto..." --type episodic --project app` |
| `recall` | Recuperación por búsqueda semántica vectorial (768d) | `brain recall -q "¿cómo migrar la base de datos?"` |
| `retrieve` | Recuperación híbrida (Vector + FTS + Grafo + Scoring) | `brain retrieve "seguridad en autenticación" --explain` |
| `relate` | Crea una arista tipada en el Grafo de Conocimiento | `brain relate Rust C++ --type PREFERS -w 0.9` |
| `graph` | Explora vecindades recursivas en el grafo | `brain graph Rust --depth 2` |
| `learn` | Registra una observación o creencia con evidencias | `brain learn "El índice HNSW acelera el recall" -e "benchmark local"` |
| `explain` | Explica el fundamento y evidencias de una creencia | `brain explain "¿por qué usamos Rust?"` |
| `reflect` | Ejecuta consolidación y clustering de experiencias | `brain reflect --project app` |
| `conflicts` | Lista y resuelve contradicciones cognitivas pendientes | `brain conflicts list` |
| `backup` | Exporta un respaldo seguro en formato JSONL o SQL | `brain backup -o backup.jsonl` |
| `restore` | Restaura el cerebro desde un archivo con suma de integridad | `brain restore -i backup.jsonl` |
| `mcp` | Inicia el servidor Model Context Protocol sobre stdio | `brain mcp` |

---

## 🛠️ Catálogo de Herramientas MCP para Agentes

Local Brain expone 11 herramientas estándar a través del protocolo MCP:

1. **`brain_remember`**: Registra recuerdos con metadatos completos y especialización cognitiva.
2. **`brain_recall`**: Recuperación rápida mediante similitud de vectores o filtros simples.
3. **`brain_search`**: Búsqueda avanzada multicriterio con filtros de fecha (ISO 8601) e importancia.
4. **`brain_retrieve`**: Recuperación híbrida avanzada (Vectores + FTS + Grafo) con decaimiento y explicabilidad.
5. **`brain_relate`**: Vincula nodos en el Grafo de Conocimiento con 10 relaciones tipadas y DAG sin ciclos.
6. **`brain_graph`**: Recorre y explora vecindades recursivas en el grafo de conocimiento.
7. **`brain_learn`**: Registra observaciones empíricas vs creencias candidatas con fuentes de evidencia.
8. **`brain_explain`**: Desglosa por qué el sistema cree una afirmación, evaluando historial y consistencia.
9. **`brain_consolidate`**: Ejecuta reflexión asíncrona sobre experiencias recientes.
10. **`brain_forget`**: Eliminación lógica (*soft-delete*) segura con confirmación explícita (`confirm: true`).
11. **`brain_session_end`**: Cierra la sesión activa y expira los recuerdos transitorios de trabajo.

---

## 📦 Skill Oficial para Agentes (`local-brain`)

Para que tus agentes de IA aprovechen automáticamente las mejores prácticas de Local Brain sin que tengas que guiarlos en cada prompt, el repositorio incluye una **Skill Oficial**:

- Manifiesto: [`.agents/skills/local-brain/SKILL.md`](file:///home/guty_3rrez/Proyectos/local-brain/.agents/skills/local-brain/SKILL.md)
- Catálogo de herramientas: [`mcp-tools.md`](file:///home/guty_3rrez/Proyectos/local-brain/.agents/skills/local-brain/references/mcp-tools.md)
- Guía de tipos cognitivos: [`memory-types.md`](file:///home/guty_3rrez/Proyectos/local-brain/.agents/skills/local-brain/references/memory-types.md)
- Patrones de flujo agéntico: [`workflow-patterns.md`](file:///home/guty_3rrez/Proyectos/local-brain/.agents/skills/local-brain/references/workflow-patterns.md)

Para instalarla en tu entorno global:
```bash
mkdir -p ~/.agents/skills/local-brain
cp -r .agents/skills/local-brain/* ~/.agents/skills/local-brain/
```

---

## 🏛️ Arquitectura del Sistema

Local Brain está estructurado bajo los principios de **Clean Architecture / Arquitectura Hexagonal**, desacoplando totalmente la lógica cognitiva de cualquier infraestructura física:

```mermaid
flowchart TD
    subgraph Clients["Agentes & Interfaces"]
        CC["Claude Code"]
        CU["Cursor / Windsurf"]
        AG["Antigravity / Codex"]
        CLI["brain CLI (clap)"]
    end

    subgraph Entrypoints["Adaptadores Primarios (Entrada)"]
        MCP["brain-mcp (MCP Server stdio)"]
        CMD["brain-cli (Comandos y flags)"]
    end

    subgraph Application["Capa de Aplicación (brain-application)"]
        UC["Casos de Uso (Remember, Recall, Retrieve, Relate, Learn, Reflect)"]
    end

    subgraph Domain["Capa de Dominio Puro (brain-domain)"]
        DOM["Entidades Invariantes, Tipos Cognitivos, Fórmulas de Scoring, Traits"]
    end

    subgraph SpecializedCrates["Módulos Especializados"]
        GRA["brain-graph (Grafo y DAG)"]
        LEA["brain-learning (Evidencias y Confianza)"]
        CON["brain-consolidation (Reflexión y Conflictos)"]
        RET["brain-retrieval (FTS + Vector + Grafo + Decaimiento)"]
    end

    subgraph Infrastructure["Adaptadores Secundarios (brain-infrastructure)"]
        PG[("PostgreSQL 17 + pgvector (SQLx)")]
        LLM["llama.cpp (Inferencia Local 768d)"]
    end

    Clients --> Entrypoints
    Entrypoints --> Application
    Application --> Domain
    Application --> SpecializedCrates
    SpecializedCrates --> Domain
    Infrastructure -.->|Implementa puertos| Domain
```

- **Dominio Puro**: `brain-domain` compila en milisegundos y tiene **cero dependencias** de bases de datos, redes, GPU o frameworks externos.

---

## 💻 Hardware de Referencia y Rendimiento

Local Brain fue probado y optimizado para ejecutarse con latencias mínimas en hardware de consumo común:

| Componente | Especificación de Referencia |
| :--- | :--- |
| **Procesador (CPU)** | AMD Ryzen 7 6800H (8 núcleos, 16 hilos) o equivalente x86_64 / ARM64 |
| **Memoria RAM** | 16 GB DDR4/DDR5 |
| **GPU / VRAM** | NVIDIA GeForce RTX 3050 Laptop (4 GB VRAM) o CPU pura |
| **Almacenamiento** | SSD NVMe |
| **Modelo de Embeddings** | `nomic-embed-text-v1.5` Q8_0 (768 dimensiones, ~140 MB) |

### Métricas de Latencia Observadas
- **Validación e Invariantes de Dominio**: `< 3 µs`
- **Similitud Coseno pura en memoria**: `< 1 µs`
- **Scoring Multidimensional con Decaimiento**: `< 500 ns`
- **Navegación de Grafo (3 saltos)**: `< 100 µs`
- **Búsqueda Vectorial HNSW en PostgreSQL**: `< 5 ms`
- **Pipeline Híbrido End-to-End**: `< 40 ms`

---

## 🛡️ Privacidad y Seguridad Local-First

- **100% Offline**: Local Brain bloquea por diseño cualquier conexión saliente externa (`--offline`).
- **Cero Telemetría**: Ningún dato de tus proyectos ni registros de memoria salen de tu máquina.
- **Protección Anti-Prompt Injection**: Todo contenido recuperado de la memoria se encapsula bajo etiquetas delimitadoras de datos no confiables (`<untrusted_memory_content>`), previniendo que recuerdos históricos puedan secuestrar las instrucciones de tu agente.
- **Soft-Delete Seguro**: Los recuerdos eliminados se marcan lógicamente y requieren confirmación explícita para evitar pérdidas accidentales.
- **Respaldos Portables**: Exportación completa a formatos abiertos JSONL o SQL con validación de sumas SHA-256.

---

## 🤝 Cómo Colaborar

Agradecemos enormemente las contribuciones de la comunidad (tanto de desarrolladores humanos como de agentes de IA):

1. **Revisa las Normas**:
   - Para desarrolladores: [`CONTRIBUTING.md`](file:///home/guty_3rrez/Proyectos/local-brain/CONTRIBUTING.md).
   - Para agentes de IA: [`AGENTS.md`](file:///home/guty_3rrez/Proyectos/local-brain/AGENTS.md).
   - Para Claude Code: [`CLAUDE.md`](file:///home/guty_3rrez/Proyectos/local-brain/CLAUDE.md).
2. **Explora Tareas**: Revisa los [GitHub Issues y Milestones](https://github.com/local-brain/local-brain/issues).
3. **Decisiones de Diseño**: Consulta las decisiones registradas en [`docs/adr/`](file:///home/guty_3rrez/Proyectos/local-brain/docs/adr).

---

## 📄 Licencia

Este proyecto está licenciado bajo la **GNU Affero General Public License v3.0 (AGPL-3.0-or-later)**.

> [!NOTE]
> La licencia AGPLv3 es una licencia de **copyleft fuerte**. Garantiza que Local Brain y cualquier mejora o derivación continúen siendo **siempre software libre y de código abierto**. Consulta el archivo [`LICENSE`](file:///home/guty_3rrez/Proyectos/local-brain/LICENSE) para los términos completos.
