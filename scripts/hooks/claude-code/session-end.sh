#!/usr/bin/env bash
# ==============================================================================
# Local Brain — Hook SessionEnd de Claude Code
#
# Se dispara al cerrar una sesión de Claude Code. Reutiliza el session_id que
# Claude Code ya manda en el JSON del hook para expirar/archivar la memoria de
# trabajo (`memory_type: working`) de esa sesión vía `brain session-end`, sin
# que el modelo tenga que acordarse de cerrarla manualmente (Fase 3 del skill
# local-brain).
#
# Best-effort: si no hay recuerdos de trabajo bajo ese session_id, o los
# servicios no están disponibles, no hace nada visible. Nunca falla el cierre
# de la sesión del usuario.
# ==============================================================================

set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./_common.sh
source "${SCRIPT_DIR}/_common.sh"

lb_load_env
[ -z "${BRAIN_BIN:-}" ] && exit 0
command -v "${BRAIN_BIN}" &>/dev/null || exit 0

HOOK_JSON="$(lb_read_stdin_json)"
SESSION_ID="$(lb_json_field "${HOOK_JSON}" '.session_id' "")"
[ -z "${SESSION_ID}" ] && exit 0

BRAIN_ARGS=(session-end "${SESSION_ID}")
[ -n "${DATABASE_URL:-}" ] && BRAIN_ARGS+=(--database-url "${DATABASE_URL}")
[ -n "${EMBEDDING_URL:-}" ] && BRAIN_ARGS+=(--embedding-url "${EMBEDDING_URL}")

timeout "${LB_TIMEOUT_SECS}" "${BRAIN_BIN}" "${BRAIN_ARGS[@]}" &>/dev/null || true
exit 0
