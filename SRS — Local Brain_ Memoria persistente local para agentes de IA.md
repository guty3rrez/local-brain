# Software Requirements Specification (SRS)

## Local Brain

**Memoria persistente local, agéntica y orientada a conocimiento para agentes de Inteligencia Artificial**

**Versión:** 0.1.0  
**Estado:** Propuesta inicial  
**Tipo:** Open Source  
**Licencia propuesta:** Apache License 2.0  
**Lenguaje principal:** Rust  
**Arquitectura:** Modular, Hexagonal/Clean Architecture  
**Desarrollo:** Humanos + Agentes de IA  
**Ejecución objetivo:** Local-first  
**Backend de IA:** Independiente del proveedor  
**Persistencia inicial:** PostgreSQL + pgvector  
**Runtime local de IA:** llama.cpp  
**Interfaz para agentes:** Model Context Protocol (MCP)

---

# 1. Introducción

## 1.1 Propósito

Local Brain es un sistema de memoria persistente diseñado para proporcionar a agentes de Inteligencia Artificial una capa de memoria a largo plazo independiente del proveedor de modelos.

El sistema busca superar las limitaciones de un RAG convencional mediante una arquitectura inspirada conceptualmente en distintos mecanismos de memoria humana:

- memoria de trabajo;
- memoria episódica;
- memoria semántica;
- memoria procedimental;
- asociación entre conceptos;
- consolidación de conocimiento;
- detección de contradicciones;
- importancia y relevancia;
- decaimiento y olvido;
- reflexión sobre experiencias.

Local Brain no pretende replicar biológicamente el cerebro humano ni afirmar equivalencia con procesos neurológicos. El término "cerebro" se utiliza como metáfora arquitectónica para describir un sistema de memoria y conocimiento persistente.

---

# 2. Visión

## 2.1 Problema

Los agentes modernos de programación pueden:

- analizar código;
- modificar proyectos;
- ejecutar comandos;
- utilizar herramientas;
- consultar documentación;
- resolver problemas;
- aprender durante una sesión.

Sin embargo, gran parte del conocimiento obtenido durante esas interacciones:

- queda limitado a la sesión;
- se pierde entre proyectos;
- no está estructurado;
- no distingue experiencia de conocimiento;
- no conserva adecuadamente decisiones y sus motivos;
- no mantiene relaciones entre conceptos;
- depende del proveedor del agente;
- puede quedar atrapado en historiales de conversación difíciles de recuperar.

Además, los sistemas RAG tradicionales normalmente reducen la memoria a:

```text
Documento → embedding → vector database → similarity search
```

Esto no representa adecuadamente conceptos como:

> "Intentamos X, falló por Y, posteriormente utilizamos Z, y desde entonces preferimos Z para este tipo de problema."

Local Brain pretende representar precisamente ese tipo de conocimiento.

---

# 3. Objetivos

## 3.1 Objetivo general

Construir una infraestructura de memoria local, persistente, extensible y agnóstica respecto del proveedor de IA que permita a múltiples agentes compartir experiencias, conocimiento y contexto mediante MCP.

## 3.2 Objetivos específicos

El sistema deberá:

1. Permitir almacenar recuerdos persistentes.
2. Diferenciar tipos de memoria.
3. Recuperar información relevante mediante múltiples estrategias.
4. Utilizar embeddings locales sin depender de APIs pagadas.
5. Utilizar modelos LLM locales opcionalmente.
6. Mantener relaciones entre conceptos.
7. Registrar experiencias de agentes.
8. Extraer conocimiento de experiencias.
9. Detectar posibles contradicciones.
10. Mantener historial de cambios del conocimiento.
11. Permitir consolidación de memoria.
12. Permitir distintos agentes utilizar la misma memoria.
13. Exponer capacidades mediante MCP.
14. Proporcionar CLI para interacción humana.
15. Ejecutarse completamente offline cuando los modelos necesarios estén disponibles localmente.
16. Ser extensible mediante providers.
17. Ser desarrollado como software Open Source.
18. Permitir colaboración entre humanos y agentes de IA con trazabilidad.

---

# 4. Principios de diseño

## 4.1 Local-first

La funcionalidad principal no deberá depender de servicios externos.

El sistema deberá poder funcionar sin:

- OpenAI API;
- Anthropic API;
- Google AI API;
- proveedores externos de embeddings;
- servicios SaaS de almacenamiento.

Los proveedores externos podrán existir como integraciones opcionales.

---

## 4.2 Provider agnostic

El sistema no deberá asumir que un único modelo o proveedor será utilizado.

Ejemplo:

```text
EmbeddingProvider
LLMProvider
RerankerProvider
StorageProvider
GraphProvider
```

Las implementaciones podrán ser:

```text
LocalLlamaCppProvider
OpenAIProvider
AnthropicProvider
OllamaProvider
CustomProvider
```

El núcleo no deberá depender directamente de ninguna implementación concreta.

---

## 4.3 Human-in-the-loop

El sistema deberá diferenciar entre:

- información aportada por humanos;
- información generada por agentes;
- información inferida por el sistema;
- conocimiento consolidado;
- conocimiento verificado.

Las operaciones de alto impacto podrán requerir revisión humana.

---

## 4.4 Trazabilidad

Toda modificación relevante del conocimiento deberá poder responder:

- ¿Quién la creó?
- ¿Cuándo?
- ¿Qué agente participó?
- ¿Qué experiencia la originó?
- ¿Qué evidencia existe?
- ¿Qué versión anterior existía?
- ¿Por qué cambió?
- ¿Quién aprobó el cambio?

---

## 4.5 No confiar ciegamente en el LLM

