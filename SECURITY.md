# Política de Seguridad y Modelo de Amenazas de Local Brain

## 🔒 Política de Divulgación Responsable

La seguridad de los datos y de la memoria de los usuarios es crítica. Si descubres una vulnerabilidad de seguridad en Local Brain, por favor **NO abras un issue público**.

En su lugar, reporta la vulnerabilidad de manera privada creando un [Security Advisory en GitHub](https://github.com/local-brain/local-brain/security/advisories/new) o contactando directamente a los mantenedores del proyecto.

Por favor incluye:
- Descripción técnica de la vulnerabilidad.
- Pasos detallados o prueba de concepto (PoC) para reproducirla.
- Impacto potencial en la confidencialidad, integridad o disponibilidad de la memoria.

Nos comprometemos a acusar recibo en un plazo de 48 horas y coordinar un parche antes de cualquier divulgación pública.

---

## 🛡️ Modelo de Amenazas (SRS §62)

Local Brain implementa controles para mitigar el siguiente catálogo de amenazas específicas para agentes de IA e infraestructura de memoria:

| ID | Amenaza | Descripción | Control / Mitigación en Local Brain |
| :--- | :--- | :--- | :--- |
| **T-001** | **Prompt Injection** | Memorias que contienen texto malicioso intentando sobreescribir instrucciones del agente al ser recuperadas. | El contenido recuperado se encapsula estrictamente como **datos delimitados**, nunca como instrucciones de sistema (SRS §32). |
| **T-002** | **Malicious MCP Client** | Un cliente o agente no autorizado que intenta emitir comandos privilegiados. | Separación de permisos MCP (READ, WRITE, MODIFY, DELETE, ADMIN) y control estricto de transporte (SRS §31). |
| **T-003** | **Memory Poisoning** | Agentes que registran recuerdos falsos para sesgar el comportamiento futuro. | Trazabilidad de procedencia (*provenance*), cálculo de confianza probabilístico y estado de conocimiento candidato (SRS §15, §63). |
| **T-004** | **Unauthorized Memory Deletion** | Borrado accidental o malicioso de la base de conocimiento acumulada. | Soft deletes por defecto, logs de auditoría inmutables y confirmación explícita para operaciones destructivas (SRS §34). |
| **T-005** | **Credential Leakage** | Filtración involuntaria de tokens, contraseñas o claves API hacia la memoria persistente. | Filtros de expresiones regulares de escaneo de secretos antes de persistir cualquier contenido de memoria. |
| **T-006** | **Malicious Dependency** | Crate comprometida introducida en la cadena de compilación. | Bloqueo estricto con `cargo audit` y `cargo deny check` en el pipeline de CI. |
| **T-007** | **Database Compromise** | Acceso no autorizado o corrupción de PostgreSQL / pgvector. | Principio de mínimo privilegio en conexiones DB, soporte para cifrado en reposo y backups periódicos (SRS §25, §52). |
| **T-008** | **Supply-chain Attack** | Modificación maliciosa de binarios o modelos preentrenados. | Verificación de hashes SHA-256 de pesos de modelos locales (`.gguf`) y builds reproducibles. |
| **T-009** | **Agent Hallucination** | Afirmaciones inventadas por el LLM asumidas como hechos. | Separación estricta entre **Observación** y **Creencia**; el LLM no tiene autoridad directa para crear hechos consolidados (SRS §4.5, §59). |
| **T-010** | **Knowledge Poisoning** | Alteración sutil y progresiva del grafo de conocimiento. | Versionado inmutable de entidades de conocimiento con historial de cambios e identificación del autor (SRS §19). |
