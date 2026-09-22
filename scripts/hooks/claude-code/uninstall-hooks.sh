#!/usr/bin/env bash
# ==============================================================================
# Local Brain — Desinstala los hooks de recall automático de Claude Code
#
# Quita de ~/.claude/settings.json únicamente las entradas cuyo comando
# apunta a scripts/hooks/claude-code/*.sh. No toca ningún otro hook que ya
# tengas configurado. Se guarda un respaldo con timestamp antes de escribir.
#
# Uso:
#   scripts/hooks/claude-code/uninstall-hooks.sh
# ==============================================================================

set -euo pipefail

if ! command -v jq &>/dev/null; then
    echo "❌ Se requiere 'jq' para desinstalar los hooks." >&2
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SETTINGS_FILE="${CLAUDE_SETTINGS_FILE:-${HOME}/.claude/settings.json}"

if [ ! -f "${SETTINGS_FILE}" ]; then
    echo "ℹ️ No existe ${SETTINGS_FILE}, nada que desinstalar."
    exit 0
fi

BACKUP_FILE="${SETTINGS_FILE}.bak-$(date +%Y%m%d%H%M%S)"
cp "${SETTINGS_FILE}" "${BACKUP_FILE}"

CLEANED="$(jq --arg dir "${SCRIPT_DIR}/" '
    if .hooks then
        .hooks |= (with_entries(
            .value |= map(
                .hooks |= map(select((.command // "") | startswith($dir) | not))
                | select((.hooks // []) | length > 0)
            )
        ) | with_entries(select((.value | length) > 0)))
    else . end
' "${SETTINGS_FILE}")"

printf '%s\n' "${CLEANED}" > "${SETTINGS_FILE}"
rm -f "${HOME}/.config/local-brain/hooks.env"
echo "✓ Hooks de Local Brain removidos de ${SETTINGS_FILE} (respaldo previo en ${BACKUP_FILE})"
echo "  Otros hooks configurados por ti no fueron modificados."
