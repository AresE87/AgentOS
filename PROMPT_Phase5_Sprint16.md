# PROMPT PARA CLAUDE CODE — PHASE 5, SPRINT 16

## Documentos que adjuntás:

1. Phase5_Sprint_Plan.md
2. AOS-041_050_Architecture.md (sección AOS-044 Marketplace API)
3. AOS-022_027_UX_Design.md (referencia design system para la UI)
4. El código completo + Sprint 15

---

## El prompt (copiá desde acá):

Sos el Backend + Frontend Developer del equipo de AgentOS. Phase 5, Sprint 16. El packaging, signing, y vault ya funcionan. Ahora construís el marketplace: API backend + UI en el dashboard.

## Lo que tenés que producir

### Ticket 1: AOS-044 — Marketplace API
- `marketplace_server/` → Nuevo directorio para el servidor FastAPI
- `marketplace_server/main.py` → FastAPI app con todos los endpoints del doc
- `marketplace_server/models.py` → SQLAlchemy models (o asyncpg raw queries)
- `marketplace_server/auth.py` → Autenticación con API keys
- Todos los endpoints: publish, search, download, rate, auth
- Rate limiting: 100 req/min por usuario
- Puede correr con SQLite (dev) o PostgreSQL (prod)
- Tests de cada endpoint

### Ticket 2: AOS-045 — Marketplace UI
- Reemplazar "Coming Soon" en Playbooks page
- Grid de playbooks con nombre, autor, rating, precio, categoría
- Búsqueda con filtros (categoría, precio, rating)
- Detalle: README renderizado, reviews, botón Install/Buy
- Instalación gratuita: download → unpack → verify → install
- Instalación paga: Stripe checkout → download → install (Stripe viene en S17, por ahora mock)
- Rating/review UI
- Loading, error, empty states

## Reglas

- El marketplace server es un proyecto SEPARADO del agente (diferente directorio, diferente pyproject.toml).
- Para dev, el server corre localmente. Para prod, se despliega en un servidor.
- El agente se comunica con el marketplace via HTTP (configurable marketplace_url en settings).
- La UI usa el design system existente (colores, tipografía de AOS-022_027).

Empezá con AOS-044.
