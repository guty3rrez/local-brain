//! Definición, esquemas JSON y despachadores de herramientas MCP (SRS §21, §31, §32).

use std::str::FromStr;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde_json::json;
use tracing::{error, info, warn};

use brain_application::{
    ApplicationError, ForgetUseCase, RecallQuery, RecallUseCase, RememberCommand, RememberUseCase,
};
use brain_domain::model::{MemoryId, MemoryType};

use crate::protocol::{ToolCallResult, ToolDefinition};
use crate::security::{
    format_retrieved_memories, McpPermission, McpSecurityError, McpSecurityPolicy,
};

/// Retorna la lista de definiciones de herramientas estándar expuestas por Local Brain (SRS §21).
pub fn list_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "brain_remember".to_string(),
            description: "Registra un nuevo recuerdo o experiencia en Local Brain con persistencia local, vector de embeddings de 768 dimensiones y metadatos de procedencia (SRS §21.2).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "Contenido textual del recuerdo a almacenar (máximo 64 KB)."
                    },
                    "memory_type": {
                        "type": "string",
                        "enum": ["episodic", "semantic", "procedural", "associative", "working"],
                        "description": "Tipo cognitivo de memoria. Por defecto: 'episodic'."
                    },
                    "project": {
                        "type": "string",
                        "description": "Nombre o identificador del proyecto asociado."
                    },
                    "agent": {
                        "type": "string",
                        "description": "Identificador del agente que registra el recuerdo."
                    },
                    "importance": {
                        "type": "number",
                        "minimum": 0.0,
                        "maximum": 1.0,
                        "description": "Importancia intrínseca de 0.0 a 1.0 (por defecto: 0.5)."
                    },
                    "confidence": {
                        "type": "number",
                        "minimum": 0.0,
                        "maximum": 1.0,
                        "description": "Nivel de certidumbre empírica de 0.0 a 1.0 (por defecto: 0.3)."
                    },
                    "session_id": {
                        "type": "string",
                        "description": "Identificador de sesión activa para trazabilidad de contexto."
                    }
                },
                "required": ["content"]
            }),
        },
        ToolDefinition {
            name: "brain_recall".to_string(),
            description: "Recupera recuerdos cognitivos relevantes mediante búsqueda semántica vectorial (768 dimensiones), ID directo o filtros básicos de proyecto/tipo (SRS §21.3).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Texto para búsqueda semántica vectorial basada en similitud coseno."
                    },
                    "id": {
                        "type": "string",
                        "description": "Identificador UUID de un recuerdo específico."
                    },
                    "project": {
                        "type": "string",
                        "description": "Filtrar por nombre de proyecto."
                    },
                    "memory_type": {
                        "type": "string",
                        "enum": ["episodic", "semantic", "procedural", "associative", "working"],
                        "description": "Filtrar por tipo cognitivo de memoria."
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 50,
                        "description": "Cantidad máxima de recuerdos a retornar (por defecto: 10)."
                    }
                }
            }),
        },
        ToolDefinition {
            name: "brain_search".to_string(),
            description: "Búsqueda avanzada multicriterio en Local Brain: permite filtrar simultáneamente por texto/semántica, proyecto, tipo, rango de fechas (ISO 8601) y umbral mínimo de importancia (SRS §21.1).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Consulta de búsqueda por texto o semántica."
                    },
                    "project": {
                        "type": "string",
                        "description": "Filtrar por proyecto."
                    },
                    "memory_type": {
                        "type": "string",
                        "enum": ["episodic", "semantic", "procedural", "associative", "working"],
                        "description": "Filtrar por tipo de memoria."
                    },
                    "min_importance": {
                        "type": "number",
                        "minimum": 0.0,
                        "maximum": 1.0,
                        "description": "Umbral mínimo de importancia (0.0 a 1.0)."
                    },
                    "from_date": {
                        "type": "string",
                        "description": "Fecha inicial de creación en formato ISO 8601 (ej. '2026-09-01T00:00:00Z')."
                    },
                    "to_date": {
                        "type": "string",
                        "description": "Fecha final de creación en formato ISO 8601."
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 50,
                        "description": "Cantidad máxima de recuerdos a retornar (por defecto: 10)."
                    }
                }
            }),
        },
        ToolDefinition {
            name: "brain_forget".to_string(),
            description: "Elimina lógicamente (soft-delete) un recuerdo existente. Requiere permiso de eliminación y confirmación explícita (SRS §31, §34).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Identificador UUID del recuerdo a eliminar."
                    },
                    "confirm": {
                        "type": "boolean",
                        "description": "Debe ser explícitamente true para confirmar la operación destructiva."
                    }
                },
                "required": ["id", "confirm"]
            }),
        },
    ]
}

