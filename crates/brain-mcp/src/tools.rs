//! Definición, esquemas JSON y despachadores de herramientas MCP (SRS §21, §31, §32).

use std::str::FromStr;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde_json::json;
use tracing::{error, info, warn};

use brain_application::{
    ApplicationError, ExpireSessionUseCase, ExplainQuery, ExplainUseCase, ForgetUseCase,
    LearnCommand, LearnUseCase, RecallQuery, RecallUseCase, RelateCommand, RelateUseCase,
    RememberCommand, RememberUseCase, TraverseGraphQuery, TraverseGraphUseCase,
};
use brain_domain::model::{MemoryId, MemoryType, ProcedureStep};
use brain_graph::model::{RelationType, TraversalDirection};
use brain_learning::model::EvidenceSourceType;

use crate::protocol::{ToolCallResult, ToolDefinition};
use crate::security::{
    format_retrieved_memories, wrap_untrusted_explanation, McpPermission, McpSecurityError,
    McpSecurityPolicy,
};

/// Retorna la lista de definiciones de herramientas estándar expuestas por Local Brain (SRS §21, §10).
pub fn list_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "brain_remember".to_string(),
            description: "Registra un nuevo recuerdo o experiencia en Local Brain con persistencia local, vector de embeddings de 768 dimensiones, especialización cognitiva (episodic, semantic, procedural, associative, working) y metadatos de procedencia (SRS §10, §21.2).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "Contenido textual del recuerdo a almacenar (opcional si se proveen campos especializados estructurados)."
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
                        "description": "Identificador de sesión activa para trazabilidad o ciclo de vida de memoria de trabajo."
                    },
                    "ttl_seconds": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Tiempo de vida en segundos antes de expirar (para memorias de trabajo)."
                    },
                    "context": {
                        "type": "string",
                        "description": "Contexto situacional (para recuerdos episódicos especializados)."
                    },
                    "action": {
                        "type": "string",
                        "description": "Acción o decisión adoptada (para recuerdos episódicos especializados)."
                    },
                    "outcome": {
                        "type": "string",
                        "description": "Resultado u observación empírica obtenida (para recuerdos episódicos especializados)."
                    },
                    "statement": {
                        "type": "string",
                        "description": "Afirmación o directriz generalizada (para memoria semántica)."
                    },
                    "evidence_ids": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "UUIDs de evidencias que respaldan una afirmación semántica (requerido si confianza >= 0.8)."
                    },
                    "domain_area": {
                        "type": "string",
                        "description": "Área de conocimiento (para memoria semántica)."
                    },
                    "goal": {
                        "type": "string",
                        "description": "Meta u objetivo (para memorias de trabajo o procedimientos)."
                    },
                    "steps": {
                        "type": "array",
                        "description": "Secuencia ordenada de pasos técnicos (para recuerdos procedimentales)."
                    },
                    "source_concept": {
                        "type": "string",
                        "description": "Concepto o entidad de origen (para recuerdos asociativos)."
                    },
                    "target_concept": {
                        "type": "string",
                        "description": "Concepto o entidad de destino (para recuerdos asociativos)."
                    },
                    "predicate": {
                        "type": "string",
                        "description": "Predicado o relación conceptual (para recuerdos asociativos)."
                    },
                    "strength": {
                        "type": "number",
                        "minimum": 0.0,
                        "maximum": 1.0,
                        "description": "Fuerza asociativa normalizada [0.0, 1.0] (por defecto: 0.5)."
                    }
                }
            }),
        },
        ToolDefinition {
            name: "brain_recall".to_string(),
            description: "Recupera recuerdos cognitivos relevantes mediante búsqueda semántica vectorial (768 dimensiones), ID directo o filtros básicos de proyecto/tipo/sesión/concepto (SRS §21.3).".to_string(),
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
                    "session_id": {
                        "type": "string",
                        "description": "Filtrar por identificador de sesión (útil para working memory activa)."
                    },
                    "concept": {
                        "type": "string",
                        "description": "Filtrar asociaciones conceptuales relacionadas con este término."
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
            description: "Búsqueda avanzada multicriterio en Local Brain: permite filtrar simultáneamente por texto/semántica, proyecto, tipo, rango de fechas (ISO 8601), sesión, concepto y umbral mínimo de importancia (SRS §21.1).".to_string(),
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
                    "session_id": {
                        "type": "string",
                        "description": "Filtrar por identificador de sesión."
                    },
                    "concept": {
                        "type": "string",
                        "description": "Filtrar asociaciones conceptuales por término."
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
        ToolDefinition {
            name: "brain_session_end".to_string(),
            description: "Finaliza una sesión de trabajo activa en Local Brain, archivando o purgando todos los recuerdos de trabajo (working memory) temporales asociados a dicha sesión (SRS §10.1, §21.4).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Identificador de la sesión activa que finaliza."
                    }
                },
                "required": ["session_id"]
            }),
        },
        ToolDefinition {
            name: "brain_relate".to_string(),
            description: "Establece una relación semántica tipada entre dos recuerdos o conceptos en el Grafo de Conocimiento (SRS §11, §21.1, F5-03). Soporta 10 tipos (RELATED_TO, USED_IN, CAUSED_BY, SOLVES, CONTRADICTS, SUPERSEDES, DERIVED_FROM, DEPENDS_ON, PREFERS, AVOID) con prevención determinista de ciclos.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "source": {
                        "type": "string",
                        "description": "ID UUID de memoria o nombre del concepto origen."
                    },
                    "target": {
                        "type": "string",
                        "description": "ID UUID de memoria o nombre del concepto destino."
                    },
                    "relation": {
                        "type": "string",
                        "enum": [
                            "RELATED_TO", "USED_IN", "CAUSED_BY", "SOLVES", "CONTRADICTS",
                            "SUPERSEDES", "DERIVED_FROM", "DEPENDS_ON", "PREFERS", "AVOID"
                        ],
                        "description": "Tipo de relación semántica en el grafo."
                    },
                    "weight": {
                        "type": "number",
                        "minimum": 0.0,
                        "maximum": 1.0,
                        "description": "Fuerza o peso de la relación en [0.0, 1.0] (por defecto: 1.0)."
                    },
                    "context": {
                        "type": "string",
                        "description": "Justificación o contexto explicativo de la relación."
                    }
                },
                "required": ["source", "target", "relation"]
            }),
        },
        ToolDefinition {
            name: "brain_graph".to_string(),
            description: "Navega y consulta vecindades recursivas multi-salto en el Grafo de Conocimiento a partir de un recuerdo o concepto (SRS §11, §13.2, F5-03).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "node": {
                        "type": "string",
                        "description": "ID UUID de memoria o nombre de concepto a explorar."
                    },
                    "depth": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 5,
                        "description": "Profundidad máxima en saltos de arista (1 a 5, por defecto: 1)."
                    },
                    "direction": {
                        "type": "string",
                        "enum": ["outbound", "inbound", "both"],
                        "description": "Dirección de exploración de aristas (por defecto: 'both')."
                    },
                    "relation_types": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Filtrar exclusivamente por tipos de relación específicos."
                    },
                    "min_weight": {
                        "type": "number",
                        "minimum": 0.0,
                        "maximum": 1.0,
                        "description": "Umbral mínimo de peso de arista."
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 100,
                        "description": "Cantidad máxima de nodos a retornar (por defecto: 20)."
                    }
                },
                "required": ["node"]
            }),
        },
        ToolDefinition {
            name: "brain_learn".to_string(),
            description: "Registra una observación empírica o propone una creencia candidata sujeta a verificación en el ciclo de aprendizaje de Local Brain (SRS §15, §21.4, §59, §60). Permite añadir evidencia de soporte o refutación.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "statement": {
                        "type": "string",
                        "description": "Afirmación, creencia o contenido de la observación."
                    },
                    "evidence": {
                        "type": "string",
                        "description": "Evidencia empírica inicial que respalda o refuta la afirmación."
                    },
                    "source_type": {
                        "type": "string",
                        "enum": ["human", "tool_execution", "direct_observation", "agent_hypothesis"],
                        "description": "Origen o tipo de la fuente de evidencia (por defecto: 'direct_observation')."
                    },
                    "domain": {
                        "type": "string",
                        "description": "Área técnica, proyecto o dominio temático asociado."
                    },
                    "agent": {
                        "type": "string",
                        "description": "Identificador del agente que reporta el aprendizaje."
                    },
                    "is_supporting": {
                        "type": "boolean",
                        "description": "Indica si la evidencia apoya (true) o refuta/contradice (false) la afirmación (por defecto: true)."
                    },
                    "human_validated": {
                        "type": "boolean",
                        "description": "Indica si la creencia cuenta con aprobación y validación humana explícita."
                    },
                    "is_observation": {
                        "type": "boolean",
                        "description": "Si es true, se registra como una observación factual puntual sin generalizar a creencia (SRS §59)."
                    }
                },
                "required": ["statement"]
            }),
        },
        ToolDefinition {
            name: "brain_explain".to_string(),
            description: "Responde '¿Por qué el sistema cree esto?' desglosando la conclusión, evidencias empíricas acumuladas, consistencia, historial y nivel de confianza (SRS §21.5, §57).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Pregunta conceptual, afirmación o identificador UUID del conocimiento a explicar (ej. '¿Por qué recomendamos .NET?')."
                    },
                    "domain": {
                        "type": "string",
                        "description": "Dominio o proyecto opcional para filtrar la búsqueda."
                    }
                },
                "required": ["query"]
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

    let project = args
        .get("project")
        .and_then(|v| v.as_str())
        .unwrap_or("default")
        .to_string();
    let agent = args
        .get("agent")
        .and_then(|v| v.as_str())
        .unwrap_or("mcp-agent")
        .to_string();
    let mtype_str = args.get("memory_type").and_then(|v| v.as_str());
    let mtype = mtype_str.and_then(parse_memory_type);

    let content_opt = args
        .get("content")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty());
    let context_opt = args
        .get("context")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty());
    let action_opt = args
        .get("action")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty());
    let outcome_opt = args
        .get("outcome")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty());

    let source_concept_opt = args
        .get("source_concept")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty());
    let target_concept_opt = args
        .get("target_concept")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty());
    let predicate_opt = args
        .get("predicate")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty());

    let session_id_opt = args
        .get("session_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty());
    let ttl_seconds = args.get("ttl_seconds").and_then(|v| v.as_u64());

    let cmd_res: Result<RememberCommand, String> = if let (Some(ctx), Some(act), Some(out)) =
        (context_opt, action_opt, outcome_opt)
    {
        RememberCommand::episodic(&project, &agent, ctx, act, out)
            .map_err(|e| format!("Error en memoria episódica: {e}"))
    } else if let (Some(src), Some(tgt), Some(pred)) =
        (source_concept_opt, target_concept_opt, predicate_opt)
    {
        let strength = args.get("strength").and_then(|v| v.as_f64()).unwrap_or(0.5) as f32;
        RememberCommand::associative(src, tgt, pred, strength)
            .map_err(|e| format!("Error en memoria asociativa: {e}"))
    } else if args.get("steps").is_some() {
        let mut parsed_steps = Vec::new();
        if let Some(steps_arr) = args.get("steps").and_then(|v| v.as_array()) {
            for (idx, step_val) in steps_arr.iter().enumerate() {
                let default_num = (idx + 1) as u32;
                if let Some(desc) = step_val.as_str() {
                    match ProcedureStep::new(default_num, desc) {
                        Ok(step) => parsed_steps.push(step),
                        Err(e) => {
                            return ToolCallResult::error(format!(
                                "Paso inválido {default_num}: {e}"
                            ))
                        }
                    }
                } else if let Some(obj) = step_val.as_object() {
                    let num = obj
                        .get("step_number")
                        .and_then(|n| n.as_u64())
                        .map(|n| n as u32)
                        .unwrap_or(default_num);
                    let desc = obj
                        .get("description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("");
                    match ProcedureStep::new(num, desc) {
                        Ok(mut step) => {
                            if let Some(cmd) = obj.get("command").and_then(|c| c.as_str()) {
                                step = step.with_command(cmd);
                            }
                            if let Some(act) = obj.get("action_type").and_then(|a| a.as_str()) {
                                step = step.with_action_type(act);
                            }
                            parsed_steps.push(step);
                        }
                        Err(e) => {
                            return ToolCallResult::error(format!("Paso inválido {num}: {e}"))
                        }
                    }
                }
            }
        }
        let name = content_opt
            .or_else(|| args.get("name").and_then(|v| v.as_str()))
            .unwrap_or("Procedimiento sin nombre");
        let goal = args
            .get("goal")
            .and_then(|v| v.as_str())
            .unwrap_or("Objetivo técnico");
        RememberCommand::procedural(name, goal, parsed_steps)
            .map_err(|e| format!("Error en memoria procedimental: {e}"))
    } else if mtype == Some(MemoryType::Working)
        || (session_id_opt.is_some() && mtype.is_none() && ttl_seconds.is_some())
    {
        let sid = session_id_opt.unwrap_or("default-session");
        let content = content_opt
            .or_else(|| args.get("goal").and_then(|v| v.as_str()))
            .unwrap_or("Memoria de trabajo");
        RememberCommand::working(sid, content, ttl_seconds)
            .map_err(|e| format!("Error en memoria de trabajo: {e}"))
    } else if mtype == Some(MemoryType::Semantic) {
        let statement = content_opt
            .or_else(|| args.get("statement").and_then(|v| v.as_str()))
            .unwrap_or("");
        if statement.trim().is_empty() {
            return ToolCallResult::error(
                "La memoria semántica requiere 'statement' o 'content' no vacío.",
            );
        }
        let confidence = args
            .get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.3) as f32;
        let mut evidence_ids = Vec::new();
        if let Some(ev_arr) = args.get("evidence_ids").and_then(|v| v.as_array()) {
            for ev in ev_arr {
                if let Some(s) = ev.as_str() {
                    match MemoryId::from_str(s) {
                        Ok(id) => evidence_ids.push(id),
                        Err(e) => {
                            return ToolCallResult::error(format!(
                                "ID de evidencia inválido '{s}': {e}"
                            ))
                        }
                    }
                }
            }
        }
        RememberCommand::semantic(statement, confidence, evidence_ids)
            .map_err(|e| format!("Error en memoria semántica: {e}"))
    } else if let Some(content) = content_opt {
        Ok(RememberCommand::new(content))
    } else {
        return ToolCallResult::error(
                "Se requiere 'content' o parámetros especializados (episodic: context/action/outcome, associative: source/target/predicate, procedural: steps, working: session_id).",
            );
    };

    let mut cmd = match cmd_res {
        Ok(c) => c,
        Err(err_msg) => return ToolCallResult::error(err_msg),
    };

    if let Some(mt) = mtype {
        cmd = cmd.with_type(mt);
    }
    if args.get("project").is_some() {
        cmd = cmd.with_project(project);
    }
    if args.get("agent").is_some() {
        cmd = cmd.with_agent(agent);
    }
    if let Some(sid) = session_id_opt {
        cmd = cmd.with_session(sid);
    }
    if let Some(ttl) = ttl_seconds {
        cmd = cmd.with_ttl(ttl);
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

    if let Some(session_id) = args.get("session_id").and_then(|v| v.as_str()) {
        query.session_id = Some(session_id.to_string());
    }

    if let Some(concept) = args.get("concept").and_then(|v| v.as_str()) {
        query.concept = Some(concept.to_string());
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

    if let Some(session_id) = args.get("session_id").and_then(|v| v.as_str()) {
        query.session_id = Some(session_id.to_string());
    }

    if let Some(concept) = args.get("concept").and_then(|v| v.as_str()) {
        query.concept = Some(concept.to_string());
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

/// Ejecuta la herramienta `brain_session_end` (SRS §10.1, §21.4).
pub async fn execute_session_end(
    args: serde_json::Value,
    expire_session_uc: Option<Arc<ExpireSessionUseCase>>,
    security: &McpSecurityPolicy,
) -> ToolCallResult {
    if let Err(e) = security.check_permission(McpPermission::Write) {
        return ToolCallResult::error(format!("Error de seguridad: {e}"));
    }

    let session_id = match args.get("session_id").and_then(|v| v.as_str()) {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => {
            return ToolCallResult::error(
                "El argumento 'session_id' es requerido y no puede estar vacío.",
            )
        }
    };

    let Some(uc) = expire_session_uc else {
        return ToolCallResult::error(
            "El caso de uso ExpireSessionUseCase no está disponible en este servidor.",
        );
    };

    match uc.execute(&session_id).await {
        Ok(count) => {
            info!(session_id = %session_id, count = count, "Sesión finalizada exitosamente vía MCP");
            ToolCallResult::success(format!(
                "Sesión '{session_id}' finalizada. Se archivaron/expiraron {count} recuerdo(s) de trabajo."
            ))
        }
        Err(e) => {
            error!(error = %e, "Fallo al ejecutar ExpireSessionUseCase vía MCP");
            ToolCallResult::error(format!("Error al finalizar sesión: {e}"))
        }
    }
}

/// Ejecuta la herramienta `brain_relate` (SRS §11, §21.1, F5-03).
pub async fn execute_relate(
    args: serde_json::Value,
    relate_uc: Option<Arc<RelateUseCase>>,
    security: &McpSecurityPolicy,
) -> ToolCallResult {
    if let Err(e) = security.check_permission(McpPermission::Write) {
        return ToolCallResult::error(format!("Error de seguridad: {e}"));
    }

    let source = match args.get("source").and_then(|v| v.as_str()) {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => return ToolCallResult::error("El argumento 'source' es obligatorio."),
    };

    let target = match args.get("target").and_then(|v| v.as_str()) {
        Some(t) if !t.trim().is_empty() => t.to_string(),
        _ => return ToolCallResult::error("El argumento 'target' es obligatorio."),
    };

    let relation_str = match args.get("relation").and_then(|v| v.as_str()) {
        Some(r) => r,
        None => return ToolCallResult::error("El argumento 'relation' es obligatorio."),
    };

    let relation = match RelationType::from_str(relation_str) {
        Ok(r) => r,
        Err(_) => {
            return ToolCallResult::error(format!(
                "Tipo de relación inválido: '{relation_str}'. Debe ser uno de: RELATED_TO, USED_IN, CAUSED_BY, SOLVES, CONTRADICTS, SUPERSEDES, DERIVED_FROM, DEPENDS_ON, PREFERS, AVOID."
            ));
        }
    };

    let weight = args
        .get("weight")
        .and_then(|v| v.as_f64())
        .map(|w| w as f32);
    let context = args
        .get("context")
        .and_then(|v| v.as_str())
        .map(|c| c.to_string());

    let Some(uc) = relate_uc else {
        return ToolCallResult::error(
            "El caso de uso RelateUseCase no está disponible en este servidor.",
        );
    };

    let mut cmd = RelateCommand::new(source, target, relation);
    if let Some(w) = weight {
        cmd = cmd.with_weight(w);
    }
    if let Some(ctx) = context {
        cmd = cmd.with_context(ctx);
    }

    match uc.execute(cmd).await {
        Ok(edge) => {
            info!(
                edge_id = %edge.id,
                relation = %edge.relation_type,
                "Relación semántica registrada exitosamente vía MCP"
            );
            ToolCallResult::success(format!(
                "Relación establecida exitosamente en el grafo de conocimiento.\nID Arista: {}\nTipo: {}\nPeso: {:.2}",
                edge.id, edge.relation_type, edge.weight
            ))
        }
        Err(e) => {
            warn!(error = %e, "Fallo al ejecutar RelateUseCase vía MCP");
            ToolCallResult::error(format!("Error al registrar relación en el grafo: {e}"))
        }
    }
}

/// Ejecuta la herramienta `brain_graph` (SRS §11, §13.2, F5-03).
pub async fn execute_graph(
    args: serde_json::Value,
    traverse_uc: Option<Arc<TraverseGraphUseCase>>,
    security: &McpSecurityPolicy,
) -> ToolCallResult {
    if let Err(e) = security.check_permission(McpPermission::Read) {
        return ToolCallResult::error(format!("Error de seguridad: {e}"));
    }

    let node = match args.get("node").and_then(|v| v.as_str()) {
        Some(n) if !n.trim().is_empty() => n.to_string(),
        _ => return ToolCallResult::error("El argumento 'node' es obligatorio."),
    };

    let depth = args.get("depth").and_then(|v| v.as_u64()).map(|d| d as u32);
    let direction = args
        .get("direction")
        .and_then(|v| v.as_str())
        .and_then(|d| TraversalDirection::from_str(d).ok());
    let min_weight = args
        .get("min_weight")
        .and_then(|v| v.as_f64())
        .map(|w| w as f32);
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|l| l as usize);

    let relation_types = args
        .get("relation_types")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|val| val.as_str().and_then(|s| RelationType::from_str(s).ok()))
                .collect()
        });

    let Some(uc) = traverse_uc else {
        return ToolCallResult::error(
            "El caso de uso TraverseGraphUseCase no está disponible en este servidor.",
        );
    };

    let mut query = TraverseGraphQuery::new(node);
    if let Some(d) = depth {
        query = query.with_depth(d);
    }
    if let Some(dir) = direction {
        query = query.with_direction(dir);
    }
    if let Some(rels) = relation_types {
        query = query.with_relations(rels);
    }
    query.min_weight = min_weight;
    query.limit = limit;

    match uc.execute(query).await {
        Ok(subgraph) => {
            let mut out = format!(
                "Grafo de Conocimiento centrado en '{}' (Tipo: {}, ID: {})\n",
                subgraph.root.label, subgraph.root.node_type, subgraph.root.id
            );
            out.push_str(&format!(
                "Total nodos conectados: {}\nTotal aristas: {}\n\n",
                subgraph.nodes.len(),
                subgraph.edges.len()
            ));

            out.push_str("=== Pasos de Exploración / Vecindad ===\n");
            for step in &subgraph.steps {
                if step.depth == 0 {
                    out.push_str(&format!("* [Raíz] {}\n", step.label));
                } else {
                    let rel_str = step
                        .relation_type
                        .map_or("CONECTADO_A".to_string(), |r| r.to_string());
                    out.push_str(&format!(
                        "  └─ (Salto {}) --[{}]--> {} (ID: {}, Peso acumulado: {:.2})\n",
                        step.depth, rel_str, step.label, step.node_id, step.accumulated_weight
                    ));
                }
            }

            let sanitized = format!(
                "<untrusted_graph_context>\n{}\n</untrusted_graph_context>",
                out.replace(
                    "</untrusted_graph_context>",
                    "&lt;/untrusted_graph_context&gt;"
                )
            );

            ToolCallResult::success(sanitized)
        }
        Err(e) => {
            warn!(error = %e, "Fallo al ejecutar TraverseGraphUseCase vía MCP");
            ToolCallResult::error(format!("Error al explorar el grafo: {e}"))
        }
    }
}

