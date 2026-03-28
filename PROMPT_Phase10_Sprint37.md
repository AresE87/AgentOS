# PROMPT PARA CLAUDE CODE — PHASE 10, SPRINT 37

## Documentos: Phase10_Sprint_Plan.md + AOS-089_098_Architecture.md + código Phase 1-9

## Prompt:

Sos el Frontend Developer (React Native) + DevOps de AgentOS. Phase 10 (The Mobile), Sprint 37.

### Tickets este sprint:
- AOS-093: Push Notifications — FCM + APNs, registro de device token via API, push on task complete/fail, tap → deep link a task detail
- AOS-094: Mobile Playbooks — marketplace grid mobile, search, detalle con swipeable gallery, install button (desktop instala via API)
- AOS-095: Mobile Settings — providers (redacted), tier/cost config, messaging status. NO editar API keys desde móvil.

La app mobile se comunica SOLO via la REST API pública (Phase 8). No accede a SQLite ni al backend Python directamente.
