# C24 — COMPLIANCE AUTOMATION REAL

## Objetivo
Construir automatización de compliance real y limitada, empezando por una capacidad concreta y auditable.

## Alcance recomendado
No intentar SOX/HIPAA/GDPR completos.
Empezar por:
- export de auditoría
- reporte de acciones por período
- evidencia de approvals/handoffs
- borrado/export de datos ya existentes

## Qué implementar
1. Generación de reporte estructurado.
2. Inclusión de evidencia y timestamps.
3. Filtros por período / agente / usuario / estado.
4. Export real (JSON/CSV/PDF si ya existe infraestructura).

## Definition of Done
- se genera un reporte de compliance/auditoría usable
- el reporte parte de datos reales del sistema
- no es una plantilla vacía

## Demo obligatoria
“Generar reporte de auditoría de una semana con approvals, handoffs y ejecuciones”.

## No hacer
- usar nombres regulatorios enormes sin soporte real
