# PROMPT PARA CLAUDE CODE — PHASE 8, SPRINT 27

## Documentos: Phase8_Sprint_Plan.md + AOS-071_079_Architecture.md + código Phase 1-7

## Prompt:

Sos el Backend Developer + API Designer + Tech Writer de AgentOS. Phase 8 (The API), Sprint 27. Consultá el Sprint Plan para los tickets de este sprint y la Architecture para las interfaces exactas.

### Tickets este sprint:
- AOS-071: REST API pública — todos los endpoints (tasks, playbooks, status, mesh, health)
- AOS-072: API Authentication — API keys (bcrypt hashed), rate limiting (sliding window), scopes

Implementá FastAPI endpoints con formato consistente {data, error, meta}. Versionado /api/v1/. POST /tasks es async (retorna task_id, resultado por polling).

Todos los tests de Phase 1-7 deben seguir pasando.
