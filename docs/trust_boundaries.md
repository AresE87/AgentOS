# Trust Boundaries

## Objetivo
Hacer visible y auditable que las superficies sensibles de AgentOS estan separadas por permisos reales, tenant scope y estado del vault.

## Comandos reales
- `cmd_trust_boundaries`
- `cmd_permission_enforcement_audit`
- `cmd_permission_check`

## Boundaries actuales
- `secret`: vault read/write/migrate
- `system`: terminal execute y shell execute
- `containment`: sandbox manage
- `extension`: plugin manage y plugin execute
- `network`: API surface y tenant scoping

## Evidencia
Cada enforcement sensible ahora escribe `permission_enforced` en `audit_log` con:
- capability
- agent_name
- allowed
- source
- reason

## Lectura honesta
Si una capability no tiene grants o nunca fue ejercida en runtime, el audit la muestra como `denied` o `granted_not_exercised`; no se marca como cerrada por inferencia.
