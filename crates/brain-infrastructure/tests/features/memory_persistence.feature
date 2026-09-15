# language: es
Característica: Persistencia de memoria cognitiva en PostgreSQL
  Como agente de Inteligencia Artificial
  Quiero almacenar recuerdos en una base de datos PostgreSQL
  Para recuperarlos de forma confiable entre distintas sesiones de trabajo

  Escenario: Persistir y recuperar memoria por ID
    Dado un repositorio PostgreSQL conectado y con migraciones aplicadas
    Cuando guardo un nuevo recuerdo episódico con contenido "Decisión de arquitectura: Hexagonal en Rust" para el proyecto "local-brain"
    Entonces puedo recuperar el recuerdo utilizando su ID
    Y el contenido recuperado coincide exactamente con "Decisión de arquitectura: Hexagonal en Rust"
    Y el estado del recuerdo es "active"
