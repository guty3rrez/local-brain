# language: es
Característica: Motor de consolidación, reflexión y detección de contradicciones (Fase 7)
  Como agente de IA o desarrollador
  Quiero que Local Brain consolide experiencias agrupándolas y detectando patrones emergentes
  Y detecte contradicciones para marcar el estado CONFLICT y enlazarlas en el grafo
  Para mantener una memoria coherente, auditable y con resolución humana explícita

  Escenario: Reflexión periódica agrupa experiencias y produce conocimiento candidato con evidencias
    Dado un repositorio PostgreSQL conectado con soporte de consolidación
    Cuando registro una memoria episódica con contenido "Uso de índices B-Tree en PostgreSQL acelera consultas de filtrado" para el proyecto "bdd-consolidation"
    Y registro una memoria episódica con contenido "Uso de índices B-Tree en PostgreSQL optimiza la búsqueda de rangos" para el proyecto "bdd-consolidation"
    Y ejecuto el proceso de reflexión para el proyecto "bdd-consolidation"
    Entonces el reporte de reflexión contiene al menos 1 cluster formado
    Y se genera al menos 1 hipótesis candidata con confianza no superior a 0.40
    Y la hipótesis candidata referencia las memorias como evidencias

  Escenario: Detección de contradicción marca estado CONFLICT y enlaza en el grafo
    Dado un repositorio PostgreSQL conectado con soporte de consolidación
    Cuando registro una memoria de preferencia "Prefiero Supabase para el backend" para el proyecto "conflict-proj"
    Y registro una memoria de preferencia "Prefiero .NET para el backend" para el proyecto "conflict-proj"
    Y ejecuto el proceso de reflexión para el proyecto "conflict-proj"
    Entonces se detecta al menos 1 contradicción
    Y las memorias involucradas pasan a estado "Conflict"
    Y existe una relación "CONTRADICTS" entre ambas memorias en el grafo

  Escenario: Resolución de contradicción restaura memorias a estado Active
    Dado una contradicción registrada entre dos memorias en el repositorio PostgreSQL
    Cuando el usuario resuelve la contradicción con el contexto "Se adopta .NET para microservicios y Supabase para prototipos"
    Entonces el estado del conflicto cambia a "Resolved"
    Y ambas memorias vuelven a estado "Active"