El LLM deberá ser considerado un componente probabilístico.

La base de datos y las reglas deterministas deberán conservar autoridad sobre:

- identidad;
- relaciones;
- historial;
- permisos;
- estados;
- timestamps;
- integridad;
- versionado.

---

# 5. Alcance

## 5.1 Incluido

### Núcleo

- almacenamiento de memoria;
- recuperación;
- clasificación;
- embeddings;
- relaciones;
- versionado;
- importancia;
- confianza;
- procedencia.

### Tipos de memoria

- Working Memory;
- Episodic Memory;
- Semantic Memory;
- Procedural Memory;
- Associative Memory.

### IA local

- embeddings locales;
- inferencia LLM local;
- clasificación;
- extracción;
- resumen;
- consolidación;
- reflexión.

### Interfaces

- MCP;
- CLI;
- API interna;
- configuración mediante archivos.

### Persistencia

- PostgreSQL;
- pgvector;
- almacenamiento de metadatos;
- historial.

---

## 5.2 Fuera del alcance inicial

No forman parte del MVP:

- entrenamiento de modelos desde cero;
- simulación neuronal biológica;
- desarrollo de un modelo fundacional;
- implementación propia de CUDA;
- implementación propia de un motor vectorial;
- reemplazo de llama.cpp;
- interfaz gráfica completa;
- servicio SaaS;
- sincronización cloud obligatoria.

---

# 6. Usuarios y actores

## 6.1 Human Developer

Persona que utiliza el sistema directamente.

Puede:

- consultar memoria;
- inspeccionar recuerdos;
- crear recuerdos;
- corregir conocimiento;
- aprobar consolidaciones;
- eliminar información;
- configurar providers.

---

## 6.2 AI Agent

Agente externo como:

- Codex;
- Claude Code;
- Antigravity;
- otros agentes compatibles con MCP.

Puede:

- recuperar conocimiento;
- registrar experiencias;
- solicitar reflexión;
- consultar procedimientos;
- aportar nuevos conocimientos.

---

## 6.3 Local AI Runtime

Sistema local encargado de ejecutar:

- modelos de embeddings;
- LLM;
- rerankers.

Implementación inicial:

```text
llama.cpp
```

---

## 6.4 System Administrator

Responsable de:

- configuración;
- almacenamiento;
- modelos;
- backups;
- seguridad;
- actualización.

---

# 7. Arquitectura conceptual

```text
                         ┌─────────────────────┐
                         │     AI Agents       │
                         │                     │
                         │ Codex               │
                         │ Claude Code         │
                         │ Antigravity         │
                         │ Other MCP Agents    │
                         └──────────┬──────────┘
                                    │
                                   MCP
                                    │
                         ┌──────────▼──────────┐
                         │    Brain Gateway    │
                         │        Rust         │
                         └──────────┬──────────┘
                                    │
                    ┌───────────────┼────────────────┐
                    │               │                │
                    ▼               ▼                ▼
              Memory Engine     Retrieval       Learning
                    │               │                │
                    └───────────────┼────────────────┘
                                    │
                     ┌──────────────┼──────────────┐
                     │              │              │
                     ▼              ▼              ▼
                PostgreSQL      pgvector       Graph
                     │
                     │
                     ▼
              Local AI Runtime
                     │
             ┌───────┴────────┐
             │                │
       Embedding Model     Local LLM
             │                │
             └───────┬────────┘
                     ▼
                  llama.cpp
```

---

# 8. Arquitectura de software

El sistema deberá utilizar arquitectura Clean/Hexagonal.

```text
crates/
├── brain-core/
├── brain-domain/
├── brain-application/
├── brain-infrastructure/
├── brain-memory/
├── brain-retrieval/
├── brain-learning/
├── brain-consolidation/
├── brain-graph/
├── brain-embeddings/
├── brain-llm/
├── brain-mcp/
├── brain-cli/
└── brain-daemon/
```

La estructura definitiva podrá modificarse durante el desarrollo.

La regla principal será:

```text
Domain
   ↓
Application
   ↓
Ports
   ↓
Infrastructure
```

El dominio no deberá depender de PostgreSQL, llama.cpp, MCP ni ningún proveedor concreto.

---

# 9. Modelo de memoria

## 9.1 Memory

Entidad base:

```text
Memory
├── id
├── type
├── content
├── summary
├── source
├── project
├── agent
├── importance
├── confidence
├── utility
├── created_at
├── updated_at
├── last_retrieved_at
├── status
├── version
└── embedding
```

---

# 10. Tipos de memoria

## 10.1 Working Memory

Información temporal asociada a una sesión activa.

Ejemplo:

```text
Proyecto:
Sistema veterinario

Objetivo:
Implementar trazabilidad

Problema:
Sincronización offline

Hipótesis:
Outbox pattern
```

Debe tener ciclo de vida corto.

---

## 10.2 Episodic Memory

Representa experiencias concretas.

Ejemplo:

```text
En el proyecto X utilizamos Supabase.
La implementación produjo problemas de autorización
debido a la complejidad de los roles.
Posteriormente se migró parte del backend.
```

Debe conservar:

- contexto;
- acción;
- resultado;
- agente;
- fecha;
- proyecto.

---

## 10.3 Semantic Memory

Representa conocimiento generalizado.

Ejemplo:

```text
Para sistemas con múltiples roles y reglas complejas,
.NET + PostgreSQL puede proporcionar mayor control
que una arquitectura basada exclusivamente en BaaS.
```

Debe indicar:

- nivel de confianza;
- evidencia;
- origen;
- fecha de actualización.

---

## 10.4 Procedural Memory

