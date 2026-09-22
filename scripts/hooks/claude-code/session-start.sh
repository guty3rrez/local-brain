#!/usr/bin/env bash
# ==============================================================================
# Local Brain — Hook SessionStart de Claude Code
#
# Se dispara al arrancar, reanudar o limpiar una sesión (`startup`, `resume`,
# `clear`). Ejecuta una recuperación híbrida genérica del proyecto actual e
# inyecta el resultado como additionalContext, reemplazando la Fase 0
# ("Context Ingestion") manual del skill local-brain.
#
# Fail-open: cualquier fallo (servicios caídos, timeout, sin recuerdos)
# resulta en salida vacía y exit 0 — nunca bloquea el arranque de la sesión.
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
PROJECT="$(lb_resolve_project "${CWD}")"

BRAIN_ARGS=(retrieve --json --project "${PROJECT}" --limit 5)
[ -n "${DATABASE_URL:-}" ] && BRAIN_ARGS+=(--database-url "${DATABASE_URL}")
[ -n "${EMBEDDING_URL:-}" ] && BRAIN_ARGS+=(--embedding-url "${EMBEDDING_URL}")
BRAIN_ARGS+=("arquitectura, convenciones y decisiones importantes de este proyecto")

RESULT="$(timeout "${LB_TIMEOUT_SECS}" "${BRAIN_BIN}" "${BRAIN_ARGS[@]}" 2>/dev/null)" || exit 0

ITEMS_COUNT="$(lb_json_field "${RESULT}" '.items | length' "0")"
[ "${ITEMS_COUNT}" = "0" ] && exit 0

CONTEXT="$(lb_json_field "${RESULT}" '.assembled_context' "")"
[ -z "${CONTEXT}" ] && exit 0

lb_emit_context "SessionStart" "${CONTEXT}"
exit 0
