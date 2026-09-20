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

`./scripts/quickstart.sh` does this for you automatically when it detects the Claude Code CLI (`claude`) — see [`docs/QUICKSTART.md`](../../../docs/QUICKSTART.md). Use the manual steps below only if you're not using the wizard, or for a different agent runtime.

### Claude Code
Claude Code discovers skills through `~/.claude/skills/<name>`. Copy the skill to `~/.agents/skills` (the shared source of truth other tools also read from) and symlink it in:

```bash
mkdir -p ~/.agents/skills/local-brain
cp -r .agents/skills/local-brain/* ~/.agents/skills/local-brain/
mkdir -p ~/.claude/skills
ln -sf ~/.agents/skills/local-brain ~/.claude/skills/local-brain
```

### Antigravity / other local agent runtimes
```bash
mkdir -p ~/.agents/skills/local-brain
cp -r .agents/skills/local-brain/* ~/.agents/skills/local-brain/
```
