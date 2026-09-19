-- 20260918000001_create_learning_tables.sql
-- Migración idempotente para el Motor de Aprendizaje y Conocimiento Candidato (SRS §15, §59, §60, Fase 6)

-- Tabla de Conocimientos Candidatos y Observaciones (SRS §15, §59)
CREATE TABLE IF NOT EXISTS learning_candidates (
    id UUID PRIMARY KEY,
    statement TEXT NOT NULL,
    stage VARCHAR(32) NOT NULL DEFAULT 'observation',
    confidence REAL NOT NULL CHECK (confidence >= 0.0 AND confidence <= 1.0),
    domain VARCHAR(255),
    human_validated BOOLEAN NOT NULL DEFAULT FALSE,
    memory_id UUID REFERENCES memories(id) ON DELETE SET NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Tabla de Evidencias Empíricas de Aprendizaje (SRS §15, §60)
CREATE TABLE IF NOT EXISTS learning_evidence (
    id UUID PRIMARY KEY,
    candidate_id UUID NOT NULL REFERENCES learning_candidates(id) ON DELETE CASCADE,
    source_type VARCHAR(32) NOT NULL,
    content TEXT NOT NULL,
    memory_id UUID REFERENCES memories(id) ON DELETE SET NULL,
    agent VARCHAR(255),
    confidence_weight REAL NOT NULL DEFAULT 0.5 CHECK (confidence_weight >= 0.0 AND confidence_weight <= 1.0),
    is_supporting BOOLEAN NOT NULL DEFAULT TRUE,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Índices B-Tree optimizados para consultas de aprendizaje
CREATE INDEX IF NOT EXISTS idx_learning_candidates_domain ON learning_candidates(domain);
CREATE INDEX IF NOT EXISTS idx_learning_candidates_stage ON learning_candidates(stage);
CREATE INDEX IF NOT EXISTS idx_learning_candidates_confidence ON learning_candidates(confidence DESC);
CREATE INDEX IF NOT EXISTS idx_learning_candidates_updated_at ON learning_candidates(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_learning_evidence_candidate_id ON learning_evidence(candidate_id);
CREATE INDEX IF NOT EXISTS idx_learning_evidence_memory_id ON learning_evidence(memory_id) WHERE memory_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_learning_evidence_source_type ON learning_evidence(source_type);
