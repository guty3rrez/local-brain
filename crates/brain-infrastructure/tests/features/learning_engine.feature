# language: es
Característica: Motor de aprendizaje y conocimiento candidato (Fase 6)
  Como agente de IA o desarrollador
  Quiero que el sistema distinga observaciones puntuales de creencias generalizadas
  Y calcule rigurosamente la confianza basada en evidencia empírica
  Para evitar alucinaciones y explicar por qué el sistema cree determinadas conclusiones

  Escenario: Distinguir observación de creencia y madurar a candidato con evidencias
    Dado un repositorio PostgreSQL conectado con tablas de aprendizaje
    Cuando registro una observación factual "El deployment tardó 4 minutos" para el dominio "devops"
    Entonces la observación se almacena en etapa "observation" con confianza menor a 0.40
    Cuando propongo una creencia candidata "Este stack tiene deployments lentos" con evidencia "El deployment tardó 4 minutos"
    Entonces la creencia se registra en etapa "candidate"
    Cuando agrego la evidencia de herramienta "Medición de latencia de pipeline: 240s promedio" de soporte
    Y agrego la evidencia de herramienta "Timeout registrado en el step de empaquetado Docker" de soporte
    Entonces la creencia madura a etapa "validated" con confianza superior a 0.65

  Escenario: Mitigación de alucinación del agente sin evidencias empíricas
    Dado un repositorio PostgreSQL conectado con tablas de aprendizaje
    Cuando un agente afirma "Tecnología X es 10 veces más rápida" sin evidencias ni validación humana
    Entonces la afirmación permanece en etapa "candidate"
    Y la confianza asignada no supera el umbral tentativo de 0.40
    Y no puede ascender a "validated" sin respaldo empírico

  Escenario: Agent asks brain to explain why .NET is preferred
    Dado un repositorio PostgreSQL conectado con tablas de aprendizaje
    Cuando registro el conocimiento candidato ".NET es preferido para backends empresariales complejos" para el dominio "tech-stack"
    Y agrego la evidencia "Experience #182: Manejo de reglas de negocio y múltiples roles" de soporte
    Y agrego la evidencia "Experience #201: Concurrencia masiva estable sin fugas de memoria" de soporte
    Y el conocimiento es validado formalmente por un humano
    Cuando solicito la explicación para ".NET es preferido"
    Entonces la explicación contiene la conclusión ".NET es preferido para backends empresariales complejos"
    Y la explicación reporta confianza superior a 0.80
    Y la explicación detalla las evidencias "Experience #182" y "Experience #201"
    Y la explicación indica validación humana aprobada
