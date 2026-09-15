-- 20260915000001_enable_pgvector_and_embedding.sql
-- Migración para habilitar la extensión pgvector y almacenamiento vectorial de 768 dimensiones (SRS §12, §25.2)

-- Extensión requerida para tipos de datos y operadores vectoriales
CREATE EXTENSION IF NOT EXISTS vector;

-- Columna vectorial de 768 dimensiones para embeddings densos de alta fidelidad
ALTER TABLE memories ADD COLUMN IF NOT EXISTS embedding vector(768);

-- Índice HNSW con distancia coseno (<=>) para recuperación semántica sub-milisegundo (p95 < 10ms)
CREATE INDEX IF NOT EXISTS idx_memories_embedding_hnsw
ON memories USING hnsw (embedding vector_cosine_ops)
WHERE status != 'soft_deleted' AND embedding IS NOT NULL;
