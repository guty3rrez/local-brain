#!/usr/bin/env bash
# ==============================================================================
# Local Brain — Quickstart & Automated Onboarding Wizard
#
# Configura y verifica todo el stack de Local Brain en menos de 2 minutos:
#   1. Valida Docker y Docker Compose
#   2. Descarga el modelo de embeddings e inicia servicios locales
#   3. Ejecuta migraciones de base de datos (brain init)
#   4. Diagnostica la salud del sistema (brain doctor)
#   5. Si detecta Claude Code, instala el skill del agente y registra el MCP
#      automáticamente (scope global, disponible en todos tus repos)
#   6. Imprime snippets de configuración MCP para el resto de clientes
# ==============================================================================

set -euo pipefail

BOLD='\033[1m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo -e "${BOLD}${BLUE}🧠 Bienvenido a Local Brain v1.0 — Asistente de Configuración Rápida${NC}\n"

# 1. Verificar Docker
echo -e "${BOLD}[1/6] Verificando dependencias del sistema...${NC}"
if ! command -v docker &>/dev/null; then
    echo -e "${RED}❌ Docker no está instalado. Por favor instálalo desde https://docs.docker.com/get-docker/${NC}"
    exit 1
fi

if ! docker compose version &>/dev/null; then
    echo -e "${RED}❌ Docker Compose no está disponible. Requiere Docker Compose v2+.${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Docker y Docker Compose detectados.${NC}"

# 2. Descargar modelo de embeddings si no existe
MODEL_DIR="${ROOT_DIR}/models"
MODEL_FILE="${MODEL_DIR}/nomic-embed-text-v1.5.Q8_0.gguf"
MODEL_URL="https://huggingface.co/nomic-ai/nomic-embed-text-v1.5-GGUF/resolve/main/nomic-embed-text-v1.5.Q8_0.gguf"

mkdir -p "${MODEL_DIR}"
if [ ! -f "${MODEL_FILE}" ]; then
    echo -e "\n${BOLD}[2/6] Descargando modelo de embeddings nomic-embed-text-v1.5 (~140 MB)...${NC}"
    if command -v curl &>/dev/null; then
        curl -L --progress-bar -o "${MODEL_FILE}" "${MODEL_URL}"
    elif command -v wget &>/dev/null; then
        wget -q --show-progress -O "${MODEL_FILE}" "${MODEL_URL}"
    fi
else
    echo -e "${GREEN}✓ Modelo de embeddings verificado en: ${MODEL_FILE}${NC}"
fi

# 3. Iniciar servicios en segundo plano
echo -e "\n${BOLD}[3/6] Iniciando PostgreSQL 17 (pgvector) y llama.cpp server...${NC}"
cd "${ROOT_DIR}"
docker compose up -d

echo -e "⏳ Esperando a que PostgreSQL esté listo..."
until docker exec local-brain-postgres pg_isready -U localbrain -d local_brain &>/dev/null; do
    sleep 1
done
echo -e "${GREEN}✓ PostgreSQL 17 + pgvector listo en puerto 5433.${NC}"

echo -e "⏳ Esperando a que el servidor de embeddings esté listo..."
RETRY=0
MAX_RETRIES=20
until curl -s "http://127.0.0.1:8081/health" &>/dev/null || [ $RETRY -ge $MAX_RETRIES ]; do
    sleep 1
    RETRY=$((RETRY + 1))
done

if [ $RETRY -lt $MAX_RETRIES ]; then
    echo -e "${GREEN}✓ llama.cpp server listo en puerto 8081 (768 dimensiones).${NC}"
else
    echo -e "${YELLOW}⚠️ llama.cpp sigue inicializándose. Continuará cargando en segundo plano.${NC}"
fi

# 4. Localizar o compilar binario brain
echo -e "\n${BOLD}[4/6] Verificando binario de Local Brain (brain CLI)...${NC}"
BRAIN_BIN=""
if command -v brain &>/dev/null; then
    BRAIN_BIN="$(command -v brain)"
elif [ -f "${ROOT_DIR}/target/release/brain" ]; then
    BRAIN_BIN="${ROOT_DIR}/target/release/brain"
elif [ -f "${ROOT_DIR}/target/debug/brain" ]; then
    BRAIN_BIN="${ROOT_DIR}/target/debug/brain"
elif command -v cargo &>/dev/null; then
    echo -e "${BLUE}Compilando la CLI de Local Brain localmente...${NC}"
    cargo build --release --bin brain
    BRAIN_BIN="${ROOT_DIR}/target/release/brain"
fi

