-- 20260920000001_add_full_text_search.sql
-- Migración para habilitar Full-Text Search (FTS) en memorias cognitivas (SRS §13.1, §25)

-- Columna generada y almacenada para representación léxica tsvector
ALTER TABLE memories ADD COLUMN IF NOT EXISTS tsv tsvector
    GENERATED ALWAYS AS (
        to_tsvector('english', coalesce(content_text, '') || ' ' || coalesce(summary, '') || ' ' || coalesce(project, ''))
    ) STORED;

-- Índice invertido generalizado (GIN) para acelerar consultas FTS con plainto_tsquery
CREATE INDEX IF NOT EXISTS idx_memories_tsv ON memories USING gin(tsv)
    WHERE status != 'soft_deleted';
