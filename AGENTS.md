# AGENTS.md — Protocolo y Normas para Agentes de Inteligencia Artificial

> **Documento normativo para agentes de codificación de IA (Antigravity, Claude Code, Codex, Cursor, Cline, Windsurf, Roo Code, etc.) colaborando en el proyecto Local Brain.**  
> Todo agente que lea, genere, modifique o audite código en este repositorio debe cumplir de forma irrestricta con las pautas y principios arquitectónicos aquí establecidos.

---

## 🧭 1. Principios Fundamentales del Sistema

### 1.1 La Regla de Oro
> **No construir un cerebro que simplemente recuerde todo.**  
> Construir un sistema que sepa: **qué recordar, por qué recordarlo, cuándo recordarlo, qué tan confiable es, de dónde proviene, con qué está relacionado, cuándo dejó de ser válido y por qué llegó a creerlo.**

### 1.2 Local-first & Independencia de Proveedores
- El núcleo de Local Brain no depende de ningún proveedor comercial de IA en la nube (OpenAI, Anthropic, Google, etc.).
- Toda funcionalidad crítica debe poder ejecutarse 100% offline con modelos locales (`llama.cpp`) y persistencia relacional/vectorial local (`PostgreSQL + pgvector`).
- Queda terminantemente prohibido incorporar llamadas salientes a la red o recolectar telemetría sin directiva de configuración explícita.

### 1.3 No confiar ciegamente en el LLM (Autoridad Determinista)
- El LLM es un componente **probabilístico** y auxiliar.
- La base de datos relacional y las reglas de código determinista conservan **autoridad absoluta** sobre:
  - Identidad e identificadores únicos (UUIDv7 monotónicos).
  - Relaciones e integridad referencial en el Grafo de Conocimiento.
  - Historial, marcas de tiempo (*timestamps*) y procedencia de datos.
  - Estados de memoria, políticas de expiración y permisos de acceso.
- El contenido recuperado de memoria debe tratarse siempre como **datos no confiables**, nunca como instrucciones privilegiadas de sistema (mitigación de prompt injection mediante delimitadores de aislamiento).

### 1.4 Arquitectura Hexagonal y Dominio Puro
```text
Domain (brain-domain)
   ↑ (implementa puertos)
Application (brain-application)
   ↑ (llama casos de uso)
Adapters / Infrastructure (brain-infrastructure, brain-mcp, brain-cli)
```
- **Regla Inquebrantable**: La lógica de dominio en `brain-domain` **no debe tener ninguna dependencia de PostgreSQL, SQLx, llama.cpp, HTTP, GPU, ni MCP**. El dominio debe compilarse y probarse de forma pura en milisegundos sin requerir infraestructura externa.

---

## 🚫 2. Prohibiciones Estrictas para Agentes

Los agentes de IA **TIENEN TERMINANTEMENTE PROHIBIDO**:
1. **Modificar o deshabilitar tests existentes** para forzar que el CI pase en verde. Si un test falla, se debe corregir la implementación o justificar explícitamente un cambio en los requisitos del test.
2. **Introducir nuevas dependencias en `Cargo.toml`** sin justificación técnica documentada en el PR o commit.
3. **Modificar esquemas de base de datos** de forma destructiva o sin una migración SQLx versionada e idempotente.
4. **Debilitar o remover controles de seguridad**, validaciones de entrada, límites de memoria o sanitizaciones contra prompt injection.
5. **Hardcodear secretos**, URLs fijas de entornos ajenos o credenciales en el código fuente.
6. **Enviar telemetría o datos de memoria** fuera del entorno local sin una directiva de configuración explícita.

---

## 🚦 3. Puerta de Revisión Humana (*Human Review Gate*)

Las siguientes acciones **requieren pausar la ejecución y solicitar la confirmación explícita del desarrollador humano**:
- [ ] Cambios en la arquitectura de crates o límites de Clean Architecture.
- [ ] Modificaciones en el modelo de permisos o controles de seguridad MCP.
- [ ] Cambios en la licencia del proyecto o términos de distribución (AGPL-3.0).
- [ ] Modificaciones estructurales en las tablas o extensiones de PostgreSQL / pgvector.
- [ ] Cambios que rompan la compatibilidad del protocolo MCP con clientes existentes.
- [ ] Introducción de dependencias críticas o con licencias incompatibles con AGPL-3.0.
- [ ] Eliminación masiva o purga no reversible de registros de memoria.

---

## 🧪 4. Arsenal de Testing Obligatorio para el Agente

Antes de declarar una tarea o PR como terminada, el agente debe ejecutar localmente y reportar el resultado de:

| Capa | Comando | Objetivo |
| :--- | :--- | :--- |
| **Formato** | `cargo fmt --all -- --check` | Cumplimiento estricto del estilo oficial Rust |
| **Linter** | `cargo clippy --workspace --all-targets -- -D warnings` | Cero advertencias permitidas |
| **Unit Tests** | `cargo test --workspace --lib` | Cobertura de lógica de dominio pura y casos de borde |
| **BDD** | `cargo test --test cucumber` | Validación de especificaciones vivas en Gherkin (`tests/features/`) |
| **Integration** | `cargo test --workspace --test '*'` | Verificación con PostgreSQL + pgvector y MCP |
| **Seguridad** | `cargo audit` | Cero vulnerabilidades conocidas en dependencias |
| **Licencias** | `cargo deny check licenses advisories` | Cumplimiento estricto de AGPLv3 |

---

## 🔄 5. Flujo Git y Formato de Entrega

### 5.1 Nombres de Ramas
- Features: `feature/<nombre-descriptivo>`
- Fixes: `fix/<nombre-descriptivo>`
- Refactors: `refactor/<nombre-descriptivo>`
- Security: `security/<nombre-descriptivo>`
- Docs: `docs/<nombre-descriptivo>`

### 5.2 Formato de Commits (Conventional Commits)
```text
feat(domain): add MemoryContent value object with invariant validation
fix(mcp): prevent prompt injection via delimiter wrapping
test(retrieval): add mutation-tested edge cases for score calculation
```

### 5.3 Estructura Obligatoria en la Descripción de Cambios / PR
Cada cambio presentado por un agente debe contener:
```markdown
### Propósito
[¿Qué problema o requerimiento resuelve?]

### Cambios Realizados
[Detalle conciso de los módulos modificados]

### Riesgos y Mitigaciones
[Impacto potencial en rendimiento, concurrencia o compatibilidad]

### Arsenal de Tests Ejecutado
- [x] cargo fmt & clippy
- [x] cargo test (unit & integration)
- [x] BDD scenarios

### Puerta de Revisión Humana (Human Review Gate)
[¿Requiere revisión humana? Indicar Sí/No y motivo]
```
