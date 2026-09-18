-- 20260916000001_add_specialized_memory_metadata_and_expiration.sql
-- Migración para soporte de TTL en memorias de trabajo e índices especializados (SRS §10, §25)

-- Añade columna para timestamp de expiración de memoria (Working Memory con TTL)
ALTER TABLE memories ADD COLUMN IF NOT EXISTS expires_at TIMESTAMPTZ;

-- Índice parcial para búsquedas rápidas y purga de memorias expiradas
CREATE INDEX IF NOT EXISTS idx_memories_expires_at ON memories(expires_at) 
WHERE expires_at IS NOT NULL;

-- Índice en expresión JSONB para recuperar rápidamente memorias activas asociadas a una sesión
CREATE INDEX IF NOT EXISTS idx_memories_session_id ON memories((provenance->>'session_id')) 
WHERE provenance->>'session_id' IS NOT NULL;
