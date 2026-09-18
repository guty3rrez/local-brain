//! Adaptador de persistencia relacional y traversal para el Grafo de Conocimiento en PostgreSQL (SRS §11, §25, F5-02).

use std::collections::HashMap;
use std::str::FromStr;

use async_trait::async_trait;
use sqlx::{PgPool, Row};

use brain_domain::model::MemoryId;
use brain_graph::errors::GraphError;
use brain_graph::invariants::GraphInvariants;
use brain_graph::model::{
    EdgeId, GraphEdge, GraphNode, GraphPathStep, GraphSubgraph, GraphTraversalOptions, NodeId,
    NodeType, RelationType, TraversalDirection,
};
use brain_graph::ports::GraphRepository;

/// Adaptador secundario de grafo de conocimiento sobre PostgreSQL mediante SQLx (SRS §11, §25).
#[derive(Debug, Clone)]
pub struct PostgresGraphRepository {
    pool: PgPool,
}

impl PostgresGraphRepository {
    /// Inicializa el repositorio con un pool de conexiones existente.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Retorna una referencia al pool de conexiones.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Ejecuta las migraciones de base de datos de forma programática e idempotente.
    pub async fn run_migrations(&self) -> Result<(), GraphError> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| {
                GraphError::StorageError(format!("Error al ejecutar migraciones SQLx: {e}"))
            })?;
        Ok(())
    }
}

#[async_trait]
impl GraphRepository for PostgresGraphRepository {
    async fn save_node(&self, node: &GraphNode) -> Result<(), GraphError> {
        let mem_uuid = node.memory_id.as_ref().map(|m| *m.as_uuid());

        sqlx::query(
            r#"
            INSERT INTO graph_nodes (
                id, node_type, label, memory_id, metadata, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (id) DO UPDATE SET
                label = EXCLUDED.label,
                metadata = EXCLUDED.metadata,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(node.id.as_uuid())
        .bind(node.node_type.as_str())
        .bind(&node.label)
        .bind(mem_uuid)
        .bind(&node.metadata)
        .bind(node.created_at)
        .bind(node.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| GraphError::StorageError(format!("Error al guardar nodo {}: {e}", node.id)))?;

        Ok(())
    }

    async fn find_node_by_id(&self, id: &NodeId) -> Result<Option<GraphNode>, GraphError> {
        let maybe_row = sqlx::query(
            r#"
            SELECT id, node_type, label, memory_id, metadata, created_at, updated_at
            FROM graph_nodes
            WHERE id = $1
            "#,
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| GraphError::StorageError(format!("Error al buscar nodo {id}: {e}")))?;

        match maybe_row {
            Some(row) => Ok(Some(row_to_graph_node(row)?)),
            None => Ok(None),
        }
    }

    async fn find_node_by_memory_id(
        &self,
        memory_id: &MemoryId,
    ) -> Result<Option<GraphNode>, GraphError> {
        let maybe_row = sqlx::query(
            r#"
            SELECT id, node_type, label, memory_id, metadata, created_at, updated_at
            FROM graph_nodes
            WHERE memory_id = $1
            "#,
        )
        .bind(memory_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            GraphError::StorageError(format!(
                "Error al buscar nodo por memory_id {memory_id}: {e}"
            ))
        })?;

        match maybe_row {
            Some(row) => Ok(Some(row_to_graph_node(row)?)),
            None => Ok(None),
        }
    }

    async fn find_node_by_label(
        &self,
        label: &str,
        node_type: NodeType,
    ) -> Result<Option<GraphNode>, GraphError> {
        let maybe_row = sqlx::query(
            r#"
            SELECT id, node_type, label, memory_id, metadata, created_at, updated_at
            FROM graph_nodes
            WHERE node_type = $1 AND lower(label) = lower($2)
            LIMIT 1
            "#,
        )
        .bind(node_type.as_str())
        .bind(label.trim())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            GraphError::StorageError(format!("Error al buscar nodo por label '{label}': {e}"))
        })?;

