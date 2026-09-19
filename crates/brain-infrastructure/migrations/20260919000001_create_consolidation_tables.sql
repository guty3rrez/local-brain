-- 20260919000001_create_consolidation_tables.sql
-- Migración idempotente para Motor de Consolidación, Reflexión y Contradicciones (SRS §16, §17, §18, Fase 7)

-- Tabla de Contradicciones y Conflictos Cognitivos (SRS §18)
CREATE TABLE IF NOT EXISTS knowledge_conflicts (
    id UUID PRIMARY KEY,
    source_memory_id UUID NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    conflicting_memory_id UUID NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    conflict_type VARCHAR(64) NOT NULL,
    reason TEXT NOT NULL,
    suggested_resolution TEXT,
    status VARCHAR(32) NOT NULL DEFAULT 'pending',
    resolution_context TEXT,
    detected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb
);

-- Tabla de Auditoría de Sesiones de Reflexión y Consolidación (SRS §16, §17)
CREATE TABLE IF NOT EXISTS consolidation_runs (
    id UUID PRIMARY KEY,
    project VARCHAR(255),
    memories_analyzed INTEGER NOT NULL,
    clusters_count INTEGER NOT NULL,
    hypotheses_count INTEGER NOT NULL,
    conflicts_count INTEGER NOT NULL,
    summary TEXT,
    executed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Índices B-Tree optimizados para consultas de resolución y auditoría
CREATE INDEX IF NOT EXISTS idx_conflicts_status ON knowledge_conflicts(status);
CREATE INDEX IF NOT EXISTS idx_conflicts_source ON knowledge_conflicts(source_memory_id);
CREATE INDEX IF NOT EXISTS idx_conflicts_target ON knowledge_conflicts(conflicting_memory_id);
CREATE INDEX IF NOT EXISTS idx_conflicts_detected_at ON knowledge_conflicts(detected_at DESC);
CREATE INDEX IF NOT EXISTS idx_consolidation_runs_project ON consolidation_runs(project);
CREATE INDEX IF NOT EXISTS idx_consolidation_runs_executed_at ON consolidation_runs(executed_at DESC);
