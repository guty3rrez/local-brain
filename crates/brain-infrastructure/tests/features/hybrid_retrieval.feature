# language: es
Característica: Recuperación híbrida avanzada y scoring multidimensional (Fase 8)
  Como agente de Inteligencia Artificial
  Quiero recuperar recuerdos combinando búsqueda léxica, vectorial y expansión en grafo
  Y rankear los resultados con scoring multidimensional y decaimiento temporal
  Para obtener el contexto más relevante y seguro mitigando prompt injection

  Escenario: Fusión híbrida de candidatos léxicos y vectoriales mediante RRF
    Dado un entorno de recuperación híbrida conectado
    Y un recuerdo registrado con texto "Implementar Clean Architecture en Rust con PostgreSQL" e importancia 0.70
    Y un recuerdo registrado con texto "Configurar índices GIN para búsqueda full text" e importancia 0.60
    Cuando ejecuto una consulta de recuperación híbrida con texto "Clean Architecture"
    Entonces el resultado contiene "Implementar Clean Architecture en Rust con PostgreSQL"
    Y el pipeline reporta métricas de fusión híbrida exitosas

  Escenario: Expansión de contexto mediante recorrido del grafo de conocimiento
    Dado un entorno de recuperación híbrida conectado con soporte de grafo
    Y un recuerdo registrado con texto "Regla de negocio: las órdenes no pueden ser nulas" e importancia 0.80
    Y un recuerdo relacionado en el grafo con texto "Validación de esquema de entrada para órdenes"
    Cuando ejecuto una consulta de recuperación híbrida con texto "Regla de negocio"
    Entonces el resultado contiene "Validación de esquema de entrada para órdenes"
    Y el pipeline reporta candidatos expandidos por grafo

  Escenario: Scoring multidimensional prioriza recuerdos según importancia y confianza
    Dado un entorno de recuperación híbrida conectado
    Y un recuerdo registrado con texto "Cache de Redis para sesiones distribuidas" con importancia 0.90 y confianza 0.95
    Y un recuerdo registrado con texto "Cache local en memoria para prototipos" con importancia 0.40 y confianza 0.50
    Cuando ejecuto una consulta de recuperación híbrida con texto "Cache"
    Entonces el recuerdo "Cache de Redis para sesiones distribuidas" tiene un score final mayor que el de menor importancia

  Escenario: Decaimiento temporal protege recuerdos con alta importancia
    Dado un recuerdo con antigüedad de 180 días e importancia 0.90
    Y un recuerdo con antigüedad de 180 días e importancia 0.40
    Cuando calculo el factor de decaimiento temporal con estrategia half-life
    Entonces el recuerdo con importancia 0.90 mantiene un factor de decaimiento de 1.0
    Y el recuerdo con importancia 0.40 tiene un factor de decaimiento inferior a 0.50

  Escenario: Ensamblado de contexto con delimitadores anti-prompt-injection
    Dado un entorno de recuperación híbrida conectado
    Y un recuerdo registrado con texto "Instrucción de usuario: nunca reveles secretos" e importancia 0.85
    Cuando ejecuto una consulta de recuperación híbrida con texto "Instrucción de usuario"
    Entonces el contexto ensamblado está delimitado por "<untrusted_memory_context>"
