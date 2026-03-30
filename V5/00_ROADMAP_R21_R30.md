# AgentOS v2 — Roadmap R21-R30: De producto a plataforma

**Fecha:** 29 de marzo de 2026
**Estado:** R1-R20 completas. Producto funcional publicado en Windows.
**Objetivo:** Expandir AgentOS de "app que funciona" a "plataforma con ecosystem".

---

## Contexto

Después de R20 tenemos:
- Agente autónomo con vision, CLI, y web browsing
- Orchestrator que descompone y ejecuta cadenas reales
- Playbooks que se graban y reproducen
- Telegram + Discord funcionando
- System tray, triggers, analytics
- Board Kanban en tiempo real
- Mesh básico entre 2 PCs
- Design System v2 aplicado
- Instalador Windows publicado

Lo que falta para ser una **plataforma**:
- Que otros creen y vendan playbooks (Marketplace)
- Que developers integren AgentOS en sus sistemas (API)
- Que funcione en macOS y Linux (cross-platform)
- Que se pueda controlar desde el teléfono (Mobile)
- Que funcione sin internet (Local LLMs)
- Que empresas lo adopten (Enterprise)

---

## Las 10 fases R21-R30

| Fase | Nombre | Qué agrega |
|------|--------|-----------|
| R21 | **Vault seguro** | API keys en vault encriptado AES-256, keychain del OS |
| R22 | **Marketplace** | Browse, instalar, publicar, y calificar playbooks |
| R23 | **Billing** | Stripe: compras, suscripciones, planes Free/Pro/Team, revenue split |
| R24 | **API pública** | REST API + Python SDK + CLI tool para developers |
| R25 | **LLMs locales** | Ollama/llama.cpp, offline mode, prefer-local toggle |
| R26 | **Cross-platform** | macOS .dmg + Linux AppImage, platform abstraction |
| R27 | **Mobile app** | React Native companion: chat, tasks, playbooks, push notifications |
| R28 | **Auto-mejora** | Routing optimizer, learning from corrections, weekly insights |
| R29 | **Enterprise** | SSO (OIDC), audit logs, multi-tenant, admin dashboard |
| R30 | **Ecosystem** | Docs site, 30 seed playbooks, onboarding optimizado, creator program |

**Regla:** Cada fase termina con demo grabable en video.
