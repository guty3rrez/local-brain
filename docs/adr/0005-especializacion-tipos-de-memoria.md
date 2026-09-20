# ADR-0005: Especialización de Tipos de Memoria Cognitiva (Working, Episodic, Semantic, Procedural, Associative)

* **Estado:** Aceptado
* **Decisores:** Local Brain Core Team & Contributors
* **Fecha:** 2026-09-16

## Contexto y Declaración del Problema

Local Brain requiere estructurar la memoria cognitiva en cinco categorías especializadas definidas en la arquitectura cognitiva humana y adaptadas a agentes de IA autónomos (SRS §10, §68, F4-01 a F4-05):
1. **Working Memory (§10.1):** Memoria volátil de contexto inmediato, ligada al ciclo de vida de una sesión y con expiración automática mediante TTL o cierre de sesión.
2. **Episodic Memory (§10.2):** Experiencias y sucesos concretos vividos por el agente, estructurados en la tríada situación/contexto, acción y resultado empírico.
3. **Semantic Memory (§10.3):** Conocimiento generalizado comprobado (principios, directrices, reglas de diseño). Exige por invariante de dominio un respaldo de evidencias empíricas cuando el nivel de certidumbre/confianza es alto (confianza ≥ 0.8).
4. **Procedural Memory (§10.4):** Recetas y procedimientos técnicos ordenados secuencialmente (pasos 1..N sin omisiones ni desorden), con versión de pasos y comandos reproducibles.
5. **Associative Memory (§10.5):** Vínculos y relaciones conceptuales entre entidades (`source_concept`, `target_concept`, `predicate`, `strength`) para navegación asociativa.

El diseño debe garantizar:
- Invariantes estrictas de dominio y tipos ricos en Rust (SRS RNF-006: dominio puro sin dependencias de infraestructura).
- Persistencia robusta y eficiente en PostgreSQL sin proliferación caótica de tablas ni duplicación de esquemas.
- Búsqueda relacional y semántica transparente sobre todos los tipos de memoria.
- Compatibilidad con el protocolo MCP y CLI.

## Opciones Consideradas

* **Opción A: Metadatos genéricos no estructurados (`serde_json::Value` sin tipado en dominio):**
  - *Ventajas:* Rápido de implementar inicialmente sin cambios en structs de dominio.
  - *Desventajas:* Viola la Regla de Oro (SRS §80) y RNF-006. Las invariantes críticas (p. ej. validación de pasos procedimentales o evidencia obligatoria para alta confianza semántica) quedarían desprotegidas o delegadas al frontend/llamador, permitiendo corrupción silenciosa del modelo cognitivo.
* **Opción B: Tablas relacionales independientes por tipo de memoria (`working_memories`, `episodic_memories`, `semantic_memories`, etc.):**
  - *Ventajas:* Esquema relacional 100% normalizado.
  - *Desventajas:* Fragmenta los índices vectoriales HNSW, complica las búsquedas globales multicriterio y el retrieval unificado (`RecallUseCase`), requiriendo complejas uniones `UNION ALL` y dificultando la evolución del esquema.
* **Opción C: Enum fuertemente tipado en Dominio (`MemoryTypeData`) + Estructuras Dedicadas + Columna `metadata JSONB` y `expires_at TIMESTAMPTZ` en `memories`:**
  - *Ventajas:*
    1. **Dominio Puro y Seguro:** Tipos ricos (`WorkingMemoryData`, `EpisodicMemoryData`, `SemanticMemoryData`, `ProceduralMemoryData`, `AssociativeMemoryData`) con validación estricta de invariantes en constructores.
    2. **Persistencia Unificada:** Conserva la tabla central `memories` con vector HNSW único de 768 dimensiones.
    3. **Indexación y Eficiencia:** La columna `expires_at` indexada permite expiración/purga eficiente en O(log N). El índice GIN sobre `metadata` permite consultas JSONB directas (sesión, conceptos asociativos).
    4. **Retrocompatibilidad:** Si `type_data` no se especifica, el sistema utiliza `MemoryTypeData::None`, manteniendo 100% de compatibilidad con recuerdos existentes creados en fases anteriores.
  - *Desventajas:* Requiere serialización/deserialización JSON serde en los límites del adaptador de persistencia.

## Decisión Adoptada

Se adopta la **Opción C**:
1. **Modelos de Dominio Especializados en `brain-domain::model::specialized`:**
   - Structs dedicados para cada uno de los 5 tipos.
   - Invariantes de dominio: `InsufficientEvidenceForHighConfidence` (confianza ≥ 0.8 exige evidencia), `InvalidStepSequence` (pasos 1..N consecutivos), `EmptyProcedureSteps`, `InvalidTtl`, `InvalidSessionId`, `InvalidAssociation`.
   - Soporte de TTL y expiración activa en `Memory::is_active()` y `Memory::is_expired_at(now)`.
2. **Puertos de Repositorio Extendidos en `brain-domain::ports`:**
   - Métodos: `find_active_by_session`, `expire_session`, `purge_expired`, `find_associations`.
   - Implementación pura en `InMemoryMemoryRepository`.
3. **Casos de Uso de Aplicación en `brain-application`:**
   - Constructores especializados y semánticos en `RememberCommand` (`episodic`, `working`, `procedural`, `semantic`, `associative`).
   - Casos de uso dedicados `ExpireSessionUseCase` y `PurgeExpiredUseCase`.
   - Filtros por `session_id` y `concept` en `RecallQuery` y `RecallUseCase`.
4. **Persistencia en PostgreSQL 17 + pgvector en `brain-infrastructure`:**
   - Migración idempotente `20260916000001_add_specialized_memory_metadata_and_expiration.sql`.
   - Columnas `expires_at TIMESTAMPTZ` e índices `idx_memories_expires_at`, `idx_memories_metadata_gin`.
   - Mapeo bidireccional serde JSON en `row_to_memory` y `save`/`update`.
5. **Superficie de Exposición en `brain-mcp` y `brain-cli`:**
   - Nueva herramienta MCP `brain_session_end` y argumentos especializados en `brain_remember`.
   - Subcomandos CLI `session-end` y `purge-expired`, más flags especializados en `remember` y `recall`.

### Consecuencias Positivas

* Pleno cumplimiento de los requisitos F4-01 a F4-05 y del estándar SRS §10, §68.
* Dominio cognitivo enriquecido con validaciones deterministas impenetrables.
* Ciclo de vida de sesiones y memoria de trabajo efímera con soporte TTL real.
* Trazabilidad rigurosa de evidencias en memoria semántica.
* Búsqueda asociativa y procedural estructurada disponible tanto para agentes MCP como usuarios de CLI.

### Consecuencias Negativas / Mitigaciones

* Mayor número de campos opcionales en el protocolo MCP: mitigado mediante valores por defecto coherentes y validación clara de esquemas JSON.

## Enlaces y Referencias

* [README — Tipos de Memoria Cognitiva](file:///home/guty_3rrez/Proyectos/local-brain/README.md)
* [Skill Reference — memory-types.md](file:///home/guty_3rrez/Proyectos/local-brain/.agents/skills/local-brain/references/memory-types.md)
* [ADR-0003 — Persistencia PostgreSQL](file:///home/guty_3rrez/Proyectos/local-brain/docs/adr/0003-persistencia-postgresql-sqlx.md)
