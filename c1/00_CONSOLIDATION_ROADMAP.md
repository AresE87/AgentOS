# AgentOS — Consolidación C1-C10: De fachada a producto real

**Fecha:** 30 de marzo de 2026
**Basado en:** AUDIT_R150.md — 30 features reales (20%), 80 fachadas (53%)
**Objetivo:** Convertir las 10 fachadas MÁS VALIOSAS en features reales. No agregar nada nuevo — hacer que lo que existe FUNCIONE.

---

## Principio: REEMPLAZAR stubs, no crear módulos nuevos

Cada fase de consolidación toma código que YA EXISTE (structs, IPC, UI) y le conecta el backend REAL. No hay que crear archivos nuevos — hay que llenar los que están vacíos.

---

## Las 10 fases de consolidación

| Fase | Qué arregla | Estado actual → Estado después |
|------|------------|-------------------------------|
| C1 | **Stripe billing** | ❌ URLs placeholder → ✅ Checkout real, webhooks, planes |
| C2 | **Auto-update** | ❌ No existe → ✅ Check + download + install automático |
| C3 | **Google Calendar** | 🔲 CRUD memoria → ✅ OAuth real, read/write eventos |
| C4 | **Gmail integration** | 🔲 CRUD memoria → ✅ OAuth real, IMAP, enviar/leer |
| C5 | **Discord bot** | ❌ No existe → ✅ WebSocket gateway, mensajes reales |
| C6 | **RAG con embeddings** | ⚠️ LIKE search → ✅ Embeddings reales, cosine similarity |
| C7 | **Clasificador LLM** | 🔲 Keywords → ✅ LLM call barato para clasificar |
| C8 | **Frontend real** | ⚠️ Básico → ✅ Board, debugger, approval, conversation view |
| C9 | **Desktop widgets** | 🔲 Configs memory → ✅ Ventanas flotantes reales (Tauri) |
| C10 | **Headless browser** | ⚠️ reqwest HTML → ✅ chromiumoxide para SPAs con JavaScript |

**Regla:** Cada fase termina cuando el feature que ERA stub ahora funciona en un demo grabable. No se agrega NADA que no esté ya scaffoldeado.
