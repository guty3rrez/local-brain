# language: es
Característica: Búsqueda semántica vectorial en PostgreSQL con pgvector
  Como agente de Inteligencia Artificial
  Quiero buscar recuerdos utilizando similitud semántica en lenguaje natural
  Para encontrar conocimiento relevante sin depender de coincidencias exactas

  Escenario: Recuperación semántica de recuerdos indexados con vectores de 768 dimensiones
    Dado un repositorio PostgreSQL conectado y con migraciones aplicadas
    Y un recuerdo indexado con embedding de 768 dimensiones y contenido "Arquitectura Hexagonal en Rust desacopla el dominio"
    Cuando realizo una búsqueda vectorial con un vector de consulta similar
    Entonces el resultado más similar contiene "Arquitectura Hexagonal en Rust desacopla el dominio"
    Y la similitud coseno calculada es superior a 0.95
