# Catálogo de Herramientas MCP — Local Brain

Local Brain expone 11 herramientas estándar a través del protocolo **Model Context Protocol (MCP)** sobre `stdio`. Este catálogo detalla cada herramienta, sus parámetros, tipos, valores por defecto y casos de uso.

---

## 1. `brain_remember`
Registra un nuevo recuerdo o experiencia con persistencia local en PostgreSQL, vector de embeddings de 768 dimensiones generado por `llama.cpp` local (`nomic-embed-text-v1.5`), especialización cognitiva y procedencia.

### Parámetros
- `content` *(string, opcional si se usan campos estructurados)*: Contenido textual del recuerdo.
- `memory_type` *(string, opcional, enum: `"episodic"`, `"semantic"`, `"procedural"`, `"associative"`, `"working"`, defecto: `"episodic"`)*: Tipo cognitivo.
- `project` *(string, opcional, defecto: `"default"`)*: Nombre del proyecto.
- `agent` *(string, opcional, defecto: `"mcp-agent"`)*: Identificador del agente.
- `importance` *(number, opcional, [0.0 - 1.0], defecto: 0.5)*: Importancia intrínseca.
- `confidence` *(number, opcional, [0.0 - 1.0], defecto: 0.3)*: Nivel de certeza empírica.
- `session_id` *(string, opcional)*: Identificador de sesión para memoria de trabajo (`working`).
- `ttl_seconds` *(integer, opcional, >= 1)*: Tiempo de vida en segundos antes de expirar (para `working`).
- **Campos episódicos:**
  - `context` *(string)*: Contexto situacional (qué se estaba intentando).
  - `action` *(string)*: Decisión o acción tomada.
  - `outcome` *(string)*: Resultado u observación empírica obtenida.
- **Campos semánticos:**
  - `statement` *(string)*: Afirmación o directriz generalizada.
  - `evidence_ids` *(array de strings)*: UUIDs de evidencias que respaldan la afirmación (obligatorio si confianza >= 0.8).
  - `domain_area` *(string)*: Área técnica asociada.
- **Campos procedimentales:**
  - `goal` *(string)*: Meta u objetivo del procedimiento.
  - `steps` *(array)*: Lista de pasos técnicos (strings u objetos con `step_number`, `description`, `command`, `action_type`).
- **Campos asociativos:**
  - `source_concept` *(string)*: Concepto de origen.
  - `target_concept` *(string)*: Concepto de destino.
  - `predicate` *(string)*: Predicado o tipo de enlace conceptual.
  - `strength` *(number, [0.0 - 1.0], defecto: 0.5)*: Fuerza asociativa.

### Ejemplo
```json
{
  "project": "ecommerce",
  "memory_type": "episodic",
  "context": "Optimización de consultas lentas en listado de productos",
  "action": "Creamos un índice GiST con pgvector sobre embeddings de categoría",
  "outcome": "La latencia bajó de 230ms a 14ms en benchmark local",
  "importance": 0.85,
  "confidence": 0.90
}
```

---

## 2. `brain_recall`
Recupera recuerdos cognitivos mediante búsqueda semántica vectorial densa (768 dimensiones), ID directo o filtros básicos de proyecto/tipo/sesión/concepto.

### Parámetros
- `query` *(string, opcional)*: Texto para búsqueda semántica vectorial basada en similitud coseno.
- `id` *(string, opcional)*: UUID de un recuerdo específico.
- `project` *(string, opcional)*: Filtro por proyecto.
- `memory_type` *(string, opcional, enum: `"episodic"`, `"semantic"`, `"procedural"`, `"associative"`, `"working"`)*: Filtro por tipo.
- `session_id` *(string, opcional)*: Filtro por sesión activa.
- `concept` *(string, opcional)*: Filtro por término conceptual.
- `limit` *(integer, opcional, 1 a 50, defecto: 10)*: Límite de resultados.

---

## 3. `brain_search`
Búsqueda multicriterio avanzada que combina filtros semánticos, temporales (ISO 8601), de importancia y de sesión.

### Parámetros
- `query` *(string, opcional)*: Texto de búsqueda.
- `project` *(string, opcional)*: Filtro por proyecto.
- `memory_type` *(string, opcional)*: Filtro por tipo.
- `session_id` *(string, opcional)*: Filtro por sesión.
- `concept` *(string, opcional)*: Filtro por concepto.
- `min_importance` *(number, opcional, [0.0 - 1.0])*: Umbral mínimo de importancia.
- `from_date` *(string, opcional, ISO 8601)*: Fecha inicial de creación.
- `to_date` *(string, opcional, ISO 8601)*: Fecha final de creación.
- `limit` *(integer, opcional, 1 a 50, defecto: 10)*: Cantidad máxima de resultados.

---

## 4. `brain_retrieve`
Recuperación híbrida avanzada que combina los tres canales de búsqueda (Vectorial HNSW + Léxica Full-Text Search PostgreSQL + Expansión en Grafo de Conocimiento) con Reciprocal Rank Fusion (RRF), scoring multidimensional y decaimiento temporal exponencial.

### Parámetros
- `query` *(string, obligatorio)*: Consulta o intención de búsqueda semántica y léxica.
- `project` *(string, opcional)*: Filtro por proyecto.
- `memory_type` *(string, opcional)*: Filtro por tipo de memoria.
- `limit` *(integer, opcional, 1 a 100, defecto: 10)*: Límite de recuerdos a retornar.
- `min_score` *(number, opcional, [0.0 - 1.0])*: Puntaje mínimo de corte de ranking.
- `explain` *(boolean, opcional, defecto: false)*: Si es `true`, desglosa el cálculo matemático de scoring y señales de ranking.