        match maybe_row {
            Some(row) => Ok(Some(row_to_graph_node(row)?)),
            None => Ok(None),
        }
    }

    async fn ensure_concept_node(&self, label: &str) -> Result<GraphNode, GraphError> {
        let clean_label = label.trim();
        if clean_label.is_empty() {
            return Err(GraphError::EmptyLabel);
        }

        if let Some(existing) = self
            .find_node_by_label(clean_label, NodeType::Concept)
            .await?
        {
            return Ok(existing);
        }

        let new_node = GraphNode::new_concept(clean_label)?;
        match self.save_node(&new_node).await {
            Ok(_) => Ok(new_node),
            Err(_) => {
                // Posible carrera concurrente: intentar recuperar el creado
                if let Some(existing) = self
                    .find_node_by_label(clean_label, NodeType::Concept)
                    .await?
                {
                    Ok(existing)
                } else {
                    Err(GraphError::StorageError(format!(
                        "No se pudo asegurar el nodo de concepto '{clean_label}'"
                    )))
                }
            }
        }
    }

    async fn ensure_memory_node(
        &self,
        memory_id: &MemoryId,
        label: &str,
    ) -> Result<GraphNode, GraphError> {
        if let Some(existing) = self.find_node_by_memory_id(memory_id).await? {
            return Ok(existing);
        }

        let new_node = GraphNode::new_memory(*memory_id, label)?;
        match self.save_node(&new_node).await {
            Ok(_) => Ok(new_node),
            Err(_) => {
                if let Some(existing) = self.find_node_by_memory_id(memory_id).await? {
                    Ok(existing)
                } else {
                    Err(GraphError::StorageError(format!(
                        "No se pudo asegurar el nodo de memoria '{memory_id}'"
                    )))
                }
            }
        }
    }

    async fn save_edge(&self, edge: &GraphEdge) -> Result<(), GraphError> {
        GraphInvariants::validate_edge(edge)?;

        // Si la relación es acíclica (SUPERSEDES, DEPENDS_ON, etc.), verificar que no cree ciclos
        if edge.relation_type.is_acyclic() {
            let cycle = self
                .has_path(&edge.target_id, &edge.source_id, Some(edge.relation_type))
                .await?;
            if cycle {
                return Err(GraphError::CycleDetected {
                    from: edge.source_id.to_string(),
                    to: edge.target_id.to_string(),
                    relation: edge.relation_type.to_string(),
                });
            }
        }

        sqlx::query(
            r#"
            INSERT INTO graph_edges (
                id, source_id, target_id, relation_type, weight, metadata, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (source_id, target_id, relation_type) DO UPDATE SET
                weight = EXCLUDED.weight,
                metadata = EXCLUDED.metadata,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(edge.id.as_uuid())
        .bind(edge.source_id.as_uuid())
        .bind(edge.target_id.as_uuid())
        .bind(edge.relation_type.as_str())
        .bind(edge.weight)
        .bind(&edge.metadata)
        .bind(edge.created_at)
        .bind(edge.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            GraphError::StorageError(format!(
                "Error al guardar arista {} -> {}: {e}",
                edge.source_id, edge.target_id
            ))
        })?;

        Ok(())
    }

    async fn find_edge_by_id(&self, id: &EdgeId) -> Result<Option<GraphEdge>, GraphError> {
        let maybe_row = sqlx::query(
            r#"
            SELECT id, source_id, target_id, relation_type, weight, metadata, created_at, updated_at
            FROM graph_edges
            WHERE id = $1
            "#,
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| GraphError::StorageError(format!("Error al buscar arista {id}: {e}")))?;

        match maybe_row {
            Some(row) => Ok(Some(row_to_graph_edge(row)?)),
            None => Ok(None),
        }
    }

    async fn find_edge(
        &self,
        source_id: &NodeId,
        target_id: &NodeId,
        relation: RelationType,
    ) -> Result<Option<GraphEdge>, GraphError> {
        let maybe_row = sqlx::query(
            r#"
            SELECT id, source_id, target_id, relation_type, weight, metadata, created_at, updated_at
            FROM graph_edges
            WHERE source_id = $1 AND target_id = $2 AND relation_type = $3
            "#,
        )
        .bind(source_id.as_uuid())
        .bind(target_id.as_uuid())
        .bind(relation.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            GraphError::StorageError(format!(
                "Error al buscar arista {source_id} -> {target_id} ({relation}): {e}"
            ))
        })?;

        match maybe_row {
            Some(row) => Ok(Some(row_to_graph_edge(row)?)),
            None => Ok(None),
        }
    }

    async fn find_edges_for_node(
        &self,
        node_id: &NodeId,
        direction: TraversalDirection,
        relation: Option<RelationType>,
    ) -> Result<Vec<GraphEdge>, GraphError> {
        let rel_str = relation.map(|r| r.as_str().to_string());

        let rows = sqlx::query(
            r#"
            SELECT id, source_id, target_id, relation_type, weight, metadata, created_at, updated_at
            FROM graph_edges
            WHERE (
                ($1 = 'outbound' AND source_id = $2) OR
                ($1 = 'inbound' AND target_id = $2) OR
                ($1 = 'both' AND (source_id = $2 OR target_id = $2))
            )
            AND ($3::VARCHAR IS NULL OR relation_type = $3)
            ORDER BY weight DESC, created_at DESC
            "#,
        )
        .bind(direction.as_str())
        .bind(node_id.as_uuid())
        .bind(rel_str)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            GraphError::StorageError(format!("Error al buscar aristas para nodo {node_id}: {e}"))
        })?;

        let mut edges = Vec::with_capacity(rows.len());
        for row in rows {
            edges.push(row_to_graph_edge(row)?);
        }
        Ok(edges)
    }

    async fn delete_edge(&self, id: &EdgeId) -> Result<(), GraphError> {
        sqlx::query("DELETE FROM graph_edges WHERE id = $1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await
            .map_err(|e| GraphError::StorageError(format!("Error al eliminar arista {id}: {e}")))?;
        Ok(())
    }

    async fn has_path(
        &self,
        from: &NodeId,
        to: &NodeId,
        relation: Option<RelationType>,
    ) -> Result<bool, GraphError> {
        if from == to {
            return Ok(true);
        }

        let rel_str = relation.map(|r| r.as_str().to_string());

        let row = sqlx::query(
            r#"
            WITH RECURSIVE path_search AS (
                SELECT source_id, target_id, ARRAY[source_id] AS visited
                FROM graph_edges
                WHERE source_id = $1 AND ($3::VARCHAR IS NULL OR relation_type = $3)

                UNION ALL

                SELECT e.source_id, e.target_id, ps.visited || e.source_id
                FROM path_search ps
                JOIN graph_edges e ON e.source_id = ps.target_id
                WHERE NOT (e.source_id = ANY(ps.visited))
                  AND ($3::VARCHAR IS NULL OR e.relation_type = $3)
            )
            SELECT EXISTS (
                SELECT 1 FROM path_search WHERE target_id = $2
            ) AS has_path
            "#,
        )
        .bind(from.as_uuid())
        .bind(to.as_uuid())
        .bind(rel_str)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            GraphError::StorageError(format!("Error comprobando camino {from} -> {to}: {e}"))
        })?;

        let exists: bool = row.try_get("has_path").unwrap_or(false);
        Ok(exists)
    }

    async fn traverse(
        &self,
        root_id: &NodeId,
        options: &GraphTraversalOptions,
    ) -> Result<GraphSubgraph, GraphError> {
        let root_node = self
            .find_node_by_id(root_id)
            .await?
            .ok_or_else(|| GraphError::NodeNotFound(root_id.to_string()))?;

        let rel_filter: Option<Vec<String>> = options
            .relation_types
            .as_ref()
            .map(|list| list.iter().map(|r| r.as_str().to_string()).collect());

        let rows = sqlx::query(
            r#"
            WITH RECURSIVE graph_traversal AS (
                SELECT 
                    n.id AS node_id,
                    n.node_type,
                    n.label,
                    n.memory_id,
                    n.metadata AS node_metadata,
                    n.created_at AS node_created_at,
                    n.updated_at AS node_updated_at,
                    NULL::UUID AS edge_id,
                    NULL::UUID AS edge_source_id,
                    NULL::UUID AS edge_target_id,
                    NULL::VARCHAR AS relation_type,
                    NULL::REAL AS edge_weight,
                    NULL::JSONB AS edge_metadata,
                    NULL::TIMESTAMPTZ AS edge_created_at,
                    NULL::TIMESTAMPTZ AS edge_updated_at,
                    NULL::UUID AS predecessor_id,
                    0 AS depth,
                    ARRAY[n.id] AS path,
                    1.0::REAL AS accumulated_weight
                FROM graph_nodes n
                WHERE n.id = $1

                UNION ALL

                SELECT 
                    next_n.id AS node_id,
                    next_n.node_type,
                    next_n.label,
                    next_n.memory_id,
                    next_n.metadata AS node_metadata,
                    next_n.created_at AS node_created_at,
                    next_n.updated_at AS node_updated_at,
                    e.id AS edge_id,
                    e.source_id AS edge_source_id,
                    e.target_id AS edge_target_id,
                    e.relation_type,
                    e.weight AS edge_weight,
                    e.metadata AS edge_metadata,
                    e.created_at AS edge_created_at,
                    e.updated_at AS edge_updated_at,
                    gt.node_id AS predecessor_id,
                    gt.depth + 1 AS depth,
                    gt.path || next_n.id AS path,
                    (gt.accumulated_weight * e.weight)::REAL AS accumulated_weight
                FROM graph_traversal gt
                JOIN graph_edges e ON (
                    ($2 = 'outbound' AND e.source_id = gt.node_id) OR
                    ($2 = 'inbound' AND e.target_id = gt.node_id) OR
                    ($2 = 'both' AND (e.source_id = gt.node_id OR e.target_id = gt.node_id))
                )
                JOIN graph_nodes next_n ON (
                    CASE 
                        WHEN e.source_id = gt.node_id THEN e.target_id 
                        ELSE e.source_id 
                    END = next_n.id
                )
                WHERE gt.depth < $3
                  AND NOT (next_n.id = ANY(gt.path))
                  AND ($4::VARCHAR[] IS NULL OR e.relation_type = ANY($4))
                  AND ($5::REAL IS NULL OR e.weight >= $5)
            )
            SELECT * FROM graph_traversal
            ORDER BY depth ASC, accumulated_weight DESC
            LIMIT $6
            "#,
        )
        .bind(root_id.as_uuid())
        .bind(options.direction.as_str())
        .bind(options.max_depth as i32)
        .bind(rel_filter)
        .bind(options.min_weight)
        .bind(options.limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            GraphError::StorageError(format!("Error en consulta recursiva CTE de traversal: {e}"))
        })?;

        let mut discovered_nodes: HashMap<NodeId, GraphNode> = HashMap::new();
        let mut discovered_edges: HashMap<EdgeId, GraphEdge> = HashMap::new();
        let mut steps: Vec<GraphPathStep> = Vec::with_capacity(rows.len());

        discovered_nodes.insert(root_node.id, root_node.clone());

        for row in rows {
            let n_id: uuid::Uuid = row.try_get("node_id").map_err(sqlx_err)?;
            let n_type_str: String = row.try_get("node_type").map_err(sqlx_err)?;
            let n_label: String = row.try_get("label").map_err(sqlx_err)?;
            let n_mem_id: Option<uuid::Uuid> = row.try_get("memory_id").map_err(sqlx_err)?;
            let n_meta: serde_json::Value = row.try_get("node_metadata").map_err(sqlx_err)?;
            let n_created: chrono::DateTime<chrono::Utc> =
                row.try_get("node_created_at").map_err(sqlx_err)?;
            let n_updated: chrono::DateTime<chrono::Utc> =
                row.try_get("node_updated_at").map_err(sqlx_err)?;

            let node_id = NodeId::from_uuid(n_id);
            let node_type = NodeType::from_str(&n_type_str)?;
            let memory_id = n_mem_id.map(MemoryId::from_uuid);

            let node = GraphNode {
                id: node_id,
                node_type,
                label: n_label.clone(),
                memory_id,
                metadata: n_meta,
                created_at: n_created,
                updated_at: n_updated,
            };

            discovered_nodes.insert(node_id, node);

            let edge_id_raw: Option<uuid::Uuid> = row.try_get("edge_id").map_err(sqlx_err)?;
            let edge_id = edge_id_raw.map(EdgeId::from_uuid);

            let rel_type = if let Some(e_id) = edge_id {
                let e_src: uuid::Uuid = row.try_get("edge_source_id").map_err(sqlx_err)?;
                let e_tgt: uuid::Uuid = row.try_get("edge_target_id").map_err(sqlx_err)?;
                let e_rel_str: String = row.try_get("relation_type").map_err(sqlx_err)?;
                let e_w: f32 = row.try_get("edge_weight").map_err(sqlx_err)?;
                let e_meta: serde_json::Value = row.try_get("edge_metadata").map_err(sqlx_err)?;
                let e_created: chrono::DateTime<chrono::Utc> =
                    row.try_get("edge_created_at").map_err(sqlx_err)?;
                let e_updated: chrono::DateTime<chrono::Utc> =
                    row.try_get("edge_updated_at").map_err(sqlx_err)?;

                let relation_type = RelationType::from_str(&e_rel_str)?;

                let edge = GraphEdge {
                    id: e_id,
                    source_id: NodeId::from_uuid(e_src),
                    target_id: NodeId::from_uuid(e_tgt),
                    relation_type,
                    weight: e_w,
                    metadata: e_meta,
                    created_at: e_created,
                    updated_at: e_updated,
                };
                discovered_edges.insert(e_id, edge);
                Some(relation_type)
            } else {
                None
            };

            let pred_id_raw: Option<uuid::Uuid> =
                row.try_get("predecessor_id").map_err(sqlx_err)?;
            let depth: i32 = row.try_get("depth").map_err(sqlx_err)?;
            let acc_w: f32 = row.try_get("accumulated_weight").map_err(sqlx_err)?;

            steps.push(GraphPathStep {
                node_id,
                label: n_label,
                node_type,
                edge_id,
                relation_type: rel_type,
                predecessor_id: pred_id_raw.map(NodeId::from_uuid),
                depth: depth as u32,
                accumulated_weight: acc_w,
            });
        }

        Ok(GraphSubgraph {
            root: root_node,
            nodes: discovered_nodes.into_values().collect(),
            edges: discovered_edges.into_values().collect(),
            steps,
        })
    }
}

