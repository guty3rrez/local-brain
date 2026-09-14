## 📌 Resumen del Cambio

<!-- Explica brevemente qué resuelve este PR y el contexto del cambio -->

## 🎯 Motivación y Vínculo con el Backlog

- **Fase / Historia del Backlog**: <!-- Ejemplo: Fase 1 [F1-03] -->
- **Issue relacionado**: Fixes #<!-- número de issue -->

## 🛠️ Implementación Técnica

<!-- Describe los componentes modificados y decisiones de diseño adoptadas -->
- 

## 🤖 Declaración de Autoría y Asistencia de IA (SRS §39)

- [ ] Este PR fue escrito 100% por un desarrollador humano.
- [ ] Este PR fue generado o asistido por un agente de IA.
  - **Nombre del Agente / Modelo**: <!-- Claude Code, Antigravity, Cursor, etc. -->
  - **Confirmación**: Se verificó el cumplimiento estricto de [`AGENTS.md`](file:///home/guty_3rrez/Proyectos/local-brain/AGENTS.md).

## 🚦 Puerta de Revisión Humana (*Human Review Gate* - SRS §41)

¿Este cambio afecta alguno de los siguientes puntos críticos?
- [ ] Arquitectura hexagonal o límites de crates
- [ ] Seguridad, modelo de amenazas o mitigación de prompt injection
- [ ] Esquema de base de datos relacional / migraciones SQLx
- [ ] Protocolo y compatibilidad de herramientas MCP
- [ ] Términos de la licencia AGPLv3 o dependencias externas
- [ ] Ninguno de los anteriores (cambio rutinario de aplicación / dominio / tests)

## 🧪 Arsenal de Testing Ejecutado (Obligatorio)

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace --lib` (Tests unitarios de dominio sin dependencias externas)
- [ ] `cargo test --workspace --test '*'` (Tests de integración con PostgreSQL/pgvector si aplica)
- [ ] `cargo test --test cucumber` (Escenarios BDD Gherkin)
- [ ] `cargo mutants -p <crate>` (Tests de mutación evaluados; score reportado: ____%)
- [ ] `cargo audit` (Cero vulnerabilidades en crates)

## ⚠️ Riesgos y Rompimiento de Compatibilidad

- **Impacto en Rendimiento**: <!-- ¿Afecta latencias p95? -->
- **Breaking Changes**: <!-- ¿Rompe APIs o interfaces previas? -->