/// Parsea un tipo de memoria desde una cadena JSON.
fn parse_memory_type(val: &str) -> Option<MemoryType> {
    match val.to_lowercase().as_str() {
        "working" => Some(MemoryType::Working),
        "episodic" => Some(MemoryType::Episodic),
        "semantic" => Some(MemoryType::Semantic),
        "procedural" => Some(MemoryType::Procedural),
        "associative" => Some(MemoryType::Associative),
        _ => None,
    }
}

/// Ejecuta la herramienta `brain_remember`.
pub async fn execute_remember(
    args: serde_json::Value,
    remember_uc: Arc<RememberUseCase>,
    security: &McpSecurityPolicy,
) -> ToolCallResult {
    if let Err(e) = security.check_permission(McpPermission::Write) {
        return ToolCallResult::error(format!("Error de seguridad: {e}"));
    }

    let content = match args.get("content").and_then(|v| v.as_str()) {
        Some(c) if !c.trim().is_empty() => c.to_string(),
        _ => {
            return ToolCallResult::error(
                "El argumento 'content' es requerido y no puede estar vacío.",
            )
        }
    };

    let mut cmd = RememberCommand::new(content);

    if let Some(type_str) = args.get("memory_type").and_then(|v| v.as_str()) {
        match parse_memory_type(type_str) {
            Some(mtype) => cmd = cmd.with_type(mtype),
            None => {
                return ToolCallResult::error(format!(
                    "Tipo de memoria inválido '{type_str}'. Válidos: episodic, semantic, procedural, associative, working"
                ));
            }
        }
    }

    if let Some(project) = args.get("project").and_then(|v| v.as_str()) {
        cmd = cmd.with_project(project);
    }

    if let Some(agent) = args.get("agent").and_then(|v| v.as_str()) {
        cmd = cmd.with_agent(agent);
    }

    if let Some(session_id) = args.get("session_id").and_then(|v| v.as_str()) {
        cmd = cmd.with_session(session_id);
    }

    if let Some(importance) = args.get("importance").and_then(|v| v.as_f64()) {
        if (0.0..=1.0).contains(&importance) {
            cmd = cmd.with_importance(importance as f32);
        } else {
            return ToolCallResult::error(
                "El argumento 'importance' debe estar en el rango [0.0, 1.0].",
            );
        }
    }

    if let Some(confidence) = args.get("confidence").and_then(|v| v.as_f64()) {
        if (0.0..=1.0).contains(&confidence) {
            cmd = cmd.with_confidence(confidence as f32);
        } else {
            return ToolCallResult::error(
                "El argumento 'confidence' debe estar en el rango [0.0, 1.0].",
            );
        }
    }

    match remember_uc.execute(cmd).await {
        Ok(mem) => {
            info!(memory_id = %mem.id, "Recuerdo persistido exitosamente vía MCP");
            let response_text = format!(
                "Recuerdo almacenado con éxito en Local Brain.\nID: {}\nTipo: {:?}\nProyecto: {}\nEstado: {:?}\nCreado: {}",
                mem.id,
                mem.memory_type,
                mem.project.as_deref().unwrap_or("default"),
                mem.status,
                mem.created_at.to_rfc3339()
            );
            ToolCallResult::success(response_text)
        }
        Err(e) => {
            error!(error = %e, "Fallo al ejecutar RememberUseCase vía MCP");
            ToolCallResult::error(format!("Error al registrar memoria: {e}"))
        }
    }
}

