# ⚡ Guía de Inicio Rápido (Quickstart) — Local Brain v1.0

> **Empieza a darle memoria cognitiva persistente a tus agentes de IA en menos de 3 minutos.**

[English](QUICKSTART.md) | 🌐 **Español**

---

## 🎯 ¿Qué vas a lograr?
Al terminar esta guía, tendrás:
1. **Infraestructura local lista**: PostgreSQL 17 con `pgvector` y el servidor de embeddings `llama.cpp` corriendo 100% offline en tu máquina.
2. **CLI `brain` operativa**: Inicializada con esquemas relacionales, índices HNSW de 768 dimensiones y diagnóstico de salud verificado.
3. **Agentes conectados vía MCP**: Claude Code, Cursor, Claude Desktop o Antigravity compartiendo recuerdos, aprendizajes y relaciones de grafo.

---

## 📋 Prerrequisitos Mínimos

- **Docker y Docker Compose** (v2+)
- **Sistema Operativo**: Linux (Ubuntu, Debian, Fedora, Arch, etc.), macOS (Intel o Apple Silicon), o Windows (vía WSL2).
- **Recursos recomendados**: 4 GB de RAM disponibles y ~2 GB de espacio en disco (para PostgreSQL + pgvector y el modelo nomic-embed-text).

---

## 🚀 Método 1: Asistente Automatizado (Recomendado)

Ejecuta el asistente interactivo en la raíz del repositorio:

```bash
./scripts/quickstart.sh
```

El script se encargará automáticamente de:
1. Descargar el modelo de embeddings GGUF cuantizado (~140 MB).
2. Levantar los contenedores de Docker en segundo plano.
3. Esperar que PostgreSQL y llama.cpp estén saludables.
4. Aplicar las migraciones de base de datos (`brain init`).
5. Ejecutar un diagnóstico de salud del sistema (`brain doctor`).
6. Imprimir los bloques de configuración MCP listos para tu cliente favorito.

---

## 🛠️ Método 2: Paso a Paso Manual

