# ADR-0003: Adopción de PostgreSQL 17 + SQLx y UUID v7 para Persistencia Relacional

* **Estado:** Aceptado
* **Decisores:** Local Brain Core Team & Contributors
* **Fecha:** 2026-09-14

## Contexto y Declaración del Problema

Local Brain requiere una capa de persistencia duradera, estructurada y de alto rendimiento en local para almacenar unidades atómicas de memoria cognitiva (SRS §9, §25). La persistencia debe cumplir con:
1. Soberanía absoluta de datos y operación 100% offline (SRS §4.1).
2. Soporte nativo para búsquedas relacionales, filtros por proyecto y tipo de memoria, metadata flexible en JSONB y transacciones ACID.
3. Compatibilidad con extensiones futuras como `pgvector` para búsqueda semántica vectorial (Fase 2).
4. Evitar fragmentación de índices en disco y garantizar localidad de caché bajo inserciones continuas.
5. Preservar la compilación y ejecución de tests unitarios puros sin dependencias obligatorias de base de datos activa en CI (RNF-006).

## Opciones Consideradas

* **PostgreSQL 17 + SQLx (dinámico) + UUID v7 (RFC 9562)**:
  - PostgreSQL es el estándar de la industria en fiabilidad, ACID, JSONB y soporte vectorial (`pgvector`).
  - SQLx ofrece manejo asíncrono sobre `tokio` sin ORM pesado, con control explícito de consultas parametrizadas y migraciones embebidas con `sqlx::migrate!`.
  - El uso de consultas dinámicas en tiempo de ejecución evita el acoplamiento en tiempo de compilación a una base de datos activa (`sqlx-data.json` o base de datos en CI), manteniendo RNF-006 intacto.
  - UUID v7 (RFC 9562) aporta ordenamiento cronológico por timestamp en milisegundos en la cabecera del ID, garantizando inserciones monotónicas en árboles B-Tree sin page splits ni fragmentación de índices.
* **SQLite / rusqlite / libsql**:
  - Excelente portabilidad embebida, pero soporte limitado o complejo para extensiones vectoriales de producción masiva comparado con `pgvector`, y concurrencia de escritura más restringida.
* **Diesel (ORM)**:
  - Verificación estricta en compilación pero mayor fricción con async/tokio y migraciones dinámicas embebidas ligeras.

## Decisión Adoptada

Opción elegida: **PostgreSQL 17 + SQLx + UUID v7 (RFC 9562)**.

1. **Tabla `memories` e integridad relacional**: Clave primaria UUID v7, constraints numéricos en rangos [0.0, 1.0], hashes de integridad SHA-256 inmutables y metadatos extensibles en JSONB indexados con GIN.
2. **Migraciones idempotentes versionadas**: Gestionadas en `crates/brain-infrastructure/migrations/` y aplicadas programáticamente mediante `sqlx::migrate!`.
3. **UUID v7 como estándar de identidad**: `MemoryId::new()` genera identificadores UUID v7 compatibles nativamente con el tipo `UUID` (16 bytes) de PostgreSQL.
4. **Desacoplamiento de CI**: `cargo check`, `cargo clippy` y `cargo test --lib` compilan y pasan en milisegundos sin requerir conexión a PostgreSQL.

### Consecuencias Positivas

* Altísimo rendimiento de inserción y lectura en PostgreSQL gracias al orden monotónico de UUID v7 en índices B-Tree.
* Capacidad de almacenar y consultar metadatos semiestructurados (`provenance`, `metadata`) vía JSONB y operadores GIN (`@>`).
* Compatibilidad total para añadir `pgvector` en la Fase 2 sin alterar la arquitectura base.
* Cumplimiento estricto de licencias (SQLx es dual MIT/Apache-2.0, compatible con AGPLv3).

### Consecuencias Negativas / Riesgos

* Requiere levantar el servicio auxiliar de PostgreSQL (mediante `docker compose up -d postgres`) para ejecutar pruebas de integración y BDD locales.

## Enlaces y Referencias

* [README — Persistencia y Servicios Locales](file:///home/guty_3rrez/Proyectos/local-brain/README.md#-inicio-r%C3%A1pido-quickstart)
* [AGENTS.md — Autoridad Determinista](file:///home/guty_3rrez/Proyectos/local-brain/AGENTS.md#13-no-confiar-ciegamente-en-el-llm-autoridad-determinista)
* [RFC 9562 — Universally Unique IDentifiers (UUID)](https://www.rfc-editor.org/rfc/rfc9562.html)
* [ADR-0001 — Adopción de Rust](file:///home/guty_3rrez/Proyectos/local-brain/docs/adr/0001-lenguaje-rust.md)