/// Ejecuta la herramienta `brain_learn` (SRS §15, §21.4, §59, §60, Fase 6).
pub async fn execute_learn(
    args: serde_json::Value,
    learn_uc: Option<Arc<LearnUseCase>>,
    security: &McpSecurityPolicy,
) -> ToolCallResult {
    if let Err(e) = security.check_permission(McpPermission::Write) {
        return ToolCallResult::error(format!("Error de seguridad: {e}"));
    }

    let statement = match args.get("statement").and_then(|v| v.as_str()) {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => {
            return ToolCallResult::error(
                "El argumento 'statement' es obligatorio y no puede estar vacío.",
            )
        }
    };

    let Some(uc) = learn_uc else {
        return ToolCallResult::error(
            "El caso de uso LearnUseCase no está configurado en este servidor MCP.",
        );
    };

    let is_observation = args
        .get("is_observation")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let mut cmd = if is_observation {
        LearnCommand::new_observation(statement)
    } else {
        LearnCommand::new_candidate(statement)
    };

    if let Some(ev) = args.get("evidence").and_then(|v| v.as_str()) {
        let src_type = args
            .get("source_type")
            .and_then(|v| v.as_str())
            .and_then(|s| EvidenceSourceType::from_str(s).ok())
            .unwrap_or(EvidenceSourceType::DirectObservation);
        cmd = cmd.with_evidence(ev, src_type);
    }

    if let Some(dom) = args.get("domain").and_then(|v| v.as_str()) {
        cmd = cmd.with_domain(dom);
    }
    if let Some(ag) = args.get("agent").and_then(|v| v.as_str()) {
        cmd = cmd.with_agent(ag);
    }
    if let Some(supp) = args.get("is_supporting").and_then(|v| v.as_bool()) {
        cmd = cmd.with_supporting(supp);
    }
    if let Some(hv) = args.get("human_validated").and_then(|v| v.as_bool()) {
        cmd = cmd.with_human_validated(hv);
    }

    match uc.execute(cmd).await {
        Ok(res) => {
            info!(
                candidate_id = %res.candidate_id,
                stage = %res.stage,
                confidence = res.confidence,
                "Aprendizaje registrado exitosamente vía MCP"
            );
            ToolCallResult::success(format!(
                "Aprendizaje registrado exitosamente.\nID Candidato: {}\nEtapa: {}\nConfianza: {:.2}\nEvidencias: {}\nValidado por humano: {}",
                res.candidate_id,
                res.stage.as_str().to_uppercase(),
                res.confidence,
                res.evidence_count,
                if res.human_validated { "Sí" } else { "No" }
            ))
        }
        Err(e) => {
            warn!(error = %e, "Fallo al ejecutar LearnUseCase vía MCP");
            ToolCallResult::error(format!("Error al registrar aprendizaje: {e}"))
        }
    }
}

