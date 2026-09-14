# ADR-0001: Adopción de Rust como lenguaje principal del sistema

* **Estado:** Aceptado
* **Decisores:** Local Brain Core Team & Contributors
* **Fecha:** 2026-09-14

## Contexto y Declaración del Problema

Local Brain es un sistema de memoria cognitiva persistente local-first diseñado para operar en conjunto con agentes de Inteligencia Artificial locales (Claude Code, Codex, Antigravity, etc.). El sistema debe proporcionar latencias extremadamente bajas (recuperación p95 < 500 ms), seguridad de memoria garantizada sin pausas por Garbage Collector (GC), aislamiento estricto de capas (Clean/Hexagonal Architecture) y portabilidad completa entre Linux, macOS y Windows.

## Opciones Consideradas

* **Rust (2021/2024 edition)**: Lenguaje compilado sin GC, con sistema de tipos estricto (ownership/borrowing), abstracciones de costo cero y ecosistema maduro de concurrencia asíncrona (`tokio`).
* **Go**: Concurrencia sencilla mediante goroutines, pero presencia de Garbage Collector con pausas no deterministas, menor expresividad para Value Objects inmutables e invariantes de dominio ricos.
* **Python**: Rápido prototipado, pero rendimiento insuficiente para pipelines de búsqueda y embeddings, tipado dinámico propenso a errores en tiempo de ejecución y alto consumo de memoria en procesos daemon.

## Decisión Adoptada

Opción elegida: **Rust**, porque:
1. **Rendimiento predecible y sin GC**: Vital para el daemon de memoria y servidores MCP de baja latencia.
2. **Sistema de tipos fuerte**: Permite codificar invariantes de dominio (Value Objects, estados de ciclo de vida de memoria) directamente en el compilador (*Parse, don't validate*).
3. **Ecosistema asíncrono y CLI**: Soporte de primer nivel con `tokio`, `sqlx`, `clap`, `serde` y bibliotecas MCP.
4. **Clean Architecture pura**: `brain-domain` puede compilarse de forma 100% aislada en milisegundos sin dependencias de I/O o red (SRS RNF-006).

### Consecuencias Positivas

* Seguridad de memoria garantizada en tiempo de compilación.
* Consumo mínimo de memoria RAM (esencial para correr junto a modelos LLM locales).
* Facilidad para generar binarios autocontenidos y compilación cruzada.

### Consecuencias Negativas / Riesgos

* Curva de aprendizaje más pronunciada en lifetimes y concurrencia asíncrona.
* Tiempos de compilación superiores a Go o Python, mitigados con modularización multi-crate (`crates/*`).

## Enlaces y Referencias

* [SRS §8 — Arquitectura de Software](file:///home/guty_3rrez/Proyectos/local-brain/SRS%20%E2%80%94%20Local%20Brain_%20Memoria%20persistente%20local%20para%20agentes%20de%20IA.md#L385)
* [SRS §76 — Requerimientos no funcionales (RNF-001, RNF-006)](file:///home/guty_3rrez/Proyectos/local-brain/SRS%20%E2%80%94%20Local%20Brain_%20Memoria%20persistente%20local%20para%20agentes%20de%20IA.md)
