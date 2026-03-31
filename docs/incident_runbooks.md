# Incident Runbooks

## Comandos reales
- `cmd_get_alerts`
- `cmd_open_incident`
- `cmd_acknowledge_alert`
- `cmd_resolve_incident`
- `cmd_incident_runbooks`

## Reglas seed
- `err-rate`
- `disk-low`
- `fail-streak`

## Runbooks persistidos
Cada regla tiene pasos persistidos en `incident_runbooks` y los incidentes abiertos quedan en `alerts`.

## Flujo
1. Abrir incidente con `cmd_open_incident`.
2. Consultar runbook ligado.
3. Ejecutar mitigacion.
4. Acknowledge cuando haya operador a cargo.
5. Resolver con nota final usando `cmd_resolve_incident`.

## Evidencia
Los incidentes ya no viven solo en memoria del proceso:
- sobreviven reinicio
- quedan consultables por IPC
- conservan timestamps y notas de resolucion
