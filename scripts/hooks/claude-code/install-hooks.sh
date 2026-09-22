#!/usr/bin/env bash
# ==============================================================================
# Local Brain — Instala los hooks de recall automático en Claude Code
#
# Escribe ~/.config/local-brain/hooks.env (BRAIN_BIN/DATABASE_URL/EMBEDDING_URL)
# y registra los hooks SessionStart / UserPromptSubmit / PreCompact /
# SessionEnd en ~/.claude/settings.json. El merge es idempotente (reejecutar
# este script no duplica entradas) y no toca ningún otro hook que ya tengas
# configurado. Se guarda un respaldo con timestamp antes de escribir.
#
# Uso:
#   scripts/hooks/claude-code/install-hooks.sh
#
# Variables de entorno opcionales:
#   BRAIN_BIN               ruta al binario `brain` (por defecto, el del PATH)
#   DATABASE_URL             URL de PostgreSQL (si no se define, `brain` usa
#                             su propio default de docker-compose)
#   EMBEDDING_URL             URL del servidor de embeddings (idem)
#   CLAUDE_SETTINGS_FILE       ruta alterna a settings.json (por defecto
#                               ~/.claude/settings.json)
# ==============================================================================

set -euo pipefail

if ! command -v jq &>/dev/null; then
    echo "❌ Se requiere 'jq' para instalar los hooks. Instálalo e intenta de nuevo." >&2
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

BRAIN_BIN="${BRAIN_BIN:-$(command -v brain || true)}"
if [ -z "${BRAIN_BIN}" ]; then
    echo "❌ No se encontró el binario 'brain'. Define BRAIN_BIN=/ruta/a/brain e intenta de nuevo." >&2
    exit 1
fi

CONFIG_DIR="${HOME}/.config/local-brain"
HOOKS_ENV="${CONFIG_DIR}/hooks.env"
SETTINGS_FILE="${CLAUDE_SETTINGS_FILE:-${HOME}/.claude/settings.json}"

mkdir -p "${CONFIG_DIR}"
{
    echo "BRAIN_BIN=\"${BRAIN_BIN}\""
    [ -n "${DATABASE_URL:-}" ] && echo "DATABASE_URL=\"${DATABASE_URL}\""
    [ -n "${EMBEDDING_URL:-}" ] && echo "EMBEDDING_URL=\"${EMBEDDING_URL}\""
} > "${HOOKS_ENV}"
echo "✓ Configuración de hooks escrita en ${HOOKS_ENV}"

mkdir -p "$(dirname "${SETTINGS_FILE}")"
[ -f "${SETTINGS_FILE}" ] || echo '{}' > "${SETTINGS_FILE}"

NEW_HOOKS_JSON="$(jq -n \
    --arg session_start "${SCRIPT_DIR}/session-start.sh" \
    --arg user_prompt "${SCRIPT_DIR}/user-prompt-submit.sh" \
    --arg pre_compact "${SCRIPT_DIR}/pre-compact.sh" \
    --arg session_end "${SCRIPT_DIR}/session-end.sh" \
    '{
        SessionStart: [{hooks: [{type: "command", command: $session_start, timeout: 6}]}],
        UserPromptSubmit: [{hooks: [{type: "command", command: $user_prompt, timeout: 6}]}],
        PreCompact: [{hooks: [{type: "command", command: $pre_compact, timeout: 3}]}],
        SessionEnd: [{hooks: [{type: "command", command: $session_end, timeout: 6}]}]
    }')"

BACKUP_FILE="${SETTINGS_FILE}.bak-$(date +%Y%m%d%H%M%S)"
cp "${SETTINGS_FILE}" "${BACKUP_FILE}"

MERGED="$(jq --argjson new "${NEW_HOOKS_JSON}" '
    .hooks = (.hooks // {})
    | reduce ($new | to_entries[]) as $kv (.;
        .hooks[$kv.key] = (
            (.hooks[$kv.key] // []) as $existing
            | if ($existing | any(.hooks[]?.command == $kv.value[0].hooks[0].command))
              then $existing
              else $existing + $kv.value
              end
        )
      )
' "${SETTINGS_FILE}")"

printf '%s\n' "${MERGED}" > "${SETTINGS_FILE}"
echo "✓ Hooks registrados en ${SETTINGS_FILE} (respaldo previo en ${BACKUP_FILE})"
echo "  SessionStart, UserPromptSubmit, PreCompact, SessionEnd -> ${SCRIPT_DIR}/"
echo ""
echo "  Para revertir: ${SCRIPT_DIR}/uninstall-hooks.sh"