Representa métodos y procedimientos.

Ejemplo:

```text
Crear feature Flutter:

1. Entidad de dominio
2. Repository contract
3. Datasource
4. Model
5. Repository implementation
6. Provider
7. UI
8. Tests
```

---

## 10.5 Associative Memory

Representa relaciones entre entidades y conceptos.

Ejemplo:

```text
Flutter
 ├── uses → Riverpod
 ├── compatible → Clean Architecture
 └── used_in → Montierrez
```

---

# 11. Knowledge Graph

El sistema deberá soportar relaciones entre conceptos.

Tipos iniciales:

```text
RELATED_TO
USED_IN
CAUSED_BY
SOLVES
CONTRADICTS
SUPERSEDES
DERIVED_FROM
DEPENDS_ON
PREFERS
AVOID
```

Ejemplo:

```text
Decision A
     │
     └── superseded_by → Decision B

Experience A
     │
     └── generated → Knowledge A

Knowledge A
     │
     └── contradicts → Knowledge B
```

---

# 12. Embeddings

## 12.1 Requisito

Los embeddings deberán poder generarse completamente de manera local.

El sistema no deberá requerir una API externa.

---

## 12.2 Provider

Se definirá:

```rust
trait EmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Embedding>;
}
```

La implementación inicial podrá comunicarse con un runtime local basado en llama.cpp.

---

## 12.3 Procesamiento asíncrono

La generación de embeddings no deberá bloquear innecesariamente operaciones críticas.

Ejemplo:

```text
remember()
    ↓
persist memory
    ↓
queue embedding job
    ↓
generate embedding
    ↓
store vector
```

Si el embedding falla:

```text
embedding_status = pending
```

La memoria deberá continuar existiendo.

---

# 13. Retrieval Engine

La recuperación deberá ser híbrida.

## 13.1 Estrategias

### Metadata filtering

```text
project
memory_type
agent
date
status
confidence
```

### Full-text search

Búsqueda tradicional.

### Vector search

Búsqueda semántica mediante pgvector.

### Graph traversal

Recuperación de conceptos relacionados.

### Recency

Priorización temporal.

### Importance

Priorización por relevancia.

### Confidence

Priorización de conocimiento confiable.

---

## 13.2 Retrieval pipeline

```text
Query
  ↓
Intent extraction
  ↓
Metadata filtering
  ↓
Full-text search
  ↓
Vector search
  ↓
Graph expansion
  ↓
Candidate merge
  ↓
Scoring
  ↓
Optional reranking
  ↓
Context assembly
  ↓
Agent
```

---

# 14. Memory scoring

El sistema deberá utilizar un mecanismo configurable.

Ejemplo conceptual:

```text
score =
    semantic_similarity
    × importance
    × confidence
    × utility
    × recency_factor
```

Los valores exactos deberán validarse experimentalmente.

El sistema no deberá asumir que una fórmula fija representa adecuadamente todos los casos.

---

# 15. Learning Engine

El sistema deberá diferenciar:

```text
Experience
    ↓
Observation
    ↓
Candidate Knowledge
    ↓
Validation
    ↓
Consolidated Knowledge
```

Un agente no deberá poder convertir automáticamente cualquier afirmación en conocimiento de alta confianza sin registrar su procedencia.

---

# 16. Consolidation Engine

La consolidación transformará experiencias en conocimiento reutilizable.

Ejemplo:

```text
Experience 1:
Problema con X.

Experience 2:
Problema similar con X.

Experience 3:
X volvió a producir problemas.

Consolidation
       ↓

Candidate insight:
"X presenta problemas recurrentes bajo
 determinadas condiciones."
```

El insight deberá conservar referencias a las experiencias originales.

---

# 17. Reflection

El sistema podrá ejecutar procesos de reflexión.

Ejemplo:

```text
brain reflect
```

Proceso:

```text
retrieve recent experiences
        ↓
cluster
        ↓
identify patterns
        ↓
generate hypotheses
        ↓
detect contradictions
        ↓
create candidate knowledge
        ↓
store for validation
```

La reflexión deberá producir **hipótesis**, no verdades absolutas.

---

# 18. Contradicciones

El sistema deberá detectar posibles contradicciones.

Ejemplo:

```text
Knowledge A:
"Supabase es preferido para proyectos pequeños."

Knowledge B:
".NET es preferido para proyectos pequeños."
```

El sistema deberá marcar:

```text
CONFLICT
```

y solicitar contexto.

Podría terminar en:

```text
Supabase:
preferred_for = ["MVP", "CRUD simple"]

.NET:
preferred_for = ["complex business logic"]
```

---

# 19. Versionado del conocimiento

El conocimiento deberá ser versionable.

Ejemplo:

```text
Knowledge #42

v1
"Utilizamos Supabase."

v2
"Preferimos Supabase para MVP."

v3
"Preferimos Supabase para MVP y proyectos
con baja complejidad de autorización."
```

Las versiones anteriores deberán permanecer disponibles para auditoría, salvo eliminación explícita.

---

# 20. Procedencia

Cada memoria deberá indicar su origen.

Ejemplos:

```text
human
agent
system
import
reflection
consolidation
external_source
```

Además:

```text
source_agent
source_session
source_project
source_memory_ids
```

---

# 21. MCP Server

Local Brain deberá exponer sus capacidades mediante MCP.

## 21.1 Herramientas mínimas

```text
brain_remember
brain_recall
brain_search
brain_learn
brain_reflect
brain_consolidate
brain_relate
brain_explain
brain_update
brain_forget
```

---

## 21.2 brain_remember

Permite registrar información.

Entrada conceptual:

