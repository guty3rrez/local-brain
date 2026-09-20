# Patrones de Flujo de Trabajo Agéntico — Local Brain

Patrones de interacción recomendados para agentes de IA que integran memoria persistente en sus tareas cotidianas.

---

## Patrón 1: Inicialización de Sesión e Ingesta de Contexto
Al comenzar a trabajar en un repositorio o feature, el agente consulta la memoria histórica acumulada para no repetir errores pasados ni violar convenciones preestablecidas.

```json
// Paso 1: Recuperación híbrida del proyecto actual
{
  "name": "brain_retrieve",
  "arguments": {
    "query": "convenciones arquitectónicas y problemas resueltos recientemente",
    "project": "local-brain",
    "limit": 5,
    "min_score": 0.4
  }
}
```

Si se identifican tecnologías clave en el contexto recuperado, el agente puede inspeccionar el grafo para ver directivas de preferencia:
```json
// Paso 2: Consultar relaciones asociadas a una tecnología
{
  "name": "brain_graph",
  "arguments": {
    "node": "PostgreSQL",
    "depth": 1,
    "direction": "both"
  }
}
```

---

## Patrón 2: Registro de Solución a un Problema Técnico (Memoria Episódica)
Cuando el agente investiga un error complejo, prueba varias alternativas y finalmente da con la solución correcta, debe registrar la experiencia situacional para que otros agentes (o él mismo en futuras sesiones) la aprovechen:

```json
{
  "name": "brain_remember",
  "arguments": {
    "project": "local-brain",
    "agent": "claude-code",
    "memory_type": "episodic",
    "importance": 0.85,
    "confidence": 0.90,
    "context": "Fallo en CI al ejecutar cargo test en contenedor alpine por símbolos ausentes de glibc",
    "action": "Migramos el target de compilación a x86_64-unknown-linux-musl e instalamos musl-tools en el runner",
    "outcome": "El binario compiló de forma completamente estática sin dependencias dinámicas, pasando todos los tests de CI"
  }
}
```

Adicionalmente, se conecta el problema y la solución en el Grafo de Conocimiento:
```json
{
  "name": "brain_relate",
  "arguments": {
    "source": "x86_64-unknown-linux-musl",
    "target": "Alpine-Linux-CI",
    "relation": "SOLVES",
    "weight": 0.95,
    "context": "Resuelve la incompatibilidad de símbolos glibc en entornos Alpine minimalistas"
  }
}
```

---

## Patrón 3: Ciclo de Aprendizaje Empírico (Observación vs Creencia)
Cuando el agente descubre un comportamiento que aún no es una verdad universal, lo registra como observación empírica o creencia candidata con nivel de confianza moderado:

```json
// Registrar observación empírica con evidencia
{
  "name": "brain_learn",
  "arguments": {
    "statement": "El índice HNSW con m=16 y ef_construction=64 reduce el uso de memoria en un 35% sin degradar el recall en benchmarks locales",
    "evidence": "Ejecución de benchmark con 50.000 vectores sintéticos en hardware Ryzen 7",
    "source_type": "tool_execution",
    "domain": "vector-search",
    "agent": "antigravity",
    "is_supporting": true
  }
}
```

Posteriormente, cuando un usuario o agente necesite entender el origen de una directriz técnica:
```json
{
  "name": "brain_explain",
  "arguments": {
    "query": "¿Por qué usamos m=16 en HNSW?",
    "domain": "vector-search"
  }
}
```

---

## Patrón 4: Registro de un Procedimiento Operativo Técnico
Para tareas recurrentes de compilación, despliegue o pruebas:

```json
{
  "name": "brain_remember",
  "arguments": {
    "project": "local-brain",
    "memory_type": "procedural",
    "content": "Protocolo de verificación previa a PR",
    "goal": "Garantizar cero fallos de linter, formato y tests antes de someter cambios",
    "steps": [
      {
        "step_number": 1,
        "description": "Verificar formateo oficial de Rust",
        "command": "cargo fmt --all -- --check"
      },
      {
        "step_number": 2,
        "description": "Ejecutar linter con advertencias tratadas como errores",
        "command": "cargo clippy --workspace --all-targets -- -D warnings"
      },
      {
        "step_number": 3,
        "description": "Ejecutar suite de tests unitarios puros de dominio",
        "command": "cargo test --workspace --lib"
      }
    ]
  }
}
```

---

## Patrón 5: Manejo de Sesión Efímera (Working Memory)
Para tareas en curso que duran múltiples turnos pero no deben persistir indefinidamente:

```json
// Paso 1: Registrar memoria de trabajo con TTL (30 minutos = 1800s)
{
  "name": "brain_remember",
  "arguments": {
    "memory_type": "working",
    "session_id": "session-refactor-retrieval-42",
    "content": "Refactorizando scoring de decaimiento: analizando impacto en tests de pipeline",
    "ttl_seconds": 1800
  }
}

// Paso 2: Al concluir la sesión de trabajo
{
  "name": "brain_session_end",
  "arguments": {
    "session_id": "session-refactor-retrieval-42"
  }
}
```
