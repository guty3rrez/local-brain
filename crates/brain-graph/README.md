# brain-graph

Crate de dominio puro de Grafo de Conocimiento y Relaciones Tipadas para **Local Brain** (SRS §11, §68, Fase 5).

Conforme a la regla de oro (SRS §80) y la regla de Clean Architecture (RNF-006):
- Este crate modela los 10 tipos de relaciones conceptuales (`RELATED_TO`, `USED_IN`, `CAUSED_BY`, `SOLVES`, `CONTRADICTS`, `SUPERSEDES`, `DERIVED_FROM`, `DEPENDS_ON`, `PREFERS`, `AVOID`).
- Implementa algoritmos de validación determinista de invariantes (anti-auto-bucles, detección de ciclos en grafos acíclicos dirigidos).
- Define el puerto secundario `GraphRepository` y una implementación 100% en memoria para pruebas unitarias instantáneas sin dependencias externas.