### Paso 1: Descargar el Binario de Local Brain
Descarga el binario precompilado para tu plataforma desde [GitHub Releases](https://github.com/guty3rrez/local-brain/releases):

```bash
# Ejemplo para Linux x86_64:
curl -sL https://github.com/guty3rrez/local-brain/releases/latest/download/brain-linux-x86_64.tar.gz | tar xz
sudo install -m 755 brain /usr/local/bin/brain
```

*O compila directamente con Rust si tienes Cargo instalado:*
```bash
cargo build --release --bin brain
sudo install -m 755 target/release/brain /usr/local/bin/brain
```

### Paso 2: Levantar los Servicios Locales
En la raíz del proyecto:
```bash
docker compose up -d
```
Verifica que los servicios estén corriendo:
```bash
docker compose ps
```
- `local-brain-postgres` en puerto `5433`
- `local-brain-embeddings` en puerto `8081`

### Paso 3: Inicializar la Persistencia
Ejecuta las migraciones de base de datos:
```bash
brain init
```
Salida esperada:
```text
🧠 Inicializando Local Brain...
✅ Base de datos inicializada exitosamente.
   Instancia:    PostgreSQL 17 + extensión pgvector
   Migraciones:  Aplicadas con éxito (memories, pgvector, HNSW 768 dim).
🚀 Local Brain está listo para operar con búsqueda semántica.
```

### Paso 4: Diagnóstico de Salud
Verifica que todo el stack esté al 100%:
```bash
brain doctor
```
```text
🩺 Diagnóstico de Salud de Local Brain:
  [OK] Conexión a PostgreSQL 17
  [OK] Extensión pgvector activa
  [OK] Runtime de embeddings llama.cpp (768 dimensiones)
  [OK] Modo offline estrictamente local
✨ Estado general: ÓPTIMO (Todos los componentes operativos).
```

---

## 🤖 Conectar tus Agentes de IA vía MCP

Local Brain implementa el protocolo abierto **Model Context Protocol (MCP)** sobre `stdio`. Elige tu entorno y añade la configuración correspondiente:

### 1. Claude Desktop
Edita tu archivo `claude_desktop_config.json`:
- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Linux**: `~/.config/Claude/claude_desktop_config.json`
- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "local-brain": {
      "command": "brain",
      "args": [
        "--database-url", "postgres://localbrain:localbrain_secret@localhost:5433/local_brain",
        "--embedding-url", "http://127.0.0.1:8081/embedding",
        "mcp"
      ]
    }
  }
}
```

### 2. Claude Code CLI
Añade Local Brain en un solo comando:
```bash
claude mcp add local-brain brain -- --database-url postgres://localbrain:localbrain_secret@localhost:5433/local_brain --embedding-url http://127.0.0.1:8081/embedding mcp
```

### 3. Cursor & Windsurf
Crea o edita `.cursor/mcp.json` en la raíz de tu proyecto (o la configuración global de MCP en Cursor/Windsurf Settings):

```json
{
  "mcpServers": {
    "local-brain": {
      "command": "brain",
      "args": [
        "--database-url", "postgres://localbrain:localbrain_secret@localhost:5433/local_brain",
        "--embedding-url", "http://127.0.0.1:8081/embedding",
        "mcp"
      ]
    }
  }
}
```

### 4. Cline / Roo Code (VS Code Extension)
En `cline_mcp_settings.json`:
```json
{
  "mcpServers": {
    "local-brain": {
      "command": "brain",
      "args": [
        "--database-url", "postgres://localbrain:localbrain_secret@localhost:5433/local_brain",
        "--embedding-url", "http://127.0.0.1:8081/embedding",
        "mcp"
      ],
      "disabled": false,
      "autoApprove": [
        "brain_remember",
        "brain_recall",
        "brain_retrieve",
        "brain_search",
        "brain_relate",
        "brain_graph",
        "brain_learn",
        "brain_explain"
      ]
    }
  }
}
```

---

## 🧪 Comprobación Rápida desde la Terminal

Puedes interactuar con tu cerebro directamente desde la CLI:

### 1. Guardar una decisión o aprendizaje técnico
```bash
brain remember \
  "En el módulo de autenticación usamos tokens PASETO v4 en lugar de JWT para evitar ataques de algoritmo 'none'" \
  --type semantic \
  --project mi-backend \
  --confidence 0.95
```

### 2. Recuperar por significado semántico
```bash
brain recall -q "¿qué formato de tokens de autenticación usamos?" --project mi-backend
```

### 3. Recuperación híbrida avanzada con desglose de scoring
```bash
brain retrieve "seguridad en autenticación" --project mi-backend --explain
```

### 4. Conectar conceptos en el Grafo de Conocimiento
```bash
brain relate PASETO JWT --type PREFERS --context "Mayor seguridad criptográfica y prevención de vulnerabilidades conocidas"
```

### 5. Ver el Grafo
```bash
brain graph PASETO --depth 1
```

---

## ❓ Preguntas Frecuentes y Solución de Problemas

#### ¿Puedo usar Local Brain sin GPU?
**Sí.** El modelo de embeddings `nomic-embed-text-v1.5` en formato cuantizado Q8_0 requiere solo ~140 MB de RAM y se ejecuta con latencias menores a 15 ms por embedding en procesadores x86_64 modernos utilizando CPU pura. Si tienes GPU NVIDIA, `docker-compose.yml` puede aprovechar CUDA para acelerar el procesamiento.

#### ¿Mis datos se envían a algún servidor externo?
**No.** Local Brain aplica una política de aislamiento local estricta (`--offline`). Toda la persistencia reside en tu PostgreSQL local y todos los vectores se calculan en tu servidor local de `llama.cpp`. No existe ninguna telemetría.

#### ¿Cómo hago una copia de seguridad de mis recuerdos?
Puedes exportar e importar tu base de conocimiento en formatos JSONL o SQL con integridad criptográfica:
```bash
# Exportar respaldo
brain backup -o mi_cerebro.jsonl

# Restaurar respaldo
brain restore -i mi_cerebro.jsonl
```