fn row_to_graph_node(row: sqlx::postgres::PgRow) -> Result<GraphNode, GraphError> {
    let id: uuid::Uuid = row.try_get("id").map_err(sqlx_err)?;
    let type_str: String = row.try_get("node_type").map_err(sqlx_err)?;
    let label: String = row.try_get("label").map_err(sqlx_err)?;
    let mem_id_raw: Option<uuid::Uuid> = row.try_get("memory_id").map_err(sqlx_err)?;
    let metadata: serde_json::Value = row.try_get("metadata").map_err(sqlx_err)?;
    let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at").map_err(sqlx_err)?;
    let updated_at: chrono::DateTime<chrono::Utc> = row.try_get("updated_at").map_err(sqlx_err)?;

    Ok(GraphNode {
        id: NodeId::from_uuid(id),
        node_type: NodeType::from_str(&type_str)?,
        label,
        memory_id: mem_id_raw.map(MemoryId::from_uuid),
        metadata,
        created_at,
        updated_at,
    })
}

fn row_to_graph_edge(row: sqlx::postgres::PgRow) -> Result<GraphEdge, GraphError> {
    let id: uuid::Uuid = row.try_get("id").map_err(sqlx_err)?;
    let src: uuid::Uuid = row.try_get("source_id").map_err(sqlx_err)?;
    let tgt: uuid::Uuid = row.try_get("target_id").map_err(sqlx_err)?;
    let rel_str: String = row.try_get("relation_type").map_err(sqlx_err)?;
    let weight: f32 = row.try_get("weight").map_err(sqlx_err)?;
    let metadata: serde_json::Value = row.try_get("metadata").map_err(sqlx_err)?;
    let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at").map_err(sqlx_err)?;
    let updated_at: chrono::DateTime<chrono::Utc> = row.try_get("updated_at").map_err(sqlx_err)?;

    Ok(GraphEdge {
        id: EdgeId::from_uuid(id),
        source_id: NodeId::from_uuid(src),
        target_id: NodeId::from_uuid(tgt),
        relation_type: RelationType::from_str(&rel_str)?,
        weight,
        metadata,
        created_at,
        updated_at,
    })
}

fn sqlx_err(e: sqlx::Error) -> GraphError {
    GraphError::StorageError(e.to_string())
}
