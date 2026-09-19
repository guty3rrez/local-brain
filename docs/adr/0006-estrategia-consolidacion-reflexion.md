# ADR-0006: Estrategia de Consolidación Cognitiva, Reflexión Asíncrona y Manejo de Contradicciones

* **Estado:** Aceptado
* **Decisores:** Local Brain Core Team & Contributors
* **Fecha:** 2026-09-19

## Contexto y Declaración del Problema

En concordancia con la Regla de Oro (SRS §80) y los requisitos del motor de consolidación y reflexión (SRS §16, §17, §18, §68):
1. **Acumulación de Experiencias Crudas (§16):** Los agentes generan múltiples memorias episódicas y de trabajo que, sin un mecanismo de destilación, saturan el espacio contextual y degradan la precisión del retrieval semántico.
2. **Abstracción e Hipótesis Candidatas (§17):** El sistema debe agrupar experiencias similares (clustering semántico y temporal) para descubrir regularidades empíricas, pero toda generalización resultante debe ser catalogada como hipótesis tentativa con confianza máxima de 0.40 (invariante de dominio) y vinculada a sus memorias fuente como evidencia.
3. **Detección Determinista de Contradicciones (§18):** Cuando dos memorias o creencias chocan (ej. colisión de preferencias metodológicas como Supabase vs .NET o inversiones lógicas de polaridad), el sistema no debe resolver la contradicción por alucinación probabilística. Debe marcar ambas memorias en estado `CONFLICT`, registrar el conflicto en `knowledge_conflicts`, enlazar una arista `CONTRADICTS` en el grafo de conocimiento y requerir resolución humana o heurística contextual explícita.
4. **Resiliencia Local-First y Desacoplamiento (SRS §4.1, RNF-006):** La lógica de clustering, detección de contradicciones y gestión de invariantes debe ser pura en dominio (`brain-consolidation`), funcionando al 100% incluso si el LLM local (`llama.cpp`) no está activo (fallback determinista).

## Opciones Consideradas

* **Opción A: Delegar la consolidación y detección de contradicciones enteramente a un prompt del LLM:**
  - *Ventajas:* Implementación superficialmente rápida sin código de clustering ni algoritmos de similitud en Rust.
  - *Desventajas:* Viola SRS §4.5 (No confiar ciegamente en el LLM) y SRS §1.3. Los LLMs son propensos a alucinaciones, inconsistencia determinista y fallos silenciosos ante contradicciones. Además, no funcionaría offline sin LLM cargado.
* **Opción B: Consolidación síncrona en cada escritura (`remember`):**
  - *Ventajas:* Las memorias estarían permanentemente consolidadas al instante.
  - *Desventajas:* Introduce latencias inaceptables en `brain remember` (violando SRS §12.3: latencia < 20ms en escrituras), generando contención masiva de bloqueos en PostgreSQL y cálculo O(N^2) de similitud por cada nuevo recuerdo.
* **Opción C: Crate de Dominio Puro (`brain-consolidation`) con Clustering Determinista, Detección de Contradicciones Heurística + LLM Híbrido, y Persistencia Relacional:**
  - *Ventajas:*
    1. **Arquitectura Hexagonal Estricta (RNF-006):** Crate `brain-consolidation` desacoplado de bases de datos, redes y dependencias pesadas.
    2. **Algoritmo de Clustering por Líderes de Similitud Coseno:** Agrupamiento O(N) determinista usando similitud vectorial (umbral por defecto 0.82) y fallback temporal/metadatos cuando no hay embeddings.
    3. **Detección Robusta de Contradicciones:** `ContradictionDetector` evalúa colisiones de preferencias mutuamente excluyentes (SRS §18) y polaridades invertidas sin requerir LLM, complementado por el puerto `ConsolidationLlmPort`.
    4. **Integración Transversal:** Conecta con `brain-learning` (creando hipótesis tentativas en etapa `candidate`), `brain-graph` (creando aristas `CONTRADICTS`), y `brain-domain` (actualizando `MemoryStatus::Conflict`).
    5. **Seguridad y Anti-Prompt Injection:** Envoltura estricta con delimitador `<untrusted_consolidation_context>` en la capa MCP (SRS §32).