/// Ejecuta la herramienta `brain_recall`.
pub async fn execute_recall(
    args: serde_json::Value,
    recall_uc: Arc<RecallUseCase>,
    security: &McpSecurityPolicy,
) -> ToolCallResult {
    if let Err(e) = security.check_permission(McpPermission::Read) {
        return ToolCallResult::error(format!("Error de seguridad: {e}"));
    }

    let mut query = RecallQuery::default();

    if let Some(id_str) = args.get("id").and_then(|v| v.as_str()) {
        match MemoryId::from_str(id_str) {
            Ok(mem_id) => query.id = Some(mem_id),
            Err(e) => {
                return ToolCallResult::error(format!("UUID de recuerdo inválido '{id_str}': {e}"))
            }
        }
    }

    if let Some(text_query) = args.get("query").and_then(|v| v.as_str()) {
        query.query = Some(text_query.to_string());
    }

    if let Some(project) = args.get("project").and_then(|v| v.as_str()) {
        query.project = Some(project.to_string());
    }

    if let Some(type_str) = args.get("memory_type").and_then(|v| v.as_str()) {
        match parse_memory_type(type_str) {
            Some(mtype) => query.memory_type = Some(mtype),
            None => {
                return ToolCallResult::error(format!("Tipo de memoria inválido '{type_str}'."));
            }
        }
    }

    if let Some(limit) = args.get("limit").and_then(|v| v.as_u64()) {
        query.limit = Some(limit as usize);
    }

    match recall_uc.execute(query).await {
        Ok(memories) => {
            let output = format_retrieved_memories(&memories);
            ToolCallResult::success(output)
        }
        Err(ApplicationError::NotFound(id)) => {
            ToolCallResult::error(format!("Recuerdo con ID '{id}' no encontrado o no activo."))
        }
        Err(e) => {
            error!(error = %e, "Fallo al ejecutar RecallUseCase vía MCP");
            ToolCallResult::error(format!("Error en recuperación de memoria: {e}"))
        }
    }
}

/// Ejecuta la herramienta `brain_search`.
pub async fn execute_search(
    args: serde_json::Value,
    recall_uc: Arc<RecallUseCase>,
    security: &McpSecurityPolicy,
) -> ToolCallResult {
    if let Err(e) = security.check_permission(McpPermission::Read) {
        return ToolCallResult::error(format!("Error de seguridad: {e}"));
    }

    let mut query = RecallQuery::default();

    if let Some(text_query) = args.get("query").and_then(|v| v.as_str()) {
        query.query = Some(text_query.to_string());
    }

    if let Some(project) = args.get("project").and_then(|v| v.as_str()) {
        query.project = Some(project.to_string());
    }

    if let Some(type_str) = args.get("memory_type").and_then(|v| v.as_str()) {
        match parse_memory_type(type_str) {
            Some(mtype) => query.memory_type = Some(mtype),
            None => {
                return ToolCallResult::error(format!("Tipo de memoria inválido '{type_str}'."));
            }
        }
    }

    if let Some(min_imp) = args.get("min_importance").and_then(|v| v.as_f64()) {
        if (0.0..=1.0).contains(&min_imp) {
            query.min_importance = Some(min_imp as f32);
        } else {
            return ToolCallResult::error("min_importance debe estar entre 0.0 y 1.0.");
        }
    }

    if let Some(from_str) = args.get("from_date").and_then(|v| v.as_str()) {
        match DateTime::parse_from_rfc3339(from_str) {
            Ok(dt) => query.from_date = Some(dt.with_timezone(&Utc)),
            Err(_) => return ToolCallResult::error(format!("Formato de fecha inválido en 'from_date': '{from_str}'. Use formato ISO 8601 (ej. 2026-09-01T00:00:00Z).")),
        }
    }

    if let Some(to_str) = args.get("to_date").and_then(|v| v.as_str()) {
        match DateTime::parse_from_rfc3339(to_str) {
            Ok(dt) => query.to_date = Some(dt.with_timezone(&Utc)),
            Err(_) => {
                return ToolCallResult::error(format!(
                    "Formato de fecha inválido en 'to_date': '{to_str}'. Use formato ISO 8601."
                ))
            }
        }
    }

    if let Some(limit) = args.get("limit").and_then(|v| v.as_u64()) {
        query.limit = Some(limit as usize);
    }

    match recall_uc.execute(query).await {
        Ok(memories) => {
            let output = format_retrieved_memories(&memories);
            ToolCallResult::success(output)
        }
        Err(e) => {
            error!(error = %e, "Fallo al ejecutar RecallUseCase para search vía MCP");
            ToolCallResult::error(format!("Error en búsqueda avanzada de memoria: {e}"))
        }
    }
}

