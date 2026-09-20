# Tipos de Memoria Cognitiva — Local Brain

Local Brain rechaza el enfoque ingenuo de tratar toda memoria como texto plano o documentos indiferenciados. El sistema modela **cinco tipos cognitivos especializados**, cada uno con su propio ciclo de vida, reglas de decaimiento y validaciones invariantes de dominio.

---

## 🧠 Matriz Comparativa

| Tipo de Memoria | Naturaleza | Persistencia | Estructura Clave | Caso de Uso Típico |
| :--- | :--- | :--- | :--- | :--- |
| **`working`** | Estado de sesión activo | Efímera (TTL) | `session_id`, `ttl_seconds`, `goal` | Contexto temporal de la tarea actual, variables transitorias. |
| **`episodic`** | Experiencias situacionales | Permanente | `context`, `action`, `outcome` | Qué intentamos, qué hicimos y qué ocurrió (éxitos o fallos). |
| **`semantic`** | Hechos y directrices | Permanente | `statement`, `confidence`, `evidence_ids` | Reglas arquitectónicas, preferencias y conocimientos validados. |
| **`procedural`** | Pasos técnicos ordenados | Permanente | `goal`, `steps: [step_number, cmd, action]` | Guías operativas, secuencias de despliegue o compilación. |
| **`associative`** | Conexiones ontológicas | Permanente | `source_concept`, `target_concept`, `predicate`, `strength` | Relaciones conceptuales directas entre entidades o tecnologías. |

---

## 1. Memoria de Trabajo (`working`)
- **Propósito**: Mantener el contexto inmediato de una sesión de trabajo sin contaminar la memoria a largo plazo.
- **Invariantes**:
  - Requiere `session_id` no vacío.
  - Expira automáticamente al superarse su `ttl_seconds` o al cerrarse la sesión con `brain_session_end`.
  - No genera decaimiento temporal gradual: se purga o archiva al vencer su tiempo de vida.

---

## 2. Memoria Episódica (`episodic`)
- **Propósito**: Capturar la experiencia empírica vivencial de los agentes a lo largo del tiempo.
- **Invariantes**:
  - Se estructura bajo la tríada estricta:
    - **Contexto (`context`)**: La situación o problema que motivó la acción.
    - **Acción (`action`)**: La decisión técnica o comando ejecutado.
    - **Resultado (`outcome`)**: Lo que realmente ocurrió (logs de error, latencia, resultado funcional).
  - Admite importancia intrínseca (`importance` $\in [0.0, 1.0]$).
  - Es el insumo primario que el motor de **reflexión y consolidación** analiza para derivar nuevas hipótesis y creencias semánticas.

---

## 3. Memoria Semántica (`semantic`)
- **Propósito**: Almacenar afirmaciones, principios, directrices de diseño y hechos generalizados.
- **Invariantes**:
  - Contiene un `statement` no vacío.
  - Posee un grado de certidumbre empírica (`confidence` $\in [0.0, 1.0]$).
  - **Invariante de Evidencia Rigurosa**: Si `confidence >= 0.8`, la memoria **exige obligatoriamente** tener al menos un UUID en `evidence_ids` que respalde empíricamente la afirmación. Ningún agente puede declarar una verdad absoluta sin pruebas.

---

## 4. Memoria Procedimental (`procedural`)
- **Propósito**: Codificar secuencias operativas paso a paso para resolver problemas conocidos.
- **Invariantes**:
  - Define una meta técnica (`goal`) y un nombre identificador.
  - Contiene una lista ordenada de pasos (`steps`), donde cada paso tiene:
    - `step_number`: Número ordinal positivo (1, 2, 3...).
    - `description`: Qué hace el paso.
    - `command` *(opcional)*: El comando de terminal asociado.
    - `action_type` *(opcional)*: Clasificación de la acción.
  - Los números de paso deben ser estrictamente secuenciales y sin huecos.

---

## 5. Memoria Asociativa (`associative`)
- **Propósito**: Establecer triplas conceptuales semánticas rápidas entre entidades de conocimiento.
- **Invariantes**:
  - Tripla canónica: `source_concept` → `predicate` → `target_concept`.
  - Peso asociativo normalizado (`strength` $\in [0.0, 1.0]$).
  - `source_concept` y `target_concept` no pueden ser idénticos (prohibición estricta de auto-asociaciones vacías).