### Ejemplo
```json
{
  "query": "cómo migrar esquemas de base de datos sin downtime",
  "project": "local-brain",
  "limit": 5,
  "explain": true
}
```

---

## 5. `brain_relate`
Establece una relación semántica tipada y ponderada entre dos recuerdos o conceptos en el Grafo de Conocimiento, con prevención estricta de ciclos en relaciones causales y jerárquicas (DAG).

### Parámetros
- `source` *(string, obligatorio)*: UUID de memoria o nombre de concepto origen.
- `target` *(string, obligatorio)*: UUID de memoria o nombre de concepto destino.
- `relation` *(string, obligatorio, enum: `"RELATED_TO"`, `"USED_IN"`, `"CAUSED_BY"`, `"SOLVES"`, `"CONTRADICTS"`, `"SUPERSEDES"`, `"DERIVED_FROM"`, `"DEPENDS_ON"`, `"PREFERS"`, `"AVOID"`)*: Tipo canónico de relación.
- `weight` *(number, opcional, [0.0 - 1.0], defecto: 1.0)*: Fuerza de la arista.
- `context` *(string, opcional)*: Justificación contextual de la relación.

### Ejemplo
```json
{
  "source": "PostgreSQL-17",
  "target": "SQLite",
  "relation": "PREFERS",
  "weight": 0.95,
  "context": "Concurrencia multiagente y extensiones nativas pgvector requeridas para memoria compartida"
}
```

---

## 6. `brain_graph`
Navega y explora vecindades en el Grafo de Conocimiento a partir de un nodo raíz con soporte para saltos recursivos (1 a 5 hops).

### Parámetros
- `node` *(string, obligatorio)*: UUID de memoria o nombre de concepto raíz.
- `depth` *(integer, opcional, 1 a 5, defecto: 1)*: Profundidad máxima de saltos.
- `direction` *(string, opcional, enum: `"outbound"`, `"inbound"`, `"both"`, defecto: `"both"`)*: Dirección de exploración de aristas.
- `relation_types` *(array de strings, opcional)*: Filtrar solo por relaciones específicas.
- `min_weight` *(number, opcional, [0.0 - 1.0])*: Umbral mínimo de peso de arista.
- `limit` *(integer, opcional, 1 a 100, defecto: 20)*: Límite de nodos a retornar.

---

## 7. `brain_learn`
Registra una observación empírica o propone una creencia candidata en el motor de aprendizaje continuo, calculando puntuación de confianza y vinculando evidencias empíricas.

### Parámetros
- `statement` *(string, obligatorio)*: Afirmación, creencia u observación fáctica.
- `evidence` *(string, opcional)*: Evidencia empírica inicial que respalda o refuta.
- `source_type` *(string, opcional, enum: `"human"`, `"tool_execution"`, `"direct_observation"`, `"agent_hypothesis"`, defecto: `"direct_observation"`)*: Origen de la evidencia.
- `domain` *(string, opcional)*: Dominio técnico o proyecto asociado.
- `agent` *(string, opcional)*: Identificador del agente.
- `is_supporting` *(boolean, opcional, defecto: true)*: Si es `true`, la evidencia apoya; si es `false`, refuta o contradice la afirmación.
- `human_validated` *(boolean, opcional, defecto: false)*: Si ha sido validado y confirmado por un humano (eleva drásticamente la confianza).
- `is_observation` *(boolean, opcional, defecto: false)*: Si es `true`, se almacena como observación factual puntual sin generalizar a creencia.

---

## 8. `brain_explain`
Responde a la pregunta fundamental: **"¿Por qué el sistema cree esto?"**, desglosando la conclusión, evidencias empíricas acumuladas, consistencia, historial de procedencia y nivel de confianza formal.

### Parámetros
- `query` *(string, obligatorio)*: Pregunta conceptual, afirmación o UUID del conocimiento a explicar.
- `domain` *(string, opcional)*: Dominio o proyecto para filtrar la búsqueda.

### Ejemplo
```json
{
  "query": "¿Por qué preferimos sqlx sobre orms reflexivos?",
  "domain": "backend"
}
```

---

## 9. `brain_consolidate`
Ejecuta el proceso de reflexión y consolidación asíncrona sobre clusters de experiencias recientes o memorias específicas, sintetizando nuevo conocimiento candidato y detectando contradicciones cognitivas.

### Parámetros
- `project` *(string, opcional)*: Filtro por proyecto para consolidar.
- `memory_ids` *(array de strings, opcional)*: Lista de UUIDs específicos para consolidar directamente.
- `similarity_threshold` *(number, opcional, [0.0 - 1.0], defecto: 0.82)*: Umbral de similitud coseno para clustering.
- `dry_run` *(boolean, opcional, defecto: false)*: Simular sin persistir cambios.

---

## 10. `brain_forget`
Elimina lógicamente (*soft-delete*) un recuerdo existente. Requiere confirmación explícita para evitar pérdidas accidentales.

### Parámetros
- `id` *(string, obligatorio)*: UUID del recuerdo a eliminar.
- `confirm` *(boolean, obligatorio)*: Debe ser estrictamente `true`.

---

## 11. `brain_session_end`
Finaliza una sesión de trabajo activa en Local Brain, archivando o purgando todos los recuerdos efímeros de trabajo (`working memory`) asociados a dicha sesión.

### Parámetros
- `session_id` *(string, obligatorio)*: Identificador de la sesión activa que finaliza.