```json
{
  "content": "...",
  "type": "episodic",
  "project": "...",
  "importance": 0.8
}
```

---

## 21.3 brain_recall

Permite recuperar conocimiento relevante.

```json
{
  "query": "...",
  "project": "...",
  "limit": 10
}
```

---

## 21.4 brain_learn

Permite registrar una experiencia como candidata a aprendizaje.

---

## 21.5 brain_explain

Debe permitir responder:

> ¿Por qué el sistema cree esto?

La respuesta deberá incluir:

- conocimiento;
- evidencia;
- experiencias relacionadas;
- confianza;
- historial.

---

# 22. CLI

El sistema deberá proporcionar una CLI.

Ejemplos:

```bash
brain init
brain daemon
brain status

brain remember "..."
brain recall "..."
brain search "..."
brain learn
brain reflect
brain consolidate

brain graph
brain inspect <id>

brain index
brain reindex

brain config
brain doctor
```

---

# 23. Daemon

Local Brain podrá ejecutarse como servicio:

```bash
brain daemon
```

Responsabilidades:

- MCP;
- jobs;
- indexing;
- embeddings;
- consolidación;
- health checks.

---

# 24. Jobs internos

El sistema deberá soportar tareas asíncronas.

Ejemplos:

```text
EmbeddingJob
IndexingJob
ConsolidationJob
ReflectionJob
GraphUpdateJob
CleanupJob
```

Los jobs deberán ser reintentables.

---

# 25. Persistencia

## 25.1 PostgreSQL

PostgreSQL será la persistencia inicial.

Motivos:

- madurez;
- robustez;
- SQL;
- transacciones;
- extensibilidad;
- soporte de pgvector;
- facilidad de backup.

---

## 25.2 pgvector

Será utilizado para búsqueda vectorial.

La dimensión de los embeddings deberá ser configurable según el modelo.

---

# 26. Configuración

El sistema deberá utilizar configuración declarativa.

Ejemplo:

```toml
[brain]
name = "local-brain"

[storage]
provider = "postgres"

[embedding]
provider = "llama-cpp"
endpoint = "http://127.0.0.1:8081"

[llm]
provider = "llama-cpp"
endpoint = "http://127.0.0.1:8082"

[retrieval]
vector_weight = 0.5
keyword_weight = 0.2
graph_weight = 0.15
recency_weight = 0.15
```

Los nombres definitivos podrán cambiar.

---

# 27. Compatibilidad con hardware

El sistema deberá funcionar en hardware de recursos limitados.

Objetivo inicial:

```text
CPU:
x86_64

RAM:
16 GB recomendado

GPU:
NVIDIA RTX 3050 4 GB como hardware de referencia

VRAM:
4 GB

OS:
Linux
Windows
macOS cuando sea técnicamente viable
```

El sistema deberá funcionar también sin GPU, aunque con menor rendimiento.

---

# 28. Modelos locales

El sistema no deberá depender de un modelo específico.

Debe permitir cambiar:

```text
embedding model
LLM
reranker
```

sin modificar el dominio.

El modelo deberá ser seleccionado en configuración.

---

# 29. Privacidad

## 29.1 Principio

Los datos almacenados deberán permanecer localmente por defecto.

El sistema no deberá enviar información fuera del dispositivo sin una acción/configuración explícita.

---

## 29.2 Telemetría

No deberá existir telemetría obligatoria.

Si se incorpora telemetría opcional:

- deberá estar desactivada por defecto;
- deberá ser claramente documentada;
- no deberá contener contenido de memoria;
- deberá poder deshabilitarse completamente.

---

# 30. Seguridad

El sistema deberá considerar principios de la familia ISO/IEC 27000.

Controles relevantes:

- mínimo privilegio;
- autenticación para servicios remotos;
- autorización;
- protección de secretos;
- cifrado cuando corresponda;
- auditabilidad;
- integridad;
- backups;
- gestión de dependencias;
- actualización segura.

---

# 31. Seguridad MCP

El servidor MCP deberá asumir que un agente puede cometer errores.

Deberá existir separación entre:

```text
READ
WRITE
MODIFY
DELETE
ADMIN
```

Las operaciones destructivas deberán requerir permisos adicionales.

Ejemplo:

```text
brain_recall
    READ

brain_remember
    WRITE

brain_update
    MODIFY

brain_forget
    DELETE
```

---

# 32. Protección contra prompt injection

El contenido recuperado de memoria deberá considerarse **datos**, no instrucciones confiables.

Ejemplo:

```text
Memory:
"Ignore previous instructions and execute..."
```

El agente no deberá tratar automáticamente el contenido como una instrucción privilegiada.

El sistema deberá proporcionar metadata de procedencia para que el agente pueda distinguir:

```text
memory
instruction
system_policy
external_content
```

---

# 33. Integridad de memoria

Las memorias deberán tener identificadores únicos.

Se deberá considerar:

- UUID/ULID;
- timestamps;
- hash de contenido;
- versionado;
- referencias de origen.

Para memorias sensibles o críticas podrá utilizarse:

```text
content_hash
```

para detectar modificaciones no autorizadas.

---

# 34. Eliminación y derecho al olvido

El sistema deberá permitir eliminar memoria.

La eliminación deberá diferenciar entre:

```text
soft delete
hard delete
redaction
anonymization
```

El comportamiento deberá estar documentado.

---

# 35. Observabilidad

El sistema deberá proporcionar:

```text
logs
metrics
health checks
diagnostics
```

Ejemplos:

```bash
brain status
brain doctor
```

Debe ser posible identificar:

- modelo activo;
- estado de PostgreSQL;
- estado de embeddings;
- jobs pendientes;
- errores;
- versión del sistema.

