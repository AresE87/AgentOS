# Recovery And Rollback Runbook

## Superficies reales
- retry de tareas: `cmd_retry_task`
- sync offline: `cmd_sync_offline`
- reporte de recovery: `cmd_recovery_report`
- rollback de plugins ya existente en lifecycle

## Evidencia persistida
`offline_recovery_events` guarda:
- `task_retry`
- `offline_sync`
- `rollback`

## Flujo operativo
1. Ejecutar `cmd_recovery_report`.
2. Revisar `recent_events` y `pending_items`.
3. Si hubo failure streak, reintentar una tarea concreta.
4. Si el problema vino de despliegue/plugin, ejecutar rollback del plugin afectado.
5. Reintentar sync offline cuando la conectividad vuelva.

## Estado honesto
El reporte distingue:
- recoveries exitosas
- fallas de sync
- retries pedidos
- rollbacks registrados
