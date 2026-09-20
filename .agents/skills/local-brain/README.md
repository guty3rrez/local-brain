# Skill de Agente — Local Brain (`local-brain`)

Skill oficial de integración agéntica para **Local Brain**. Proporciona a los agentes de IA (Claude Code, Cursor, Antigravity, Codex, Windsurf, Cline, Roo Code) el playbook cognitivo necesario para aprovechar el servidor MCP sin necesidad de que el usuario recuerde los nombres exactos o la sintaxis de las herramientas.

---

## 📦 Estructura de la Skill

- **[`SKILL.md`](file:///home/guty_3rrez/Proyectos/local-brain/.agents/skills/local-brain/SKILL.md)**: Manifiesto principal de la skill con disparadores de activación, ciclo de vida agéntico y reglas de scope.
- **[`references/mcp-tools.md`](file:///home/guty_3rrez/Proyectos/local-brain/.agents/skills/local-brain/references/mcp-tools.md)**: Catálogo técnico de las 11 herramientas MCP con sus esquemas JSON.
- **[`references/memory-types.md`](file:///home/guty_3rrez/Proyectos/local-brain/.agents/skills/local-brain/references/memory-types.md)**: Guía de especialización cognitiva para los 5 tipos de memoria (Working, Episodic, Semantic, Procedural, Associative).
- **[`references/workflow-patterns.md`](file:///home/guty_3rrez/Proyectos/local-brain/.agents/skills/local-brain/references/workflow-patterns.md)**: Ejemplos de flujos de trabajo agénticos (inicio de sesión, registro de experiencia, resolución de conflictos, grafo).

---

## 🚀 Instalación en el Sistema del Usuario

Para que cualquier agente en tu máquina pueda usar esta skill automáticamente:

```bash
# Copiar la carpeta de la skill al directorio global de skills de agentes
mkdir -p ~/.agents/skills/local-brain
cp -r .agents/skills/local-brain/* ~/.agents/skills/local-brain/
```