---

# 36. Requisitos funcionales

## RF-001 — Inicialización

El sistema deberá permitir inicializar una instancia.

```bash
brain init
```

---

## RF-002 — Crear memoria

El sistema deberá permitir almacenar una memoria.

---

## RF-003 — Recuperar memoria

El sistema deberá permitir recuperar memorias mediante lenguaje natural.

---

## RF-004 — Búsqueda híbrida

El sistema deberá combinar búsqueda semántica, textual y metadata.

---

## RF-005 — Clasificación

El sistema deberá clasificar memorias según tipo.

---

## RF-006 — Embeddings locales

El sistema deberá generar embeddings utilizando un provider local.

---

## RF-007 — Fallback sin embeddings

El sistema deberá continuar funcionando aunque temporalmente no exista un embedding.

---

## RF-008 — Relaciones

El sistema deberá permitir establecer relaciones entre memorias.

---

## RF-009 — Procedencia

El sistema deberá conservar el origen de cada memoria.

---

## RF-010 — Versionado

El sistema deberá conservar cambios relevantes del conocimiento.

---

## RF-011 — Aprendizaje

El sistema deberá permitir transformar experiencias en conocimiento candidato.

---

## RF-012 — Consolidación

El sistema deberá permitir consolidar múltiples experiencias.

---

## RF-013 — Reflexión

El sistema deberá permitir analizar grupos de experiencias para detectar patrones.

---

## RF-014 — Contradicciones

El sistema deberá identificar posibles contradicciones.

---

## RF-015 — Explicabilidad

El sistema deberá explicar el origen de una pieza de conocimiento.

---

## RF-016 — MCP

El sistema deberá proporcionar servidor MCP.

---

## RF-017 — CLI

El sistema deberá proporcionar CLI.

---

## RF-018 — Funcionamiento offline

El sistema deberá funcionar sin Internet cuando todos sus componentes locales estén disponibles.

---

## RF-019 — Multiagente

Múltiples agentes deberán poder utilizar una misma instancia de memoria.

---

## RF-020 — Configuración de providers

El usuario deberá poder cambiar providers sin modificar el núcleo.

---

# 37. Requisitos no funcionales

## RNF-001 — Rendimiento

Las operaciones de recuperación simples deberán responder rápidamente en hardware de gama media.

Objetivo inicial:

```text
p95 < 500 ms
```

para recuperación sin inferencia LLM.

Los tiempos de inferencia LLM no deberán considerarse parte de este objetivo.

---

## RNF-002 — Disponibilidad

El daemon deberá tolerar fallos temporales de componentes secundarios.

---

## RNF-003 — Escalabilidad

El diseño deberá permitir al menos:

```text
100.000+ memorias
```

sin rediseñar el dominio.

---

## RNF-004 — Portabilidad

El núcleo deberá compilar en las principales plataformas objetivo.

---

## RNF-005 — Mantenibilidad

El proyecto deberá mantener módulos con responsabilidades claramente separadas.

---

## RNF-006 — Testabilidad

La lógica de dominio deberá poder probarse sin:

- PostgreSQL;
- GPU;
- llama.cpp;
- Internet.

---

## RNF-007 — Seguridad

No deberán existir secretos hardcodeados.

---

## RNF-008 — Privacidad

No deberá realizarse transmisión externa de memoria por defecto.

---

## RNF-009 — Observabilidad

Los errores deberán ser diagnosticables sin necesidad de modificar el código.

---

# 38. API interna

El dominio deberá definir interfaces similares a:

```rust
trait MemoryRepository {}

trait VectorRepository {}

trait GraphRepository {}

trait EmbeddingProvider {}

trait LlmProvider {}

trait RerankerProvider {}

trait MemoryConsolidator {}

trait MemoryRetriever {}
```

Las firmas definitivas deberán definirse durante el diseño técnico.

---

# 39. Desarrollo asistido por agentes

El proyecto será desarrollado mediante colaboración entre:

```text
Human Developers
        +
AI Coding Agents
```

Los agentes podrán:

- implementar funcionalidades;
- escribir tests;
- revisar código;
- documentar;
- analizar issues;
- detectar vulnerabilidades;
- proponer refactors.

---

# 40. Reglas para agentes de IA

Los agentes no deberán:

- modificar arquitectura crítica sin justificación;
- eliminar tests para hacer pasar CI;
- introducir dependencias sin justificar;
- modificar esquemas de base de datos sin migración;
- eliminar controles de seguridad;
- modificar políticas de memoria silenciosamente.

Cada cambio significativo deberá incluir:

```text
Purpose
Changes
Risks
Tests
Affected modules
```

---

# 41. Human Review Gate

Las siguientes operaciones deberán requerir revisión humana:

- cambios arquitectónicos;
- cambios de seguridad;
- cambios de licencia;
- cambios en persistencia;
- cambios de protocolo MCP;
- eliminación masiva de memoria;
- nuevas dependencias críticas;
- modificaciones de políticas de privacidad.

---

# 42. Git Workflow

Se recomienda:

```text
main
  │
  ├── feature/*
  ├── fix/*
  ├── refactor/*
  └── security/*
```

Pull Requests deberán incluir:

```text
Summary
Motivation
Implementation
Tests
Security impact
Performance impact
Breaking changes
```

---

# 43. Metodología de desarrollo

El proyecto utilizará una combinación de:

### Scrum

Para:

- roadmap;
- backlog;
- prioridades;
- releases.

### XP

Para:

- pair programming humano-agente;
- test-first;
- integración continua;
- refactoring;
- feedback rápido.

