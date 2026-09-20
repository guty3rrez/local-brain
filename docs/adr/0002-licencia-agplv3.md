# ADR-0002: Adopción de Licencia GNU AGPLv3 (Affero General Public License v3)

* **Estado:** Aceptado
* **Decisores:** Local Brain Core Team & Contributors
* **Fecha:** 2026-09-14

## Contexto y Declaración del Problema

Local Brain nace como una herramienta soberana para devolver el control y persistencia del conocimiento a los desarrolladores y agentes de IA locales. Sin una licencia copyleft fuerte que proteja el uso en red y servicios en la nube, proveedores comerciales de IA podrían empaquetar el motor de memoria como un servicio SaaS privativo sin revertir mejoras a la comunidad (el *network loophole* de licencias GPL tradicionales o licencias permisivas tipo MIT/Apache).

## Opciones Consideradas

* **GNU AGPLv3 (Affero General Public License v3)**: Copyleft fuerte que cierra expresamente la brecha de servicios de red (SaaS). Si alguien modifica Local Brain y lo ofrece a través de la red (ej. APIs remotas o servidores de memoria propietarios), debe liberar el código fuente bajo la misma licencia.
* **Apache-2.0 / MIT**: Licencias altamente permisivas que permiten a grandes proveedores de nube cerrar el código, privatizar el backend y crear monopolios propietarios sobre la tecnología sin contribuir upstream.
* **GNU GPLv3**: Copyleft fuerte para binarios distribuidos, pero no cubre el escenario donde el software se ejecuta exclusivamente como servicio backend / API remota.

## Decisión Adoptada

Opción elegida: **GNU AGPLv3 (AGPL-3.0-or-later)**.
Garantiza perpetuamente la libertad del software, fomenta la colaboración comunitaria y asegura que cualquier modificación o extensión del motor central de memoria permanezca abierta y accesible para todos los agentes y desarrolladores.

### Consecuencias Positivas

* Protección contra el secuestro comercial de infraestructura (*anti-SaaS lock-in*).
* Certeza jurídica para la comunidad de código abierto.
* Alineación con los principios de *Local-first* y *Soberanía de Datos* del SRS.

### Consecuencias Negativas / Riesgos

* Algunas organizaciones con políticas corporativas restrictivas hacia licencias copyleft requerirán revisión interna antes de integrar el código como dependencia directa (sin embargo, el consumo vía MCP sobre stdio/red no afecta a los clientes consumidores, solo a modificaciones del servidor en sí).

## Enlaces y Referencias

* [LICENSE (GNU AGPLv3)](file:///home/guty_3rrez/Proyectos/local-brain/LICENSE)
* [README — Principios de Diseño y Licencia](file:///home/guty_3rrez/Proyectos/local-brain/README.md#-licencia)
