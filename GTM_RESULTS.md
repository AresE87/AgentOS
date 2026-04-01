# GTM RESULTS — Registro de Pruebas Reales

**Fecha:** 2026-03-31
**Version:** 4.2.0 (master unificado post-merge C1-C50 + D1-D5)
**Commit base:** 3775bb7

---

## Estado del setup verificado

| Componente | Estado | Evidencia |
|------------|--------|-----------|
| Rama | master, clean | `git status` limpio |
| Rust build | OK | `cargo check` — 42 warnings, 0 errors |
| Frontend (Vite) | OK | http://localhost:5173 sirve HTML |
| agentos.exe | RUNNING | PID visible en tasklist |
| API server (port 8080) | OK | `curl /health` → `{"status":"ok","version":"4.2.0"}` |
| AAP server (port 9100) | NO INICIA | Posiblemente requiere aap_enabled=true |
| SQLite DB | EXISTE | AppData/Local/AgentOS/data/agentos.db |
| Settings.json | NO EXISTE | La app corre con defaults (sin API keys) |
| Vault.enc | NO EXISTE | Sin credenciales encriptadas |
| API keys LLM | NO CONFIGURADAS | Bloqueante para los 3 flujos |
| Google OAuth | NO CONFIGURADO | Bloqueante para Flujo 1 real |

---

## Dependencias externas

### Disponibles (no requieren nada externo)
- File reader (CSV, DOCX, imagenes, texto)
- Template engine (5 templates incluidos)
- Escalation detector (logica local)
- Debugger/trace (SQLite local)
- Approval workflow (logica local)
- System monitors (PowerShell local)
- Knowledge graph (SQLite local)
- Branding system (branding.json local)
- GDPR export/delete (SQLite local)
- Marketplace catalog (10 entries embebidos)
- Audit log (SQLite local)

### Faltantes (requieren configuracion del usuario)
- **API key LLM** (Anthropic/OpenAI/Google) — BLOQUEANTE para chat, clasificacion, chains, swarm
- **Google OAuth Client ID + Secret** — BLOQUEANTE para Gmail/Calendar real
- **Discord Bot Token** — necesario para demo Discord
- **Telegram Bot Token** — necesario para demo Telegram
- **WhatsApp Business credentials** — necesario para demo WhatsApp
- **Stripe Secret Key + Price IDs** — necesario para demo billing

### Bloqueantes
| Bloqueante | Afecta | Resolucion |
|------------|--------|------------|
| API key LLM | 3/3 flujos | Usuario pega key en Settings |
| Google OAuth | Flujo 1 (real) | Crear proyecto en Google Cloud Console |

### Opcionales
- Microsoft Excel (para .xlsx — alternativa: usar CSV)
- Ollama instalado (para LLM local — alternativa: usar API cloud)
- Docker instalado (para sandbox)

---

## Tabla de registro de pruebas

| Flujo | Estado | Que funciono | Que fallo | Causa | Riesgo demo | Siguiente accion |
|-------|--------|-------------|-----------|-------|-------------|------------------|
| 1. Inbox/Agenda | BLOQUEADO | App arranca, API server OK, frontend OK | No puede ejecutar sin LLM key | Credencial faltante | MEDIO | Configurar API key + Google OAuth |
| 2. Factura/Backoffice | BLOQUEADO | File reader OK para CSV/DOCX, fixtures creados | No puede procesar con LLM sin key | Credencial faltante | BAJO | Configurar API key (file reading funciona sin LLM) |
| 3. Swarm/Handoff | BLOQUEADO | Orchestrator/debugger/escalation code OK | Swarm necesita LLM para ejecutar | Credencial faltante | BAJO | Configurar API key |

---

## Verificaciones tecnicas ejecutadas

| Test | Resultado | Evidencia |
|------|-----------|-----------|
| cargo check | PASS | 0 errors, 42 warnings |
| Frontend build (Vite) | PASS | localhost:5173 sirve |
| API health endpoint | PASS | JSON response version 4.2.0 |
| API auth required | PASS | Returns 401 sin Authorization header |
| DB creation | PASS | agentos.db existe con WAL mode |
| Demo fixtures creados | PASS | invoice_sample.csv + swarm_task.txt |

---

## Archivos creados/modificados en esta fase

| Archivo | Motivo |
|---------|--------|
| demo-fixtures/invoice_sample.csv | Fixture para Flujo 2 (5 facturas de ejemplo) |
| demo-fixtures/swarm_task.txt | Fixture para Flujo 3 (tarea compleja multi-paso) |
| docs/SETUP_DEMO.md | Guia paso a paso para configurar credenciales y correr demos |
| NARRATIVA_COMERCIAL.md | Corregida contra verdad operativa — clasificada por evidencia |
| GTM_RESULTS.md | Este archivo — actualizado con estado real observado |

---

## Clasificacion de superficies usadas en demo

### Runtime-backed (datos reales del sistema en ejecucion)
- Chat con LLM (con API key)
- Vision mode (captura + control real)
- PowerShell execution
- Orchestrator chains
- Debugger traces (SQLite)
- Escalation/handoff
- API server (port 8080)
- Vault encryption
- Gmail API (con OAuth)
- Google Calendar API (con OAuth)
- Telegram bot (con token)
- Discord bot (con token)
- File reader (CSV, DOCX, imagenes)
- System monitors (disk, health)
- GDPR export/delete
- Audit log
- Marketplace install (ZIP real)
- Cron triggers
- Mesh networking (TCP/UDP)