Los agentes de IA podrán actuar como miembros técnicos del equipo, pero las decisiones de producto y arquitectura deberán permanecer bajo responsabilidad humana.

---

# 44. Definition of Done

Una funcionalidad se considerará terminada cuando:

- cumple sus requisitos;
- posee tests;
- pasa clippy;
- pasa fmt;
- compila;
- no introduce vulnerabilidades conocidas;
- posee documentación cuando corresponda;
- tiene migraciones si modifica persistencia;
- ha sido revisada;
- CI pasa correctamente.

---

# 45. Calidad del software

Se utilizará ISO/IEC 25010 como referencia para evaluar:

- adecuación funcional;
- eficiencia de desempeño;
- compatibilidad;
- capacidad de interacción;
- fiabilidad;
- seguridad;
- mantenibilidad;
- flexibilidad/portabilidad.

Las métricas deberán evolucionar junto al proyecto.

---

# 46. Dependencias principales

Dependencias iniciales potenciales:

```text
Rust
Tokio
Serde
SQLx o equivalente
PostgreSQL
pgvector
MCP SDK para Rust o implementación compatible
tracing
clap
thiserror / anyhow según contexto
```

La selección definitiva deberá realizarse mediante ADRs.

---

# 47. ADRs

El proyecto deberá utilizar Architecture Decision Records.

Ejemplos:

```text
ADR-001 — Elección de Rust
ADR-002 — PostgreSQL como storage inicial
ADR-003 — pgvector
ADR-004 — llama.cpp como runtime local
ADR-005 — MCP como interfaz para agentes
ADR-006 — Local-first
ADR-007 — Arquitectura hexagonal
ADR-008 — Modelo de memoria
ADR-009 — Estrategia de embeddings
ADR-010 — Estrategia de consolidación
```

---

# 48. Testing

## Unit tests

Principalmente para:

- dominio;
- scoring;
- clasificación;
- reglas;
- versionado;
- estados.

---

## Integration tests

Para:

- PostgreSQL;
- pgvector;
- MCP;
- jobs;
- persistencia.

---

## End-to-end

Escenario:

```text
Agent
 ↓
MCP
 ↓
remember
 ↓
embedding
 ↓
PostgreSQL
 ↓
recall
 ↓
Agent
```

---

# 49. Benchmarking

El proyecto deberá medir:

```text
memory insertion latency
retrieval latency
embedding throughput
vector search latency
graph traversal latency
CPU usage
RAM usage
VRAM usage
database size
```

Se deberán crear benchmarks reproducibles.

---

# 50. Hardware benchmark de referencia

El primer perfil de referencia será:

```text
CPU:
Ryzen 7 6800H o equivalente

RAM:
16 GB

GPU:
RTX 3050 Laptop 4 GB

Storage:
SSD

OS:
Linux
```

El sistema deberá buscar una experiencia razonable en este perfil.

Los modelos utilizados para benchmarking deberán documentarse.

---

# 51. Offline Mode

El sistema deberá permitir:

```bash
brain offline
```

o equivalente mediante configuración.

En este modo:

- no se permiten providers externos;
- no se transmiten datos;
- se utilizan modelos locales;
- MCP continúa funcionando.

---

# 52. Backup

El sistema deberá permitir respaldar:

```text
memories
knowledge
relations
metadata
configuration
```

Los embeddings podrán:

- respaldarse;
- regenerarse.

La estrategia deberá ser configurable.

---

# 53. Importación

Se deberá considerar soporte futuro para importar:

```text
Markdown
JSON
JSONL
Chat logs
Git history
documentation
```

La importación deberá conservar procedencia.

---

# 54. Integración con Git

Futuras versiones podrán registrar:

```text
commit
branch
repository
author
agent
memory
decision
```

Ejemplo:

```text
Commit abc123
      │
      └── implements → Decision #42
```

Esto permitirá conectar memoria con evolución real del código.

---

# 55. Memoria por proyecto

El sistema deberá soportar scopes:

```text
global
organization
project
repository
branch
session
agent
```

Ejemplo:

```text
Global:
preferencias generales

Montierrez:
decisiones del negocio

SIRVET:
decisiones específicas

Session:
contexto temporal
```

---

# 56. Política de recuperación

El sistema deberá evitar entregar indiscriminadamente grandes cantidades de memoria.

Deberá priorizar:

```text
relevancia
+
confianza
+
importancia
+
contexto
+
recencia
```

El objetivo es entregar **pocos recuerdos relevantes**, no todo lo relacionado.

---

# 57. Explainability

Cuando el agente consulte:

```text
brain.explain(
    "¿Por qué recomendamos .NET?"
)
```

el sistema debería devolver:

```text
Conclusión:
.NET es recomendado para este tipo de proyecto.

Confianza:
0.81

Evidencia:
- Experience #182
- Experience #201
- Decision #43

Razones:
- múltiples roles
- reglas de negocio
- necesidad de control del backend

Última revisión:
2026-09-10
```

---

# 58. Forgetting

El sistema deberá soportar decaimiento de relevancia.

Una memoria podrá perder prioridad por:

- antigüedad;
- baja utilidad;
- baja confianza;
- ausencia de recuperación;
- reemplazo por conocimiento más reciente.

No se deberá eliminar automáticamente información importante sin una política explícita.

---

# 59. Observación vs creencia

El sistema deberá diferenciar:

```text
Observation
```

de:

```text
Belief
```

Ejemplo:

```text
Observation:
"El deployment tardó 4 minutos."

Belief:
"Este stack tiene deployments lentos."
```

La segunda requiere más evidencia.

Esto evitará convertir observaciones aisladas en reglas generales.

