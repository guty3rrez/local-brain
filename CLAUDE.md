# CLAUDE.md — Directrices de Desarrollo para Claude Code

> **Proyecto:** Local Brain  
> **Propósito:** Memoria persistente local, agéntica y orientada a conocimiento para agentes de IA  
> **Documento rector:** [`SRS — Local Brain`](file:///home/guty_3rrez/Proyectos/local-brain/SRS%20%E2%80%94%20Local%20Brain_%20Memoria%20persistente%20local%20para%20agentes%20de%20IA.md) y [`AGENTS.md`](file:///home/guty_3rrez/Proyectos/local-brain/AGENTS.md)  
> **Licencia:** GNU AGPLv3

---

## 🛠️ Comandos Rápidos de Trabajo

```bash
# Formateo y Linter
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings

# Testing Unitario (rápido, sin dependencias externas)
cargo test --workspace --lib

# Testing de Integración (requiere PostgreSQL local / docker)
cargo test --workspace --test '*'

# Testing BDD (Gherkin)
cargo test --test cucumber

# Pruebas de Mutación
cargo mutants --workspace -v

# Cobertura de Código (objetivo >= 85%)
cargo llvm-cov --workspace --html

# Auditoría de Seguridad y Licencias
cargo audit
cargo deny check licenses advisories
```

---

## 🏗️ Estructura y Arquitectura

El proyecto sigue arquitectura hexagonal estricta:
- `crates/brain-domain`: Entidades, Value Objects, eventos y puertos (traits). **Cero dependencias externas** (sin SQLx, sin red, sin MCP).
- `crates/brain-application`: Casos de uso y orquestación de servicios.
- `crates/brain-infrastructure`: Implementaciones concretas de puertos (PostgreSQL con SQLx, pgvector, cliente HTTP llama.cpp).
- `crates/brain-mcp`: Servidor Model Context Protocol sobre stdio y SSE.
- `crates/brain-cli`: Línea de comandos con `clap`.
- `crates/brain-daemon`: Daemon en segundo plano para jobs asíncronos y MCP.

---

## ⚠️ Reglas Inviolables para Claude Code

1. **Nunca debilites tests existentes**: Prohibido borrar asserts o silenciar tests que fallan para lograr un build verde en CI.
2. **Dominio puro (SRS RNF-006)**: `brain-domain` no debe depender de bases de datos, redes ni GPU. Debe probarse en milisegundos.
3. **Autoridad Determinista**: Trata al LLM como un componente probabilístico auxiliar. La base de datos relacional y las reglas de código mandan sobre identidad, versionado, timestamps y permisos.
4. **Human Review Gate (SRS §41)**: Consulta al humano antes de:
   - Modificar esquemas de persistencia o migraciones.
   - Alterar contratos del protocolo MCP.
   - Añadir dependencias externas en `Cargo.toml`.
   - Modificar políticas de seguridad o borrado de memoria.
5. **Formato de Commits**: Usa *Conventional Commits* (`feat:`, `fix:`, `test:`, `refactor:`, `docs:`, `chore:`).
