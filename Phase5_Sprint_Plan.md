# SPRINT PLAN — PHASE 5: EL MERCADO

**Proyecto:** AgentOS
**Fase:** 5 — The Market (Semanas 15–18)
**Sprints:** 4 (1 por semana)
**Preparado por:** Project Manager
**Fecha:** Marzo 2026

---

## Objetivo de la fase

Convertir AgentOS de un producto standalone en una **plataforma con marketplace**. Los usuarios pueden publicar, compartir, comprar y vender playbooks. La plataforma genera ingresos a través de comisiones (70/30), suscripciones (BYOK $19-49/mes), y planes managed (markup sobre tokens).

---

## Entregable final

Un creador publica un playbook en el marketplace con precio. Otro usuario lo compra con Stripe, se instala automáticamente. Las API keys se almacenan en vault AES-256. La plataforma cobra 30% de comisión.

---

## Tickets

| Ticket | Título | Sprint | Prioridad | Asignado a |
|--------|--------|--------|-----------|------------|
| AOS-041 | Playbook Packaging — Formato .aosp | S15 | Crítica | API Designer → Backend Dev |
| AOS-042 | Playbook Signing — Firma Ed25519 | S15 | Crítica | CISO → Backend Dev |
| AOS-043 | BYOK Vault — AES-256-GCM + OS keychain | S15 | Crítica | CISO → Backend Dev |
| AOS-044 | Marketplace API — Backend FastAPI | S16 | Crítica | Architect → Backend Dev |
| AOS-045 | Marketplace UI — Frontend en dashboard | S16 | Alta | UX/UI → Frontend Dev |
| AOS-046 | Creator Tools — Publicar, versionar, analytics | S17 | Alta | Frontend + Backend Dev |
| AOS-047 | Stripe Billing — Pagos y comisiones | S17 | Crítica | Backend Dev + CISO |
| AOS-048 | Plan Enforcement — Free/Pro/Team limits | S17 | Alta | Backend Dev |
| AOS-049 | Managed AI Plan — Token proxy con markup | S18 | Alta | Backend Dev |
| AOS-050 | Integración E2E Phase 5 | S18 | Crítica | QA + Security Auditor |

---

## Dependencias

```
Phase 4 completa
    ├── AOS-041 (Packaging) ──┬── AOS-042 (Signing)
    │                         └── AOS-044 (Marketplace API)
    │                                 ├── AOS-045 (Marketplace UI)
    │                                 ├── AOS-046 (Creator Tools)
    │                                 └── AOS-047 (Stripe Billing)
    │                                         ├── AOS-048 (Plan Enforcement)
    │                                         └── AOS-049 (Managed AI Plan)
    ├── AOS-043 (BYOK Vault)
    └──────────────────────── AOS-050 (E2E Phase 5)
```

---

## Criterios de éxito

| Métrica | Target |
|---------|--------|
| Marketplace E2E (publish → buy → install) | Funciona |
| Playbooks sembrados al launch | >= 50 |
| Vault security audit | 0 findings críticos |
| Stripe integration | Pagos reales funcionan |
| Plan limits enforced | Free/Pro/Team correctos |