/// Ejecuta la herramienta `brain_explain` (SRS §21.5, §57, Fase 6).
pub async fn execute_explain(
    args: serde_json::Value,
    explain_uc: Option<Arc<ExplainUseCase>>,
    security: &McpSecurityPolicy,
) -> ToolCallResult {
    if let Err(e) = security.check_permission(McpPermission::Read) {
        return ToolCallResult::error(format!("Error de seguridad: {e}"));
    }

    let query = match args.get("query").and_then(|v| v.as_str()) {
        Some(q) if !q.trim().is_empty() => q.to_string(),
        _ => {
            return ToolCallResult::error(
                "El argumento 'query' es obligatorio y no puede estar vacío.",
            )
        }
    };

    let Some(uc) = explain_uc else {
        return ToolCallResult::error(
            "El caso de uso ExplainUseCase no está configurado en este servidor MCP.",
        );
    };

    let mut q = ExplainQuery::new(query);
    if let Some(dom) = args.get("domain").and_then(|v| v.as_str()) {
        q = q.with_domain(dom);
    }

    match uc.execute(q).await {
        Ok(report) => {
            let sanitized = wrap_untrusted_explanation(
                &report.formatted_explanation,
                &report.conclusion,
                report.confidence,
            );
            ToolCallResult::success(sanitized)
        }
        Err(e) => {
            warn!(error = %e, "Fallo al ejecutar ExplainUseCase vía MCP");
            ToolCallResult::error(format!("Error al generar explicación: {e}"))
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
        assert!(names.contains(&"brain_session_end".to_string()));
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
    async fn remember_specialized_episodic_and_session_lifecycle() {
        let repo = Arc::new(InMemoryMemoryRepository::new());
        let remember_uc = Arc::new(RememberUseCase::new(repo.clone()));
        let recall_uc = Arc::new(RecallUseCase::new(repo.clone()));
        let expire_uc = Arc::new(ExpireSessionUseCase::new(repo.clone()));
        let security = McpSecurityPolicy::new_default();

        // 1. Remember episodic especializado
        let episodic_args = json!({
            "context": "Migración de base de datos",
            "action": "Agregar columna expires_at",
            "outcome": "Migración ejecutada sin downtime",
            "project": "local-brain"
        });
        let res_episodic = execute_remember(episodic_args, remember_uc.clone(), &security).await;
        assert!(!res_episodic.is_error);
        assert!(res_episodic.content[0].text.contains("Episodic"));

        // 2. Remember working memory ligado a sesión
        let working_args = json!({
            "memory_type": "working",
            "session_id": "ses-mcp-1",
            "content": "Variable temporal de refactor",
            "ttl_seconds": 60
        });
        let res_work = execute_remember(working_args, remember_uc.clone(), &security).await;
        assert!(!res_work.is_error);

        // 3. Recall por sesión
        let recall_args = json!({
            "session_id": "ses-mcp-1"
        });
        let recall_res = execute_recall(recall_args, recall_uc.clone(), &security).await;
        assert!(!recall_res.is_error);
        assert!(recall_res.content[0]
            .text
            .contains("Variable temporal de refactor"));

        // 4. Finalizar sesión
        let end_args = json!({
            "session_id": "ses-mcp-1"
        });
        let end_res = execute_session_end(end_args, Some(expire_uc), &security).await;
        assert!(!end_res.is_error);
        assert!(end_res.content[0]
            .text
            .contains("Se archivaron/expiraron 1 recuerdo(s)"));

        // 5. Recall posterior: la memoria expirada no aparece activa
        let recall_after =
            execute_recall(json!({"session_id": "ses-mcp-1"}), recall_uc, &security).await;
        assert!(!recall_after.is_error);
        assert!(recall_after.content[0]
            .text
            .contains("No se encontraron recuerdos coincidentes"));
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

    #[tokio::test]
    async fn test_relate_and_graph_tools_flow() {
        use brain_graph::InMemoryGraphRepository;

        let graph_repo = Arc::new(InMemoryGraphRepository::new());
        let relate_uc = Arc::new(RelateUseCase::new(graph_repo.clone()));
        let traverse_uc = Arc::new(TraverseGraphUseCase::new(graph_repo));
        let policy = McpSecurityPolicy::new_full();

        // 1. Vincular conceptos
        let relate_args = json!({
            "source": "Rust",
            "target": "PostgreSQL",
            "relation": "USED_IN",
            "weight": 0.9,
            "context": "Base de datos principal"
        });
        let res_relate = execute_relate(relate_args, Some(relate_uc.clone()), &policy).await;
        assert!(!res_relate.is_error);
        assert!(res_relate.content[0].text.contains("USED_IN"));

        // 2. Explorar grafo
        let graph_args = json!({
            "node": "Rust",
            "depth": 1
        });
        let res_graph = execute_graph(graph_args, Some(traverse_uc), &policy).await;
        assert!(!res_graph.is_error);
        let graph_txt = &res_graph.content[0].text;
        assert!(graph_txt.contains("<untrusted_graph_context>"));
        assert!(graph_txt.contains("PostgreSQL"));

        // 3. Permiso de solo lectura bloquea relate
        let ro_policy = McpSecurityPolicy::new_read_only();
        let ro_res = execute_relate(
            json!({"source": "A", "target": "B", "relation": "RELATED_TO"}),
            Some(relate_uc),
            &ro_policy,
        )
        .await;
        assert!(ro_res.is_error);
        assert!(ro_res.content[0].text.contains("Permiso denegado"));
    }

    #[tokio::test]
    async fn test_learn_and_explain_tools_flow() {
        use brain_learning::InMemoryLearningRepository;

        let learn_repo = Arc::new(InMemoryLearningRepository::new());
        let learn_uc = Arc::new(LearnUseCase::new(learn_repo.clone()));
        let explain_uc = Arc::new(ExplainUseCase::new(learn_repo));
        let policy = McpSecurityPolicy::new_full();

        // 1. Proponer candidato con evidencia vía MCP
        let learn_args = json!({
            "statement": ".NET es altamente recomendado para backends complejos",
            "evidence": "Experience #182: Manejo de reglas empresariales y roles",
            "source_type": "direct_observation",
            "domain": "backend",
            "human_validated": true
        });
        let res_learn = execute_learn(learn_args, Some(learn_uc.clone()), &policy).await;
        assert!(!res_learn.is_error);
        assert!(res_learn.content[0]
            .text
            .contains("Aprendizaje registrado exitosamente"));
        assert!(res_learn.content[0].text.contains("VALIDATED"));

        // 2. Consultar explicación vía MCP
        let explain_args = json!({
            "query": ".NET es altamente recomendado"
        });
        let res_explain = execute_explain(explain_args, Some(explain_uc), &policy).await;
        assert!(!res_explain.is_error);
        let exp_txt = &res_explain.content[0].text;
        assert!(exp_txt.contains("<untrusted_explanation_context"));
        assert!(exp_txt.contains("Conclusión:"));
        assert!(exp_txt.contains("Confianza:"));
        assert!(exp_txt.contains("Validación Humana:"));

        // 3. Política de solo lectura bloquea learn
        let ro_policy = McpSecurityPolicy::new_read_only();
        let ro_res = execute_learn(
            json!({"statement": "Prueba de solo lectura"}),
            Some(learn_uc),
            &ro_policy,
        )
        .await;
        assert!(ro_res.is_error);
        assert!(ro_res.content[0].text.contains("Permiso denegado"));
    }
}
