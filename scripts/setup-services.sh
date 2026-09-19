#!/usr/bin/env bash
# ==============================================================================
# Local Brain — Setup y Bootstrap de Servicios Auxiliares Locales
#
# Levanta automáticamente:
#   1. PostgreSQL 17 con extensión pgvector (puerto 5433)
#   2. llama.cpp server con modelo nomic-embed-text-v1.5 (puerto 8081)
#
# Uso:
#   ./scripts/setup-services.sh
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "🧠 Iniciando configuración de servicios auxiliares de Local Brain..."

# 1. Verificar Docker y Docker Compose
if ! command -v docker &>/dev/null; then
    echo "❌ Error: Docker no está instalado o no se encuentra en el PATH."
    exit 1
fi

if ! docker compose version &>/dev/null; then
    echo "❌ Error: Docker Compose no está disponible."
    exit 1
fi

# 2. Descarga del modelo de embeddings (si no existe)
MODEL_DIR="${ROOT_DIR}/models"
MODEL_FILE="${MODEL_DIR}/nomic-embed-text-v1.5.Q8_0.gguf"
MODEL_URL="https://huggingface.co/nomic-ai/nomic-embed-text-v1.5-GGUF/resolve/main/nomic-embed-text-v1.5.Q8_0.gguf"

mkdir -p "${MODEL_DIR}"

if [ ! -f "${MODEL_FILE}" ]; then
    echo "📥 Descargando modelo de embeddings nomic-embed-text-v1.5 (768 dim, ~140 MB)..."
    if command -v curl &>/dev/null; then
        curl -L --progress-bar -o "${MODEL_FILE}" "${MODEL_URL}"
    elif command -v wget &>/dev/null; then
        wget -q --show-progress -O "${MODEL_FILE}" "${MODEL_URL}"
    else
        echo "⚠️ curl o wget no encontrados en el host. Docker Compose descargará el modelo automáticamente dentro del contenedor."
    fi
else
    echo "✅ Modelo de embeddings encontrado en: ${MODEL_FILE}"
fi

# 3. Levantar servicios con Docker Compose
echo "🚀 Levantando servicios (PostgreSQL 17 + pgvector y llama.cpp server)..."
cd "${ROOT_DIR}"
docker compose up -d

# 4. Esperar a que PostgreSQL esté listo
echo "⏳ Esperando a que PostgreSQL esté operativo..."
until docker exec local-brain-postgres pg_isready -U localbrain -d local_brain &>/dev/null; do
    sleep 1
done
echo "🟢 PostgreSQL 17 + pgvector listo en puerto 5433."

# 5. Esperar a que llama.cpp esté listo
echo "⏳ Esperando a que el servidor de embeddings llama.cpp esté operativo..."
MAX_RETRIES=30
RETRY=0
until curl -s "http://127.0.0.1:8081/health" &>/dev/null || [ $RETRY -ge $MAX_RETRIES ]; do
    sleep 1
    RETRY=$((RETRY + 1))
done

if [ $RETRY -lt $MAX_RETRIES ]; then
    echo "🟢 Runtime llama.cpp listo en puerto 8081 (768 dim)."
else
    echo "⚠️ Advertencia: llama.cpp aún está inicializándose o tardando en cargar."
fi

# 6. Ejecutar migraciones si brain CLI está disponible
if command -v brain &>/dev/null; then
    echo "🔄 Aplicando migraciones de base de datos..."
    brain init
    echo ""
    brain status
elif [ -f "${ROOT_DIR}/target/release/brain" ]; then
    echo "🔄 Aplicando migraciones de base de datos con binario local..."
    "${ROOT_DIR}/target/release/brain" init
    echo ""
    "${ROOT_DIR}/target/release/brain" status
else
    echo "💡 Puedes compilar la CLI con: cargo build --release --bin brain"
fi

echo ""
echo "✨ ¡Local Brain está completamente operativo!"