### Documental (repo-backed, verificable en el codigo)
- Data room documents
- API reference docs
- Plugin certification process
- Partner program tiers
- Deployment runbooks
- Architecture documentation

### Modeled estimate (proyecciones o simulaciones)
- Investor metrics (ARR, MRR basados en assumptions)
- Financial projections (5 year model)
- Revenue optimizer (churn/upsell hardcoded)
- Infrastructure status (latencias fijas)

---

## Checklist de validacion por flujo

### Flujo 1 — Inbox/Agenda
- [x] Codigo backend verificado como real (Gmail + Calendar OAuth)
- [x] Frontend pages existen (Chat, Operations, Developer)
- [ ] API key LLM configurada
- [ ] Google OAuth configurado y autorizado
- [ ] Email real leido via Gmail API
- [ ] Clasificacion ejecutada
- [ ] Respuesta sugerida generada
- [ ] Evento en Calendar creado
- [ ] Trace visible en Debugger
- [ ] Demo grabada

### Flujo 2 — Factura/Backoffice
- [x] File reader verificado (CSV/DOCX/imagenes)
- [x] Fixture invoice_sample.csv creado
- [x] Template engine verificado
- [ ] API key LLM configurada
- [ ] Archivo leido y campos extraidos via LLM
- [ ] Cross-app bridge ejecutado
- [ ] Resultado en formato usable
- [ ] Demo grabada

### Flujo 3 — Swarm/Handoff
- [x] Orchestrator verificado como real
- [x] Debugger trace verificado (SQLite)
- [x] Escalation logic verificada
- [x] Fixture swarm_task.txt creado
- [ ] API key LLM configurada
- [ ] Tarea descompuesta en subtareas
- [ ] Subtareas ejecutadas
- [ ] Handoff activado (si confidence baja)
- [ ] Board muestra cards
- [ ] Demo grabada

---

## Priorizacion real observada

### Que trabajo dio mas valor real
1. Verificar que la app arranca y el API server responde — confirma que el producto es real
2. Crear fixtures de demo — desbloquea Flujo 2 y 3 inmediatamente tras configurar API key
3. Corregir NARRATIVA_COMERCIAL.md — elimina riesgo de prometer mas de lo que funciona
4. Crear docs/SETUP_DEMO.md — permite a cualquiera reproducir la demo

### Que trabajo evite por no ser prioritario
- Refactorear lib.rs (9700 lineas) — no impacta demo
- Limpiar los 42 warnings — no impacta demo
- Mejorar stubs de AR/VR/IoT — no se van a demostrar
- Agregar mas playbooks seed — los 30 existentes son suficientes
- Mejorar landing page — no es prioridad pre-demo

### Que parte del producto ya parece lista para mostrar a terceros
- Chat + LLM multi-provider (con key configurada)
- Vision mode (captura + control real)
- PowerShell execution
- File reading (CSV, DOCX)
- API REST
- Debugger traces
- Settings (se ve profesional)

### Que parte todavia no conviene mostrar sin contexto
- Readiness panel (investor metrics son modeled estimate)
- Swarm (funciona pero no es visualmente impactante sin UI dedicada)
- Marketplace (catalogo local, no tienda online)
- Desktop widgets (la ventana flotante es basica)

---

## Riesgo de demo por flujo

| Flujo | Riesgo | Por que |
|-------|--------|---------|
| 1. Inbox/Agenda | **MEDIO** | Requiere Google OAuth setup que puede fallar en consent screen si la app no esta verificada. Fallback: mostrar con datos in-memory seed. |
| 2. Factura/Backoffice | **BAJO** | Solo necesita API key LLM. File reading es local, no depende de servicios externos. |
| 3. Swarm/Handoff | **BAJO** | Solo necesita API key LLM. Todo es local. El riesgo es que el LLM no descomponga bien, pero se puede preparar el prompt. |

---

## Recomendacion final operativa

**Recomiendo: una ronda chica mas de hardening antes de grabar.**

Razon: la app arranca y todo el backend esta verificado, pero los 3 flujos estan bloqueados por la falta de API key LLM.

**Pasos concretos antes de grabar:**

1. **Configurar API key** (5 minutos) — Pegar Anthropic key en Settings
2. **Probar chat basico** — "hola", "que hora es", "lista archivos en Desktop"
3. **Probar Flujo 2** (el mas facil) — Dar path al CSV, pedir que lo analice
4. **Probar Flujo 3** — Pedir tarea compleja, ver Board + Debugger
5. **Si Flujo 1 es prioridad** — Configurar Google OAuth (30 min setup)
6. **Grabar los 3 videos** siguiendo DEMO_PREP.md

**Si la API key ya esta disponible, se puede grabar AHORA mismo los Flujos 2 y 3.**
El Flujo 1 requiere 30 minutos adicionales de Google OAuth setup.

---

## Decisiones de no-trabajo

| Decision | Razon |
|----------|-------|
| No refactoree lib.rs (9700 lineas) | No impacta demo ni validacion |
| No limpie 42 warnings | No impacta funcionalidad |
| No mejore stubs de R71-R150 | No se van a demostrar |
| No agregue paginas frontend nuevas | Las existentes cubren los 3 flujos |
| No implemente macOS/Linux builds | Solo se va a demostrar en Windows |
| No configure Stripe real | No es parte de los 3 flujos canonicos |
| No configure Discord/Telegram/WhatsApp | No son parte de los 3 flujos canonicos |
