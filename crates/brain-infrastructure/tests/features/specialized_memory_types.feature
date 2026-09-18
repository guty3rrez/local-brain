# language: es
Característica: Especialización de tipos de memoria cognitiva
  Como sistema cognitivo Local Brain
  Quiero soportar tipos especializados de memoria (episódica, de trabajo, procedimental, asociativa, semántica)
  Para respetar los requerimientos de estructura, TTL e invariantes de cada tipo

  Escenario: Persistir y recuperar recuerdo episódico con contexto, acción y resultado
    Dado un repositorio PostgreSQL conectado y con migraciones aplicadas
    Cuando guardo un recuerdo episódico con contexto "Refactorización" y acción "Correr migración" y resultado "Éxito" para el proyecto "local-brain"
    Entonces puedo recuperar el recuerdo episódico por su ID
    Y el contexto episódico es "Refactorización", la acción es "Correr migración" y el resultado es "Éxito"

  Escenario: Recuerdo de trabajo con expiración por sesión
    Dado un repositorio PostgreSQL conectado y con migraciones aplicadas
    Cuando guardo un recuerdo de trabajo para la sesión "ses-bdd-1" con contenido "Variable temporal" para el proyecto "local-brain"
    Entonces encuentro 1 recuerdo activo en la sesión "ses-bdd-1"
    Cuando expiro la sesión "ses-bdd-1"
    Entonces encuentro 0 recuerdos activos en la sesión "ses-bdd-1"

  Escenario: Recuerdo procedimental con pasos ordenados y versión
    Dado un repositorio PostgreSQL conectado y con migraciones aplicadas
    Cuando guardo un recuerdo procedimental para la tarea "Build" con 2 pasos para el proyecto "local-brain"
    Entonces puedo recuperar el recuerdo procedimental por su ID
    Y el procedimiento tiene 2 pasos y la versión es 1

  Escenario: Recuerdo asociativo entre conceptos
    Dado un repositorio PostgreSQL conectado y con migraciones aplicadas
    Cuando guardo una asociación desde "PostgreSQL" hacia "pgvector" con predicado "usa_extension" para el proyecto "local-brain"
    Entonces al buscar asociaciones para "PostgreSQL" encuentro relación con "pgvector"

  Escenario: Rechazo de memoria semántica con alta confianza sin evidencias
    Dado que intento crear una memoria semántica con confianza 0.95 y sin evidencias
    Entonces la creación es rechazada por regla de invariante