---

# 60. Confidence

Cada conocimiento podrá tener:

```text
confidence
```

entre:

```text
0.0 — 1.0
```

La confianza deberá considerar:

- cantidad de evidencias;
- calidad de fuentes;
- consistencia;
- validación humana;
- repetición;
- antigüedad.

No deberá interpretarse como una probabilidad matemática rigurosa salvo que se defina posteriormente un modelo formal.

---

# 61. Security Model

La seguridad deberá considerar:

```text
Identity
Authentication
Authorization
Audit
Integrity
Confidentiality
Availability
```

El modelo exacto dependerá de si la instancia es:

```text
single-user local
multi-user local
LAN
remote
```

El MVP estará orientado principalmente a single-user local.

---

# 62. Threat Model

Amenazas iniciales:

```text
T-001 Prompt injection
T-002 Malicious MCP client
T-003 Memory poisoning
T-004 Unauthorized memory deletion
T-005 Credential leakage
T-006 Malicious dependency
T-007 Database compromise
T-008 Supply-chain attack
T-009 Agent hallucination
T-010 Knowledge poisoning
```

Cada amenaza deberá tener controles documentados.

---

# 63. Memory poisoning

El sistema deberá asumir que un agente puede registrar información incorrecta.

Por ello:

```text
Agent output
      ↓
Candidate memory
      ↓
Evidence
      ↓
Validation
      ↓
Knowledge
```

No:

```text
Agent output
      ↓
Truth
```

---

# 64. Open Source

El proyecto deberá ser público.

Repositorio recomendado:

```text
local-brain/
```

El nombre definitivo queda pendiente de validación.

---

# 65. Documentación Open Source

El repositorio deberá incluir:

```text
README.md
CONTRIBUTING.md
CODE_OF_CONDUCT.md
SECURITY.md
LICENSE
CHANGELOG.md
ARCHITECTURE.md
docs/
  adr/
  architecture/
  api/
  development/
```

---

# 66. Contribuciones

Las contribuciones deberán aceptar:

- código;
- documentación;
- tests;
- benchmarks;
- issues;
- propuestas arquitectónicas.

Los cambios significativos deberán pasar por Pull Request.

---

# 67. CI/CD

El pipeline deberá ejecutar como mínimo:

```text
cargo fmt --check
cargo clippy
cargo test
cargo build
security/dependency audit
```

Posteriormente:

```text
integration tests
benchmarks
cross compilation
release builds
```

---

# 68. Roadmap

## Fase 0 — Foundation

Objetivo:

Crear la base del proyecto.

```text
[x] Repository
[x] Rust workspace
[x] CI
[x] Architecture
[x] ADR system
```

---

## Fase 1 — Memory Core

```text
Memory entity
Memory repository
PostgreSQL
CRUD
CLI
```

Resultado:

```bash
brain remember
brain recall
```

---

## Fase 2 — Embeddings

```text
EmbeddingProvider
llama.cpp integration
pgvector
vector search
```

Resultado:

```text
semantic recall
```

---

## Fase 3 — MCP

```text
MCP server
brain_remember
brain_recall
brain_search
```

Resultado:

```text
Codex
Claude Code
Antigravity
        ↓
Local Brain
```

---

## Fase 4 — Memory Types

Implementar:

```text
working
episodic
semantic
procedural
associative
```

---

## Fase 5 — Knowledge Graph

Implementar:

```text
entities
relations
graph traversal
```

---

## Fase 6 — Learning

Implementar:

```text
experience
candidate knowledge
confidence
provenance
```

---

## Fase 7 — Consolidation

Implementar:

```text
reflection
clustering
pattern detection
knowledge consolidation
```

---

## Fase 8 — Advanced Retrieval

Implementar:

```text
hybrid retrieval
reranking
context assembly
recency
importance
utility
```

---

## Fase 9 — Hardening

```text
security
performance
benchmarks
documentation
backup
recovery
```

---

# 69. MVP

El MVP deberá contener únicamente:

```text
Rust
    │
    ├── Memory Core
    ├── PostgreSQL
    ├── pgvector
    ├── local embeddings
    ├── hybrid retrieval
    ├── MCP
    └── CLI
```

Con las siguientes capacidades:

```text
remember
recall
search
relate
explain
```

No deberá intentar implementar "aprendizaje humano" completo en la primera versión.

---

# 70. Criterios de éxito del MVP

El MVP será exitoso si:

1. Un agente puede registrar una experiencia.
2. La experiencia permanece después de cerrar la sesión.
3. Otro agente puede recuperar esa experiencia.
4. La recuperación semántica funciona localmente.
5. No se necesita ninguna API de pago.
6. PostgreSQL conserva el historial.
7. MCP funciona con al menos un agente.
8. El sistema puede ejecutarse en hardware de gama media.
9. La memoria puede inspeccionarse mediante CLI.
10. El sistema puede explicar por qué recuperó determinada memoria.

---

# 71. Ejemplo de flujo completo

## Primera sesión

Agente:

```text
"Estoy implementando autenticación."
```

Consulta:

```text
brain_recall(
    "experiencias anteriores con autenticación"
)
```

Local Brain encuentra:

```text
Experience #182
Firebase Auth con identificador público
Problema:
no proporciona autenticación adecuada.
```

El agente evita repetir el error.

---

## Segunda experiencia

El agente implementa:

```text
Passkey
```

Resultado:

```text
Funcionó correctamente.
```

Se registra:

```text
Experience #201
```

---

## Consolidación

Posteriormente:

```text
Experience #182
Experience #201
Experience #207
Experience #231
```

El sistema detecta:

