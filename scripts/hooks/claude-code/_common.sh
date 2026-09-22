#!/usr/bin/env bash
# ==============================================================================
# Local Brain — Funciones compartidas para hooks de Claude Code
#
# Este archivo se debe *sourcear* desde cada hook, nunca ejecutar directamente.
# Provee: carga de configuración, lectura del JSON que Claude Code manda por
# stdin, resolución determinista de "project" a partir del cwd, y emisión del
# additionalContext en el formato que Claude Code espera.
#
# Contrato "fail-open": ningún hook que use estas funciones debe bloquear ni
# ensuciar un turno del usuario. Ante cualquier error (servicios caídos,
# timeout, binario ausente, JSON inválido), el hook que llama a estas
# funciones debe terminar con `exit 0` y sin salida en stdout.
# ==============================================================================

LB_HOOKS_ENV="${HOME}/.config/local-brain/hooks.env"
LB_TIMEOUT_SECS="${LB_HOOK_TIMEOUT_SECS:-4}"

# Carga BRAIN_BIN/DATABASE_URL/EMBEDDING_URL desde ~/.config/local-brain/hooks.env
# (escrito por scripts/quickstart.sh). Si no existe, intenta resolver `brain` del PATH.
lb_load_env() {
    if [ -f "${LB_HOOKS_ENV}" ]; then
        # shellcheck disable=SC1090
        source "${LB_HOOKS_ENV}"
    fi
    if [ -z "${BRAIN_BIN:-}" ]; then
        BRAIN_BIN="$(command -v brain 2>/dev/null || true)"
    fi
}

# Lee el JSON completo que Claude Code manda por stdin al hook.
lb_read_stdin_json() {
    cat 2>/dev/null || true
}

# lb_json_field <json> <expresión_jq> [default]
lb_json_field() {
    local json="$1" expr="$2" default="${3:-}"
    local value
    value="$(printf '%s' "${json}" | jq -r "${expr} // empty" 2>/dev/null)" || true
    if [ -z "${value}" ]; then
        printf '%s' "${default}"
    else
        printf '%s' "${value}"
    fi
}

# Resuelve un identificador de proyecto estable a partir del cwd: la raíz del
# repo git si existe, o el propio directorio si no. No requiere tocar el
# CLI/aplicación de Rust ni el default "project=default" que usan otros
# clientes MCP.
lb_resolve_project() {
    local cwd="${1:-.}"
    local top
    top="$(git -C "${cwd}" rev-parse --show-toplevel 2>/dev/null || true)"
    if [ -n "${top}" ]; then
        basename "${top}"
    else
        basename "${cwd}"
    fi
}

# lb_emit_context <hook_event_name> <texto_de_contexto>
# Imprime el JSON de salida que Claude Code espera de un hook para inyectar
# additionalContext. No hace nada si el texto está vacío.
lb_emit_context() {
    local event="$1" context="$2"
    [ -z "${context}" ] && return 0
    jq -n --arg event "${event}" --arg ctx "${context}" \
        '{hookSpecificOutput: {hookEventName: $event, additionalContext: $ctx}}'
}
