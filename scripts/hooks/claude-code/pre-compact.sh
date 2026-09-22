#!/usr/bin/env bash
# ==============================================================================
# Local Brain — Hook PreCompact de Claude Code
#
# Se dispara justo antes de que el contexto se compacte (se pierde detalle).
# No escribe memoria: decidir qué vale la pena persistir es un juicio
# semántico que le corresponde al modelo, no a un script determinista. Este
# hook solo se lo recuerda mediante additionalContext, reforzando la Fase 1
# del skill local-brain (brain_remember / brain_learn) antes de que ese
# detalle se pierda.
# ==============================================================================

set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./_common.sh
source "${SCRIPT_DIR}/_common.sh"

REMINDER='Antes de compactar el contexto: si en esta sesión se tomó alguna decisión arquitectónica, se resolvió un bug no trivial, o se estableció una convención que valga la pena recordar entre sesiones, guárdala ahora con brain_remember/brain_learn (skill local-brain, Fase 1) antes de perder el detalle en la compactación.'

lb_emit_context "PreCompact" "${REMINDER}"
exit 0
