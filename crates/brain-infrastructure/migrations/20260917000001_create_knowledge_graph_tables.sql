-- 20260917000001_create_knowledge_graph_tables.sql
-- Migración idempotente para el Grafo de Conocimiento y Relaciones Tipadas (SRS §11, §25, Fase 5)

-- Tabla de Nodos del Grafo (SRS §11)
CREATE TABLE IF NOT EXISTS graph_nodes (
    id UUID PRIMARY KEY,
    node_type VARCHAR(32) NOT NULL DEFAULT 'concept',
    label VARCHAR(255) NOT NULL,
    memory_id UUID REFERENCES memories(id) ON DELETE CASCADE,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Si un nodo es de tipo 'memory', su memory_id debe ser único (un solo nodo por memoria)
CREATE UNIQUE INDEX IF NOT EXISTS idx_graph_nodes_memory_id 
ON graph_nodes(memory_id) 
WHERE memory_id IS NOT NULL;

-- Para conceptos o entidades, el label dentro del tipo es único (insensible a mayúsculas)
CREATE UNIQUE INDEX IF NOT EXISTS idx_graph_nodes_type_label 
ON graph_nodes(node_type, lower(label)) 
WHERE memory_id IS NULL;

-- Índice general por label
CREATE INDEX IF NOT EXISTS idx_graph_nodes_label ON graph_nodes(label);

-- Tabla de Aristas / Relaciones Tipadas del Grafo (SRS §11)
CREATE TABLE IF NOT EXISTS graph_edges (
    id UUID PRIMARY KEY,
    source_id UUID NOT NULL REFERENCES graph_nodes(id) ON DELETE CASCADE,
    target_id UUID NOT NULL REFERENCES graph_nodes(id) ON DELETE CASCADE,
    relation_type VARCHAR(64) NOT NULL,
    weight REAL NOT NULL DEFAULT 1.0 CHECK (weight >= 0.0 AND weight <= 1.0),
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- No permitir aristas idénticas duplicadas (mismo source, target y relación)
    CONSTRAINT uq_graph_edge UNIQUE (source_id, target_id, relation_type)
);

-- Índices B-Tree optimizados para consultas de adyacencia y traversal recursivo
CREATE INDEX IF NOT EXISTS idx_graph_edges_source ON graph_edges(source_id);
CREATE INDEX IF NOT EXISTS idx_graph_edges_target ON graph_edges(target_id);
CREATE INDEX IF NOT EXISTS idx_graph_edges_relation ON graph_edges(relation_type);
CREATE INDEX IF NOT EXISTS idx_graph_edges_source_relation ON graph_edges(source_id, relation_type);
CREATE INDEX IF NOT EXISTS idx_graph_edges_target_relation ON graph_edges(target_id, relation_type);
