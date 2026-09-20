# Official Agent Skill — Local Brain (`local-brain`)

Official agent skill for **Local Brain**. Provides autonomous AI agents (Claude Code, Cursor, Antigravity, Codex, Windsurf, Cline, Roo Code) with a cognitive playbook to utilize the MCP server without manual user prompting.

---

## 📦 Skill Structure

- **[`SKILL.md`](SKILL.md)**: Main skill manifest with activation triggers, agent lifecycle, and scope rules.
- **[`references/mcp-tools.md`](references/mcp-tools.md)**: Technical catalog of all 11 MCP tools with JSON schemas.
- **[`references/memory-types.md`](references/memory-types.md)**: Cognitive specialization guide for all 5 memory types (`working`, `episodic`, `semantic`, `procedural`, `associative`).
- **[`references/workflow-patterns.md`](references/workflow-patterns.md)**: Practical agent interaction patterns (context ingestion, bug resolution, graph linking, session cleanup).

---

## 🚀 Installation

To install globally for Antigravity or your local agent runtime:

```bash
mkdir -p ~/.agents/skills/local-brain
cp -r .agents/skills/local-brain/* ~/.agents/skills/local-brain/
```
