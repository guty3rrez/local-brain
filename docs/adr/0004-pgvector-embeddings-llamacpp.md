# ADR-0004: Búsqueda Semántica Vectorial con pgvector (HNSW), 768 Dimensiones y llama.cpp

* **Estado:** Aceptado
* **Decisores:** Local Brain Core Team & Contributors
* **Fecha:** 2026-09-15

## Contexto y Declaración del Problema

Local Brain requiere recuperar memorias persistentes no solo mediante filtros relacionales exactos (proyecto, agente, tipo), sino a través de **búsqueda semántica por significado en lenguaje natural** (SRS §12, §13, §25.2).  
Los requisitos clave son:
1. **Local-first y 100% offline (SRS §4.1):** Cero dependencia de APIs externas en la nube (OpenAI, Anthropic, Cohere).
2. **Alta fidelidad semántica para programación:** Capacidad de capturar matices complejos en fragmentos de código, documentación técnica y razonamiento bilingüe (español/inglés) sin truncamiento prematuro de tokens.
3. **Resiliencia y desacople (SRS §12.3, RNF-006):** Si el servidor de embeddings local no está disponible, la memoria debe persistir sin bloquear ni abortar la operación (`PendingEmbedding`). El dominio puro no debe acoplarse a frameworks de inferencia ni base de datos.
4. **Búsqueda sub-milisegundo:** Latencia p95 < 10 ms para la consulta vectorial `LIMIT K` sobre colecciones de decenas de miles de recuerdos en hardware de consumo (Ryzen 7 + 16 GB RAM).

## Opciones Consideradas

* **Opción A: 384 dimensiones (`all-MiniLM-L6-v2`, `bge-small`) + IVFFlat:**
  - *Ventajas:* Menor uso de VRAM (< 150 MB) y menor tamaño vectorial en disco (1.5 KB/fila).
  - *Desventajas:* Ventana de contexto muy reducida (máx 512 tokens), menor precisión semántica en vocabulario de programación y requiere reentrenar listas de centroides en IVFFlat.
* **Opción B: 768 dimensiones (`nomic-embed-text-v1.5`, `bge-base`) + HNSW (pgvector):**
  - *Ventajas:* Ventana de contexto amplia (hasta 8.192 tokens en Nomic), máxima fidelidad semántica en código/jerga técnica (+7% a +10% en MTEB Retrieval) e índice HNSW que no requiere entrenamiento previo y responde en sub-milisegundos.
  - *Desventajas:* Duplica el almacenamiento de vectores a 3.0 KB/fila (despreciable en hardware de consumo moderno: ~30 MB por cada 10.000 recuerdos).
* **Opción C: Base de datos vectorial externa dedicada (Qdrant, Milvus, Chroma):**
  - *Desventajas:* Introduce otro proceso/contenedor obligatorio, complica transacciones ACID y duplica el mantenimiento respecto a tener PostgreSQL y pgvector integrados.

## Decisión Adoptada

Opción elegida: **Opción B: 768 dimensiones + pgvector con índice HNSW (`vector_cosine_ops`) + cliente HTTP para runtime local llama.cpp con fallback asíncrono (`PendingEmbedding`)**.

1. **Estándar de 768 dimensiones:** Adoptado para garantizar alta fidelidad en bloques de código y textos extensos sin truncamiento.
2. **Índice HNSW en PostgreSQL 17:** Se crea `idx_memories_embedding_hnsw` con métrica coseno (`<=>`) y predicado parcial `WHERE status != 'soft_deleted' AND embedding IS NOT NULL`.
3. **Desacople Hexagonal:**
   - Trait `EmbeddingProvider` y `VectorRepository` en `brain-domain::ports`.
   - Implementaciones deterministas en memoria (`InMemoryEmbeddingProvider`, `InMemoryVectorRepository`) para pruebas unitarias instantáneas sin red ni base de datos.
   - Adaptador HTTP `LlamaCppEmbeddingProvider` en `brain-infrastructure`.
4. **Resiliencia Operativa (SRS §12.3):** Si el servidor `llama.cpp` no responde, `RememberUseCase` persiste la memoria con `status = MemoryStatus::PendingEmbedding` y `EmbedPendingUseCase` permite procesarlas en segundo plano.

### Consecuencias Positivas

* Búsqueda semántica conceptual de alta fidelidad disponible en la CLI (`brain recall -q "consulta"`) y lista para ser consumida por herramientas MCP (Fase 3).
* Consultas vectoriales HNSW sobre PostgreSQL extremadamente rápidas (< 2 ms) con soporte ACID integral.
* Cero dependencias de proveedores comerciales en la nube.
* Suite de pruebas unitarias de dominio puro 100% aislada de red y GPU.

### Consecuencias Negativas / Riesgos

* Si el usuario no tiene iniciado el servicio de embeddings (`llama.cpp`), los recuerdos se guardan en estado `pending_embedding` hasta que se ejecute `brain embed-pending`.

## Enlaces y Referencias

* [README — Motor de Embeddings y Búsqueda Semántica](file:///home/guty_3rrez/Proyectos/local-brain/README.md)
* [ADR-0003 — Persistencia PostgreSQL](file:///home/guty_3rrez/Proyectos/local-brain/docs/adr/0003-persistencia-postgresql-sqlx.md)
