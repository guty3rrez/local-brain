-- 20260914000001_create_memories_table.sql
-- Migración inicial idempotente para el almacenamiento de memorias cognitivas (SRS §9.1, §25.1)

-- Extensión requerida para utilidades UUID en PostgreSQL si aplica
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Tabla principal de memorias cognitivas
CREATE TABLE IF NOT EXISTS memories (
    id UUID PRIMARY KEY,
    memory_type VARCHAR(32) NOT NULL,
    content_text TEXT NOT NULL,
    content_hash VARCHAR(64) NOT NULL,
    summary TEXT,
    project VARCHAR(255),
    agent VARCHAR(255),
    importance REAL NOT NULL CHECK (importance >= 0.0 AND importance <= 1.0),
    confidence REAL NOT NULL CHECK (confidence >= 0.0 AND confidence <= 1.0),
    utility REAL NOT NULL CHECK (utility >= 0.0 AND utility <= 1.0),
    status VARCHAR(32) NOT NULL DEFAULT 'active',
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    provenance JSONB NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_retrieved_at TIMESTAMPTZ
);

-- Índices de consulta y rendimiento (SRS §25)
CREATE INDEX IF NOT EXISTS idx_memories_project ON memories(project) WHERE status != 'soft_deleted';
CREATE INDEX IF NOT EXISTS idx_memories_type ON memories(memory_type) WHERE status != 'soft_deleted';
CREATE INDEX IF NOT EXISTS idx_memories_status ON memories(status);
CREATE INDEX IF NOT EXISTS idx_memories_content_hash ON memories(content_hash);
CREATE INDEX IF NOT EXISTS idx_memories_created_at ON memories(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_memories_metadata ON memories USING gin(metadata);
CREATE INDEX IF NOT EXISTS idx_memories_provenance ON memories USING gin(provenance);