BRAIN_FOUND=false
if [ -n "${BRAIN_BIN}" ]; then
    echo -e "${GREEN}✓ Binario detectado en: ${BRAIN_BIN}${NC}"
    echo -e "\n🔄 Ejecutando migraciones de base de datos..."
    "${BRAIN_BIN}" init
    echo -e "\n🩺 Diagnóstico del sistema:"
    "${BRAIN_BIN}" doctor || true
    BRAIN_FOUND=true
else
    echo -e "${YELLOW}ℹ️ No se detectó binario 'brain'. Puedes descargarlo de GitHub Releases o compilarlo con:${NC}"
    echo -e "   ${BOLD}cargo build --release --bin brain${NC}"
    BRAIN_BIN="brain"
fi

# 5. Instalar skill de agente + registrar MCP en Claude Code (si está disponible)
echo -e "\n${BOLD}[5/6] Integrando con Claude Code (skill + MCP)...${NC}"
if command -v claude &>/dev/null && [ "${BRAIN_FOUND}" = true ]; then
    SKILL_SRC="${ROOT_DIR}/.agents/skills/local-brain"
    SKILL_DST="${HOME}/.agents/skills/local-brain"
    mkdir -p "${SKILL_DST}"
    cp -r "${SKILL_SRC}"/* "${SKILL_DST}/"
    mkdir -p "${HOME}/.claude/skills"
    ln -sf "${SKILL_DST}" "${HOME}/.claude/skills/local-brain"
    echo -e "${GREEN}✓ Skill instalado en ~/.claude/skills/local-brain${NC}"

    if claude mcp get local-brain &>/dev/null; then
        echo -e "${GREEN}✓ MCP 'local-brain' ya estaba registrado en Claude Code.${NC}"
    elif claude mcp add local-brain "${BRAIN_BIN}" -s user -- \
        --database-url postgres://localbrain:localbrain_secret@localhost:5433/local_brain \
        --embedding-url http://127.0.0.1:8081/embedding \
        mcp &>/dev/null; then
        echo -e "${GREEN}✓ MCP 'local-brain' registrado en Claude Code (scope 'user', disponible en todos tus repos).${NC}"
    else
        echo -e "${YELLOW}⚠️ No se pudo registrar el MCP automáticamente. Usa el comando manual en la sección de abajo.${NC}"
    fi
else
    echo -e "${YELLOW}ℹ️ Claude Code CLI ('claude') no detectado, o falta el binario 'brain'. Omitiendo integración automática — usa los pasos manuales de abajo.${NC}"
fi

# 6. Imprimir configuraciones MCP para el resto de agentes
echo -e "\n${BOLD}${GREEN}==============================================================================${NC}"
echo -e "${BOLD}${GREEN}✨ ¡Local Brain está listo para operar!${NC}"
echo -e "${BOLD}${GREEN}==============================================================================${NC}\n"

echo -e "${BOLD}Para el resto de tus agentes de IA, agrega la siguiente configuración MCP:${NC}\n"

echo -e "${BOLD}${BLUE}1. Claude Desktop${NC} (en su archivo de configuración claude_desktop_config.json):"
cat << EOF
{
  "mcpServers": {
    "local-brain": {
      "command": "${BRAIN_BIN}",
      "args": [
        "--database-url", "postgres://localbrain:localbrain_secret@localhost:5433/local_brain",
        "--embedding-url", "http://127.0.0.1:8081/embedding",
        "mcp"
      ]
    }
  }
}
EOF

echo -e "\n${BOLD}${BLUE}2. Claude Code CLI${NC} (si el paso [5/6] no pudo hacerlo por ti — el flag ${BOLD}-s user${NC}${BLUE} lo deja disponible en todos tus repos):"
echo -e "${BOLD}claude mcp add local-brain ${BRAIN_BIN} -s user -- --database-url postgres://localbrain:localbrain_secret@localhost:5433/local_brain --embedding-url http://127.0.0.1:8081/embedding mcp${NC}\n"

echo -e "${BOLD}${BLUE}3. Cursor / Windsurf${NC} (en .cursor/mcp.json o mcp_config.json):"
cat << EOF
{
  "mcpServers": {
    "local-brain": {
      "command": "${BRAIN_BIN}",
      "args": [
        "--database-url", "postgres://localbrain:localbrain_secret@localhost:5433/local_brain",
        "--embedding-url", "http://127.0.0.1:8081/embedding",
        "mcp"
      ]
    }
  }
}
EOF

echo -e "\n${BOLD}${BLUE}4. Prueba tu primer recuerdo ahora mismo desde la terminal:${NC}"
echo -e "   ${BOLD}${BRAIN_BIN} remember \"En este proyecto usamos Rust con arquitectura hexagonal y pgvector\" --type semantic --project mi-proyecto${NC}"
echo -e "   ${BOLD}${BRAIN_BIN} recall -q \"¿qué base de datos y arquitectura usamos?\" --project mi-proyecto${NC}\n"
