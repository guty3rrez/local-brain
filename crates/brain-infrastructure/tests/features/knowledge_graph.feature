# language: es
Característica: Grafo de conocimiento y relaciones conceptuales en PostgreSQL
  Como agente de Inteligencia Artificial o desarrollador
  Quiero conectar recuerdos y conceptos mediante relaciones semánticas tipadas
  Para navegar recursivamente vecindades y respetar invariantes de aciclicidad (SRS §11)

  Escenario: Conectar dos decisiones arquitectónicas mediante SUPERSEDES
    Dado un repositorio PostgreSQL conectado con tablas de grafo
    Cuando registro un recuerdo con decisión "Decisión A: SQLite"
    Y registro un nuevo recuerdo con decisión "Decisión B: PostgreSQL"
    Y vinculo "Decisión B: PostgreSQL" hacia "Decisión A: SQLite" con relación "SUPERSEDES"
    Entonces el grafo contiene la arista "SUPERSEDES" entre ambas decisiones
    Y al intentar vincular "Decisión A: SQLite" hacia "Decisión B: PostgreSQL" como "SUPERSEDES" la operación es rechazada por detección de ciclo

  Escenario: Consulta de vecindad y traversal de conceptos
    Dado un repositorio PostgreSQL conectado con tablas de grafo
    Cuando vinculo el concepto "Rust" hacia "PostgreSQL" con relación "USED_IN" y peso 0.9
    Y vinculo el concepto "Rust" hacia "Memory Safety" con relación "SOLVES" y peso 1.0
    Entonces al consultar el grafo para "Rust" a profundidad 1 obtengo los nodos "PostgreSQL" y "Memory Safety"
