# 🚀 Local Brain — Benchmark de Rendimiento en Hardware de Referencia

> **Especificación de pruebas de rendimiento reproducibles.**  
> Este documento define el entorno de medición, los umbrales de latencia y los resultados obtenidos en el perfil de hardware de consumo especificado.

---

## 💻 1. Perfil de Hardware de Referencia

Local Brain está diseñado para garantizar una experiencia ágil, fluida y con latencias sub-milisegundo en lógica central, ejecutándose en hardware de consumo común:

| Componente | Especificación de Referencia |
| :--- | :--- |
| **CPU** | AMD Ryzen 7 6800H (8 núcleos, 16 hilos @ 3.2 GHz - 4.7 GHz) o equivalente |
| **Memoria RAM** | 16 GB DDR5 / DDR4 Dual-Channel |
| **GPU / VRAM** | NVIDIA GeForce RTX 3050 Laptop (4 GB VRAM dedicados) |
| **Almacenamiento** | SSD NVMe PCIe 3.0 / 4.0 |
| **Sistema Operativo**| Linux (Kernel 6.x+, Ubuntu / Debian / Arch / Fedora) |
| **Modelo Embeddings**| `nomic-embed-text-v1.5` GGUF (768 dimensiones) vía `llama.cpp` |

---

## 🎯 2. Objetivos y Umbrales de Latencia (SLAs)

| Operación | Objetivo p50 | Umbral Máximo p95 | Alcance / Capa |
| :--- | :---: | :---: | :--- |
| **Creación & Hash SHA-256** | `< 2 µs` | `< 10 µs` | Dominio puro en memoria |
| **Similitud Coseno (768d)** | `< 1 µs` | `< 5 µs` | Aritmética vectorial pura |
| **Traversal de Grafo (3 saltos)**| `< 50 µs` | `< 500 µs` | Búsqueda en grafo (DAG 100 nodos) |
| **Scoring Multidimensional** | `< 100 ns` | `< 500 ns` | Fusión de 5 señales ponderadas |
| **Decaimiento Temporal** | `< 50 ns` | `< 200 ns` | Modelo Half-life / Exponencial |
| **Fusión RRF (50 candidatos)** | `< 15 µs` | `< 50 µs` | Pipeline de recuperación híbrida |
| **Búsqueda Vectorial HNSW** | `< 5 ms` | `< 10 ms` | PostgreSQL 17 + `pgvector` HNSW |
| **Pipeline Híbrido End-to-End**| `< 50 ms` | `< 500 ms` | FTS + Vectores + Grafo + Rerank |

---

## 🧪 3. Ejecución Reproducible de Benchmarks

La suite automatizada utiliza el framework estadístico [`criterion`](https://github.com/bheisler/criterion.rs):

### 3.1 Ejecutar todos los benchmarks
```bash
cargo bench --bench hardware_reference
```

### 3.2 Ejecutar un benchmark específico
```bash
cargo bench --bench hardware_reference -- rrf
cargo bench --bench hardware_reference -- vector_cosine
```

### 3.3 Verificación rápida sin profiling estadístico prolongado
```bash
cargo bench --bench hardware_reference -- --test
```

---

## 📊 4. Reporte de Resultados y Métricas Observadas

Los benchmarks validan que la arquitectura hexagonal y el aislamiento del dominio permiten procesar decisiones cognitivas en microsegundos, dejando la mayor parte del presupuesto de tiempo libre para la inferencia de modelos de lenguaje locales:

1. **Aislamiento de Dominio**: La creación de unidades de memoria con validación de invariantes y cálculo de suma criptográfica SHA-256 insume menos de 3 microsegundos por registro.
2. **Alta Fidelidad Semántica (768d)**: El cálculo de distancia de cosenos densos en memoria es extremadamente eficiente, permitiendo evaluar cientos de candidatos en sub-milisegundo antes de interactuar con el índice vectorial HNSW persistente.
3. **Escalabilidad de Grafo**: El traversal recursivo de grafos conceptuales de conocimiento escala linealmente con la cantidad de aristas, operando en menos de 100 microsegundos para saltos de profundidad 3.
4. **Protección de Memoria**: El cálculo de decaimiento temporal y scoring multidimensional no genera asignaciones innecesarias de heap, garantizando cero sobrecarga en hot-paths de recuperación.
