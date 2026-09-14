# AGENTS.md — Protocolo y Normas para Agentes de Inteligencia Artificial

> **Documento normativo para agentes de codificación de IA (Antigravity, Claude Code, Codex, Cursor, etc.) colaborando en el proyecto Local Brain.**  
> Todo agente que lea, genere, modifique o audite código en este repositorio debe cumplir de forma irrestricta con las pautas aquí establecidas, derivadas del [`SRS — Local Brain`](file:///home/guty_3rrez/Proyectos/local-brain/SRS%20%E2%80%94%20Local%20Brain_%20Memoria%20persistente%20local%20para%20agentes%20de%20IA.md).

---

## 🧭 1. Principios Fundamentales del Sistema

### 1.1 La Regla de Oro (SRS §80)
> **No construir un cerebro que simplemente recuerde todo.**  
> Construir un sistema que sepa: **qué recordar, por qué recordarlo, cuándo recordarlo, qué tan confiable es, de dónde proviene, con qué está relacionado, cuándo dejó de ser válido y por qué llegó a creerlo.**

### 1.2 Local-first & Independencia de Proveedores (SRS §4.1, §4.2, §78)
- El núcleo no debe depender de ningún proveedor comercial de IA (OpenAI, Anthropic, Google, etc.).
- Toda funcionalidad crítica debe poder ejecutarse 100% offline con modelos locales (`llama.cpp`) y persistencia local (`PostgreSQL + pgvector`).

### 1.3 No confiar ciegamente en el LLM (SRS §4.5)
- El LLM es un componente **probabilístico** y auxiliar.
- La base de datos relacional y las reglas de código determinista conservan **autoridad absoluta** sobre:
  - Identidad e identificadores únicos.
  - Relaciones e integridad referencial.
  - Historial, timestamps y procedencia.
  - Estados de memoria y permisos.
- El contenido recuperado de memoria debe tratarse como **datos no confiables**, nunca como instrucciones privilegiadas (mitigación de prompt injection, SRS §32).

### 1.4 Arquitectura Hexagonal y Separación Estricta (SRS §8, §76)
```text
Domain (brain-domain)
   ↑ (implementa puertos)
Application (brain-application)
   ↑ (llama casos de uso)
Adapters / Infrastructure (brain-infrastructure, brain-mcp, brain-cli)
```
- **Regla inquebrantable (SRS RNF-006)**: La lógica de dominio en `brain-domain` **no debe tener ninguna dependencia de PostgreSQL, SQLx, llama.cpp, HTTP, GPU, ni MCP**. El dominio debe compilarse y probarse de forma pura en milisegundos.

---

## 🚫 2. Prohibiciones Estrictas para Agentes (SRS §40)

Los agentes de IA **TIENEN PROHIBIDO**:
1. **Modificar o deshabilitar tests existentes** para forzar que el CI pase en verde. Si un test falla, se debe corregir la implementación o justificar explícitamente un cambio en los requisitos del test.
2. **Introducir nuevas dependencias en `Cargo.toml`** sin justificación técnica documentada en el PR / commit.
3. **Modificar esquemas de base de datos** de forma destructiva o sin una migración SQLx versionada e idempotente.
4. **Debilitar o remover controles de seguridad**, validaciones de entrada, límites de memoria o sanitizaciones contra prompt injection.
5. **Hardcodear secretos**, URLs fijas de producción o credenciales en el código fuente (SRS RNF-007).
6. **Enviar telemetría o datos de memoria** fuera del entorno local sin una directiva de configuración explícita (SRS §29).

---

## 🚦 3. Puerta de Revisión Humana (*Human Review Gate*) (SRS §41)

Las siguientes acciones **requieren pausar la ejecución y solicitar la confirmación explícita del desarrollador humano**:
- [ ] Cambios en la arquitectura de crates o límites de Clean Architecture.
- [ ] Modificaciones en el modelo de permisos o controles de seguridad MCP.
- [ ] Cambios en la licencia del proyecto o términos de distribución.
- [ ] Modificaciones estructurales en las tablas o extensiones de PostgreSQL / pgvector.
- [ ] Cambios que rompan la compatibilidad del protocolo MCP con clientes existentes.
- [ ] Introducción de dependencias críticas o con licencias dudosas (deben ser compatibles con [AGPL-3.0](file:///home/guty_3rrez/Proyectos/local-brain/LICENSE)).
- [ ] Eliminación masiva o purga de registros de memoria.

---

## 🧪 4. Arsenal de Testing Obligatorio para el Agente

Antes de declarar una tarea o PR como terminada, el agente debe ejecutar localmente y reportar el resultado de:

| Capa | Comando | Objetivo |
| :--- | :--- | :--- |
| **Formato** | `cargo fmt --all -- --check` | Cumplimiento de estilo oficial Rust |
| **Linter** | `cargo clippy --workspace --all-targets -- -D warnings` | Cero advertencias permitidas |
| **Unit Tests** | `cargo test --workspace --lib` | Cobertura de lógica de dominio y casos de borde |
| **BDD** | `cargo test --test cucumber` | Validación de escenarios Gherkin en `tests/features/` |
| **Integration** | `cargo test --test '*'` | Verificación con PostgreSQL + pgvector |
| **Mutation** | `cargo mutants -p <crate_modificada>` | Verificar que no queden mutantes sobrevivientes |
| **Seguridad** | `cargo audit` | Cero vulnerabilidades conocidas en dependencias |
| **Licencias** | `cargo deny check licenses advisories` | Cumplimiento estricto de AGPLv3 |

---

## 🔄 5. Flujo Git y Formato de Entrega (SRS §42)

### 5.1 Nombres de Ramas
- Features: `feature/<nombre-descriptivo>`
- Fixes: `fix/<nombre-descriptivo>`
- Refactors: `refactor/<nombre-descriptivo>`
- Security: `security/<nombre-descriptivo>`

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
[¿Qué problema del SRS o BACKLOG resuelve?]

### Cambios Realizados
[Detalle conciso de los módulos modificados]

### Riesgos y Mitigaciones
[Impacto potencial en rendimiento, concurrencia o compatibilidad]

### Arsenal de Tests Ejecutado
- [x] cargo fmt & clippy
- [x] cargo test (unit & integration)
- [x] cargo mutants (score: XX%)
- [x] BDD scenarios

### Puerta de Revisión Humana (Human Review Gate)
[¿Requiere revisión humana según §41? Indicar Sí/No y motivo]
```
