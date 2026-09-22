#!/usr/bin/env bash
# ==============================================================================
# Local Brain — Hook UserPromptSubmit de Claude Code
#
# Se dispara antes de que el modelo procese cada mensaje del usuario. Ejecuta
# una recuperación híbrida usando el propio prompt como consulta e inyecta
# los recuerdos más relevantes como additionalContext — recall dirigido a lo
# que el usuario está pidiendo en este turno, no un volcado genérico.
#
# Fail-open: cualquier fallo (servicios caídos, timeout, sin recuerdos
# relevantes) resulta en salida vacía y exit 0 — nunca bloquea el envío del
# prompt.
# ==============================================================================

set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./_common.sh
source "${SCRIPT_DIR}/_common.sh"

lb_load_env
[ -z "${BRAIN_BIN:-}" ] && exit 0
command -v "${BRAIN_BIN}" &>/dev/null || exit 0

HOOK_JSON="$(lb_read_stdin_json)"
CWD="$(lb_json_field "${HOOK_JSON}" '.cwd' "$(pwd)")"
PROMPT="$(lb_json_field "${HOOK_JSON}" '.prompt' "")"
PROJECT="$(lb_resolve_project "${CWD}")"

# Prompts triviales (saludos, confirmaciones cortas) no aportan una consulta
# semántica útil; evita gastar una llamada de recuperación en ellos.
[ "${#PROMPT}" -lt 8 ] && exit 0

BRAIN_ARGS=(retrieve --json --project "${PROJECT}" --limit 3)
[ -n "${LB_MIN_SCORE:-}" ] && BRAIN_ARGS+=(--min-score "${LB_MIN_SCORE}")
[ -n "${DATABASE_URL:-}" ] && BRAIN_ARGS+=(--database-url "${DATABASE_URL}")
[ -n "${EMBEDDING_URL:-}" ] && BRAIN_ARGS+=(--embedding-url "${EMBEDDING_URL}")
BRAIN_ARGS+=("${PROMPT}")

RESULT="$(timeout "${LB_TIMEOUT_SECS}" "${BRAIN_BIN}" "${BRAIN_ARGS[@]}" 2>/dev/null)" || exit 0

ITEMS_COUNT="$(lb_json_field "${RESULT}" '.items | length' "0")"
[ "${ITEMS_COUNT}" = "0" ] && exit 0

CONTEXT="$(lb_json_field "${RESULT}" '.assembled_context' "")"
[ -z "${CONTEXT}" ] && exit 0

lb_emit_context "UserPromptSubmit" "${CONTEXT}"
exit 0