/// Ejecuta la herramienta `brain_forget` (Operación destructiva protegida SRS §31, §34).
pub async fn execute_forget(
    args: serde_json::Value,
    forget_uc: Arc<ForgetUseCase>,
    security: &McpSecurityPolicy,
) -> ToolCallResult {
    // 1. Verificar permiso de borrado
    if let Err(e) = security.check_permission(McpPermission::Delete) {
        warn!("Intento no autorizado de ejecutar brain_forget: {}", e);
        return ToolCallResult::error(format!("Error de seguridad: {e}"));
    }

    // 2. Verificar confirmación explícita obligatoria (SRS §31)
    let confirm = args
        .get("confirm")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !confirm {
        return ToolCallResult::error(format!(
            "Error de seguridad: {}",
            McpSecurityError::ConfirmationRequired
        ));
    }

    let id_str = match args.get("id").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return ToolCallResult::error("El argumento 'id' es requerido para brain_forget."),
    };

    let mem_id = match MemoryId::from_str(id_str) {
        Ok(id) => id,
        Err(e) => return ToolCallResult::error(format!("UUID inválido '{id_str}': {e}")),
    };

    match forget_uc.execute(mem_id).await {
        Ok(()) => {
            info!(memory_id = %mem_id, "Recuerdo eliminado exitosamente vía brain_forget MCP");
            ToolCallResult::success(format!(
                "Recuerdo con ID '{mem_id}' ha sido eliminado lógicamente (soft-delete) de Local Brain."
            ))
        }
        Err(ApplicationError::NotFound(_)) => ToolCallResult::error(format!(
            "Recuerdo con ID '{mem_id}' no encontrado o ya eliminado."
        )),
        Err(e) => {
            error!(error = %e, "Fallo al ejecutar ForgetUseCase vía MCP");
            ToolCallResult::error(format!("Error al eliminar recuerdo: {e}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brain_domain::ports::InMemoryMemoryRepository;

    #[tokio::test]
    async fn list_tools_returns_standard_tools() {
        let tools = list_tools();
        let names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
        assert!(names.contains(&"brain_remember".to_string()));
        assert!(names.contains(&"brain_recall".to_string()));
        assert!(names.contains(&"brain_search".to_string()));
        assert!(names.contains(&"brain_forget".to_string()));
    }

    #[tokio::test]
    async fn remember_and_recall_via_tools() {
        let repo = Arc::new(InMemoryMemoryRepository::new());
        let remember_uc = Arc::new(RememberUseCase::new(repo.clone()));
        let recall_uc = Arc::new(RecallUseCase::new(repo.clone()));
        let security = McpSecurityPolicy::new_default();

        // 1. Remember
        let remember_args = json!({
            "content": "Aprender Rust con Arquitectura Hexagonal",
            "project": "local-brain",
            "importance": 0.9
        });
        let res = execute_remember(remember_args, remember_uc, &security).await;
        assert!(!res.is_error);
        assert!(res.content[0]
            .text
            .contains("Recuerdo almacenado con éxito"));

        // 2. Recall por proyecto
        let recall_args = json!({
            "project": "local-brain"
        });
        let recall_res = execute_recall(recall_args, recall_uc, &security).await;
        assert!(!recall_res.is_error);
        assert!(recall_res.content[0]
            .text
            .contains("Aprender Rust con Arquitectura Hexagonal"));
    }

    #[tokio::test]
    async fn forget_requires_delete_permission_and_confirm() {
        let repo = Arc::new(InMemoryMemoryRepository::new());
        let remember_uc = Arc::new(RememberUseCase::new(repo.clone()));
        let forget_uc = Arc::new(ForgetUseCase::new(repo.clone()));

        // Guardar
        let mem = remember_uc
            .execute(RememberCommand::new("Dato efímero"))
            .await
            .unwrap();

        // Política por defecto NO tiene Delete
        let default_policy = McpSecurityPolicy::new_default();
        let res_no_perm = execute_forget(
            json!({"id": mem.id.to_string(), "confirm": true}),
            forget_uc.clone(),
            &default_policy,
        )
        .await;
        assert!(res_no_perm.is_error);
        assert!(res_no_perm.content[0].text.contains("Permiso denegado"));

        // Política completa con Delete pero confirm: false
        let full_policy = McpSecurityPolicy::new_full();
        let res_no_confirm = execute_forget(
            json!({"id": mem.id.to_string(), "confirm": false}),
            forget_uc.clone(),
            &full_policy,
        )
        .await;
        assert!(res_no_confirm.is_error);
        assert!(res_no_confirm.content[0].text.contains("confirm"));

        // Política completa y confirm: true
        let res_ok = execute_forget(
            json!({"id": mem.id.to_string(), "confirm": true}),
            forget_uc.clone(),
            &full_policy,
        )
        .await;
        assert!(!res_ok.is_error);
        assert!(res_ok.content[0].text.contains("eliminado lógicamente"));
    }
}
