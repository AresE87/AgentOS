# PROMPT PARA CLAUDE CODE — PHASE 5, SPRINT 17

## Documentos que adjuntás:

1. Phase5_Sprint_Plan.md
2. AOS-041_050_Architecture.md (secciones AOS-047, AOS-048)
3. El código completo + Sprint 15+16

---

## El prompt (copiá desde acá):

Sos el Backend + Frontend Developer del equipo de AgentOS. Phase 5, Sprint 17 — monetización. El marketplace funciona (publicar, buscar, instalar gratis). Ahora agregás pagos con Stripe, herramientas de creador, y enforcement de planes.

## Lo que tenés que producir

### Ticket 1: AOS-046 — Creator Tools
- Nueva sección "Creator Studio" en el dashboard
- Publish flow: seleccionar playbook → agregar metadata → pack → sign → upload
- Versioning: publicar nueva versión (semver)
- Analytics: descargas, ingresos, ratings (datos del marketplace API)
- Editar listing, retirar del marketplace

### Ticket 2: AOS-047 — Stripe Billing
- Integración Stripe en marketplace_server
- Stripe Checkout para compras únicas y suscripciones de playbooks
- Stripe Connect para payouts a creadores (70/30 split)
- Webhooks de Stripe (verificación de firma, idempotency)
- Stripe Checkout para suscripción a planes (Free→Pro→Team)
- Stripe keys en vault del servidor
- Tests con Stripe test mode

### Ticket 3: AOS-048 — Plan Enforcement
- `agentos/plan_enforcer.py` → PlanEnforcer middleware
- Free: 100 tasks/mo, 1 playbook, Junior only
- Pro: 2000 tasks/mo, unlimited playbooks, all levels
- Team: unlimited + 5 seats + team features
- Counter mensual de tareas con reset automático
- Graceful degradation: mensaje claro + link a upgrade
- Plan visible en dashboard Settings y Home
- Trial de 14 días de Pro para nuevos usuarios
- Tests de cada límite por plan

## Reglas

- Stripe API keys NUNCA en código ni en logs — solo en vault.
- NUNCA manejar datos de tarjeta — todo via Stripe Checkout.
- Verificar firma de Stripe en CADA webhook.
- Plan enforcement es un middleware que se ejecuta ANTES de cada tarea.
- Para testing: Stripe test mode con test API keys.

Empezá con AOS-046.
