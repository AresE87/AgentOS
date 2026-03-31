# Partner Enablement Runbook

## Runtime path
- Register partner through `cmd_register_partner`
- Review state through `cmd_list_partners`
- Certify partner through `cmd_certify_partner`

## Frontend path
- `frontend/src/pages/dashboard/Readiness.tsx`

## Rules
- Do not mark a partner certified without an explicit certification action.
- Keep partner type and integration level visible in the UI.
- Treat certification as operational proof, not marketing copy.

## Status
- Real registry and certification flow, still local-runtime scoped.