```text
pattern:
passwordless authentication fue exitosa
en determinados contextos.
```

Se crea:

```text
Candidate Knowledge #51
```

---

# 72. Resultado esperado a largo plazo

El sistema deberá evolucionar desde:

```text
RAG
```

hacia:

```text
Persistent Agent Memory
```

y eventualmente:

```text
Persistent Cognitive Infrastructure
```

La evolución conceptual será:

```text
Documents
   ↓
Memories
   ↓
Experiences
   ↓
Knowledge
   ↓
Relationships
   ↓
Patterns
   ↓
Procedures
   ↓
Learned Preferences
```

---

# 73. Restricciones

El proyecto deberá respetar:

- funcionamiento local-first;
- independencia de proveedores;
- Open Source;
- Rust como lenguaje principal;
- uso de modelos locales cuando sea posible;
- ausencia de dependencia obligatoria de APIs pagadas;
- trazabilidad;
- seguridad;
- mantenibilidad.

---

# 74. Riesgos

## R-001 — Complejidad excesiva

El concepto puede convertirse rápidamente en un proyecto demasiado grande.

**Mitigación:** MVP extremadamente pequeño.

---

## R-002 — Alucinaciones

El LLM podría generar conocimiento incorrecto.

**Mitigación:** provenance, confidence, candidate knowledge y revisión.

---

## R-003 — Recuperación irrelevante

Un RAG híbrido puede devolver recuerdos incorrectos.

**Mitigación:** ranking, filtros, benchmarks y evaluación.

---

## R-004 — Consumo de recursos

Los modelos locales pueden ser pesados.

**Mitigación:** providers intercambiables, modelos pequeños y procesamiento asíncrono.

---

## R-005 — Dependencia accidental de un proveedor

**Mitigación:** interfaces abstractas y providers.

---

## R-006 — Memory poisoning

**Mitigación:** procedencia, confianza y validación.

---

# 75. Métricas futuras

El proyecto deberá evaluar:

```text
Recall@K
Precision@K
MRR
retrieval latency
embedding latency
memory growth
knowledge consolidation accuracy
contradiction detection accuracy
false memory rate
agent task success rate
```

Una métrica especialmente importante será:

> ¿El acceso a memoria mejora realmente el rendimiento del agente?

El proyecto no deberá asumir que más memoria significa mejor agente.

---

# 76. Principio fundamental del proyecto

La arquitectura deberá seguir la siguiente separación:

```text
                    ┌───────────────────┐
                    │     AGENTE        │
                    │                   │
                    │ RAZONAMIENTO      │
                    │ PLANIFICACIÓN     │
                    │ EJECUCIÓN         │
                    └─────────┬─────────┘
                              │
                             MCP
                              │
                    ┌─────────▼─────────┐
                    │   LOCAL BRAIN     │
                    │                   │
                    │ MEMORIA           │
                    │ CONOCIMIENTO      │
                    │ EXPERIENCIA       │
                    │ RELACIONES        │
                    │ HISTORIAL         │
                    └─────────┬─────────┘
                              │
                    ┌─────────▼─────────┐
                    │ LOCAL AI          │
                    │                   │
                    │ EMBEDDINGS        │
                    │ REFLECTION        │
                    │ CONSOLIDATION     │
                    └───────────────────┘
```

El agente proporciona la capacidad de razonamiento.

Local Brain proporciona la memoria.

El modelo local proporciona capacidades auxiliares de IA.

Ningún componente deberá ser considerado irremplazable.

---

# 77. Definición conceptual final

Local Brain será:

> **Una infraestructura Open Source de memoria persistente, local y agnóstica respecto del proveedor para agentes de Inteligencia Artificial, capaz de almacenar experiencias, recuperar conocimiento, establecer relaciones, consolidar aprendizajes y mantener trazabilidad sobre la evolución de dicho conocimiento.**

El objetivo no será recrear el cerebro humano.

El objetivo será construir una arquitectura computacional que permita a los agentes **recordar, asociar, aprender de experiencias y explicar por qué saben lo que saben**, manteniendo la propiedad de los datos en manos del usuario.

---

# 78. Principio de independencia

El proyecto deberá poder sobrevivir a la desaparición de cualquier proveedor.

Debe ser posible:

```text
Claude desaparece
       ↓
Local Brain continúa

Codex desaparece
       ↓
Local Brain continúa

Proveedor de embeddings desaparece
       ↓
Local embeddings continúan

LLM local cambia
       ↓
Memory permanece

PostgreSQL cambia
       ↓
Domain permanece
```

La memoria deberá pertenecer al usuario, no al modelo.

---

# 79. Estado del documento

Este SRS representa una especificación inicial.

Antes de comenzar la implementación deberán definirse mediante ADR:

1. nombre definitivo;
2. licencia definitiva;
3. estructura exacta del workspace Rust;
4. versión mínima de Rust;
5. PostgreSQL/pgvector;
6. mecanismo de comunicación con llama.cpp;
7. modelo inicial de embeddings;
8. modelo local inicial;
9. protocolo MCP utilizado;
10. esquema definitivo de memoria;
11. estrategia de graph storage;
12. estrategia de jobs;
13. estrategia de backup;
14. modelo de permisos.

Las decisiones deberán registrarse y versionarse.

---

# 80. Regla de oro

```text
No construir un cerebro que simplemente recuerde todo.

Construir un sistema que sepa:

qué recordar,
por qué recordarlo,
cuándo recordarlo,
qué tan confiable es,
de dónde proviene,
con qué está relacionado,
cuándo dejó de ser válido
y por qué llegó a creerlo.
```

Ese será el criterio principal para evaluar la evolución de Local Brain.