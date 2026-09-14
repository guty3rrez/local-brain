# Guía de Contribución para Local Brain

¡Gracias por tu interés en contribuir a **Local Brain**!  
Este proyecto es de código abierto bajo la licencia [GNU AGPLv3](file:///home/guty_3rrez/Proyectos/local-brain/LICENSE) y se desarrolla de forma completamente pública y transparente.

Aceptamos contribuciones de **desarrolladores humanos y agentes de IA**. Para mantener la alta calidad técnica, la seguridad y la coherencia arquitectónica, solicitamos seguir las directrices que se describen a continuación.

---

## 🧭 ¿Cómo Puedo Colaborar?

1. **Implementando Historias del Backlog**: Consulta [`BACKLOG.md`](file:///home/guty_3rrez/Proyectos/local-brain/BACKLOG.md) para encontrar historias marcadas como `[ ] Pendiente`. Si deseas trabajar en una, coméntalo en el issue correspondiente para asignártela y evitar duplicar esfuerzos.
2. **Escribiendo Escenarios BDD y Casos de Prueba**: Redacta escenarios en Gherkin (`tests/features/`) que describan casos de uso reales de agentes de IA o agrega tests de mutación con `cargo-mutants`.
3. **Proponiendo Decisiones Arquitectónicas (ADRs)**: Abre un Issue con la plantilla de RFC / ADR para discutir elecciones tecnológicas, esquemas de bases de datos o estrategias de embeddings.
4. **Ejecutando Benchmarks**: Si tienes hardware con GPU NVIDIA, AMD o Apple Silicon, ayúdanos a correr los benchmarks de rendimiento y registrar métricas de consumo de VRAM y latencia.
5. **Reportando Errores**: Utiliza la plantilla de reporte de bugs si encuentras fallos en la recuperación, persistencia o servidor MCP.

---

## 🌿 Flujo de Trabajo con Git

### 1. Nombres de Ramas
Crea ramas de trabajo a partir de `main` siguiendo la convención:
- `feature/<nombre-breve>` para nuevas funcionalidades.
- `fix/<nombre-breve>` para correcciones de bugs.
- `refactor/<nombre-breve>` para mejoras de código sin cambio funcional.
- `docs/<nombre-breve>` para documentación o especificaciones.
- `test/<nombre-breve>` para adición de pruebas unitarias, BDD o benchmarks.
- `security/<nombre-breve>` para parches de seguridad.

### 2. Mensajes de Commit (Conventional Commits)
Utilizamos el estándar de *Conventional Commits*:
```text
<tipo>(<ámbito opcional>): <descripción concisa en imperativo>

[cuerpo opcional explicando el porqué del cambio]

[pie opcional con referencias a issues o BREAKING CHANGE]
```
Ejemplos:
- `feat(domain): implement memory retention policy and decay logic`
- `fix(postgres): handle connection pool timeout gracefully`
- `test(bdd): add gherkin scenario for contradiction detection`

---

## 🤖 Política de Contribuciones Asistidas por IA

Local Brain es un proyecto desarrollado activamente en simbiosis humano-agente (SRS §39). Reconocemos y fomentamos las contribuciones apoyadas por agentes de IA bajo las siguientes reglas:

1. **Transparencia Obligatoria**: Si un Pull Request ha sido generado total o parcialmente por un agente de IA (Antigravity, Claude Code, Cursor, Codex, etc.), debe indicarse explícitamente en la descripción del PR.
2. **Cumplimiento Estricto de [`AGENTS.md`](file:///home/guty_3rrez/Proyectos/local-brain/AGENTS.md)**: El código generado por IA debe adherirse sin excepciones a las restricciones de arquitectura hexagonal, la prohibición de debilitar tests y el respeto al *Human Review Gate*.
3. **Responsabilidad**: El autor del PR (la cuenta de GitHub que lo envía) es el responsable final del código ante la comunidad.

---

## 🧪 Checklist Previo al Envío de un Pull Request

Antes de enviar tu Pull Request, asegúrate de que todos los siguientes comandos pasen localmente en verde:

```bash
# 1. Formateo y estilo oficial
cargo fmt --all -- --check

# 2. Análisis estático (cero advertencias toleradas)
cargo clippy --workspace --all-targets -- -D warnings

# 3. Tests unitarios del workspace
cargo test --workspace --lib

# 4. Tests de integración
cargo test --workspace --test '*'

# 5. Escenarios BDD
cargo test --test cucumber

# 6. Auditoría de dependencias y vulnerabilidades
cargo audit

# 7. Verificación de licencias (compatibilidad AGPLv3)
cargo deny check licenses
```

---

## 📬 Plantilla de Pull Request

Todos los PRs deben completar los campos solicitados en [`.github/pull_request_template.md`](file:///home/guty_3rrez/Proyectos/local-brain/.github/pull_request_template.md).

¡Gracias por construir el futuro de la memoria para agentes de IA con nosotros!