## Decisión Adoptada

Se adopta la **Opción C**:

1. **Crate de Dominio `brain-consolidation`:**
   - Define los invariantes de consolidación: `TENTATIVE_MAX_CONFIDENCE = 0.40`, `MIN_CLUSTER_SIZE = 2`, `DEFAULT_SIMILARITY_THRESHOLD = 0.82`.
   - Modela `Contradiction`, `ConflictType` (`DirectOpposite`, `MutuallyExclusivePreference`, `ConditionalContradiction`), `ConflictStatus` (`Pending`, `Resolved`, `Dismissed`), `ClusterableMemory`, `MemoryCluster`, `Pattern`, `Hypothesis`, `ReflectionReport`.
   - Implementa `ContradictionDetector` determinista y `cluster_memories` por similitud coseno.
   - Define los puertos `ConflictRepository` y `ConsolidationLlmPort`, acompañados de implementaciones en memoria (`InMemoryConflictRepository`, `MockConsolidationLlm`).

2. **Capa de Infraestructura `brain-infrastructure`:**
   - Migración SQLx `20260919000001_create_consolidation_tables.sql` con tablas `knowledge_conflicts` y `consolidation_runs`.
   - `PostgresConflictRepository` con CRUD completo, resolución de conflictos y registro de auditoría.
   - `LlamaCppReflectionClient` comunicándose con `llama.cpp` (`/completion`) con fallback automático y transparente a síntesis heurística si el runtime LLM está desconectado.

3. **Capa de Aplicación `brain-application`:**
   - Casos de uso: `ReflectUseCase`, `ConsolidateUseCase`, `ResolveConflictUseCase`, `ListConflictsUseCase`.
   - Conexión bidireccional con el grafo de conocimiento (`RelationType::Contradicts`) y con el motor de aprendizaje (`save_candidate` con evidencias enlazadas).

4. **Superficie MCP y CLI:**
   - Herramienta MCP `brain_consolidate` con delimitador `<untrusted_consolidation_context>` e información de seguridad.
   - Subcomandos CLI `brain reflect` y `brain conflicts` (`list`, `resolve`).

### Consecuencias Positivas

* **Prevención de Alucinaciones:** Toda creencia originada por consolidación nace con confianza tentativa acotada (≤ 0.40) y evidencias trazables.
* **Consistencia e Integridad Referencial:** Los conflictos son auditables y no pueden borrarse arbitrariamente; requieren contexto de resolución.
* **Rendimiento Offline y Resiliencia:** El sistema reflexiona y detecta colisiones incluso en entornos restringidos sin GPU ni LLM activo.
* **Cobertura Total de Pruebas:** Verificado mediante tests unitarios puros, pruebas de integración contra PostgreSQL 17 + pgvector, y escenarios BDD completos en Gherkin.

### Consecuencias Negativas / Riesgos

* **Complejidad Heurística:** Las reglas deterministas de colisión de preferencias cubren patrones idiomáticos frecuentes, pero afirmaciones altamente ambiguas o matizadas dependen del análisis semántico profundo del LLM cuando está disponible.
* **Mitigación:** Se utiliza una arquitectura híbrida donde el detector heurístico analiza primero y el LLM complementa, respetando siempre que la decisión final de estado y resolución es determinista y auditable.

## Enlaces y Referencias

* [SRS §16: Motor de Reflexión y Consolidación](SRS%20%E2%80%94%20Local%20Brain_%20Memoria%20persistente%20local%20para%20agentes%20de%20IA.md)
* [SRS §17: Formación de Hipótesis y Abstracciones](SRS%20%E2%80%94%20Local%20Brain_%20Memoria%20persistente%20local%20para%20agentes%20de%20IA.md)
* [SRS §18: Detección y Resolución de Contradicciones](SRS%20%E2%80%94%20Local%20Brain_%20Memoria%20persistente%20local%20para%20agentes%20de%20IA.md)
* [SRS §41 & §80: Regla de Oro y Human Review Gate](SRS%20%E2%80%94%20Local%20Brain_%20Memoria%20persistente%20local%20para%20agentes%20de%20IA.md)
* [ADR-0005: Especialización de Tipos de Memoria](0005-especializacion-tipos-de-memoria.md)
