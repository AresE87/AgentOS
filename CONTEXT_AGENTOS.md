# AgentOS — Contexto Completo del Proyecto

**Version:** 4.2.0 (build actual compilando limpio)
**Repo:** https://github.com/AresE87/AgentOS
**Stack:** Tauri v2 + Rust (backend) + React 18 + TypeScript + Tailwind (frontend)
**Plataforma:** Windows (macOS/Linux: stubs preparados, no probados)

---

## ARQUITECTURA CORE

### Backend (Rust — 59,884 lineas, 254+ archivos .rs)

```
src-tauri/src/
├── agent_loop/          ★ NUEVO — Agentic tool loop (Claude Code pattern)
│   ├── runtime.rs       → AgentRuntime::run_turn() — loop hasta 25 iteraciones
│   ├── types.rs         → ContentBlock, ToolUse, ToolResult, AgentTurnResult
│   ├── compaction.rs    → Auto-compaction de contexto a 80K tokens
│   ├── session.rs       → Persistencia JSONL de sesiones
│   └── sub_agent.rs     → Sub-agent spawning (depth max 3)
│
├── tools/               ★ NUEVO — Tool Registry (Claw Code pattern)
│   ├── trait_def.rs     → trait Tool con execute(), input_schema(), permission_level()
│   ├── registry.rs      → ToolRegistry — registra y filtra tools por agente
│   ├── permission.rs    → Middleware de permisos (ReadOnly/Write/Execute/Dangerous)
│   ├── hooks.rs         → Pre/Post hooks (audit, safety, cost)
│   └── builtins/        → 14 herramientas registradas:
│       ├── bash.rs          → PowerShell/shell execution
│       ├── read_file.rs     → Lectura de archivos (50KB limit)
│       ├── write_file.rs    → Escritura de archivos
│       ├── edit_file.rs     → Edicion parcial (find & replace)
│       ├── search_files.rs  → Busqueda por patron en directorio
│       ├── screenshot.rs    → Captura de pantalla (GDI real)
│       ├── click.rs         → Click de mouse (SendInput real)
│       ├── type_text.rs     → Tipeo de teclado (SendInput real)
│       ├── web_browse.rs    → Navegacion web (reqwest + headless Chrome)
│       ├── web_search.rs    → Busqueda DuckDuckGo
│       ├── calendar.rs      → Google Calendar (OAuth real)
│       ├── email.rs         → Gmail API (OAuth real)
│       ├── memory_search.rs → RAG con embeddings OpenAI
│       └── spawn_agent.rs   → Crear sub-agentes autonomos
│
├── brain/               → LLM Gateway
│   ├── gateway.rs       → Multi-provider: Anthropic, OpenAI, Google + tool_use API
│   ├── providers.rs     → HTTP calls reales + call_with_tools()
│   ├── classifier.rs    → Clasificacion via LLM (cheap tier) con cache + keyword fallback
│   └── router.rs        → Routing por tier (cheap/standard/premium)
│
├── pipeline/            → Ejecucion de tareas
│   ├── engine.rs        → Vision loop (captura → LLM → click/type, max 10 turns)
│   ├── orchestrator.rs  → Chain execution (descompone → subtasks secuenciales)
│   └── executor.rs      → Ejecucion de acciones individuales
│
├── eyes/                → Captura de pantalla
│   ├── capture.rs       → GDI real (GetDC, BitBlt, GetDIBits) → JPEG
│   ├── multi_monitor.rs → Deteccion de monitores
│   ├── ocr.rs           → Windows OCR API via PowerShell
│   └── diff.rs          → Comparacion pixel-by-pixel de screenshots
│
├── hands/               → Control de input
│   ├── input.rs         → SendInput real (mouse + keyboard Unicode)
│   └── cli.rs           → PowerShell execution con timeout
│
├── channels/            → Canales de comunicacion
│   ├── telegram.rs      → Bot API real (long-polling, sendMessage, Markdown)
│   ├── discord.rs       → WebSocket Gateway real (heartbeat, embeds, DM/mention)
│   ├── whatsapp.rs      → Meta Graph API real (send/receive, webhook)
│   └── webhook.rs       → Axum webhook server (port 9099)
│
├── mesh/                → Red multi-PC
│   ├── transport.rs     → TCP server (port 9090, JSON newline-delimited)
│   ├── discovery.rs     → UDP broadcast (port 9091)
│   ├── capabilities.rs  → Registro de capacidades por nodo
│   ├── orchestrator.rs  → Scoring + asignacion de nodos
│   └── relay.rs         → HTTP client para relay cloud
│
├── api/                 → API REST publica
│   ├── server.rs        → Axum en port 8080 (/health, /v1/message, /v1/task/:id)
│   ├── routes.rs        → Handlers + Stripe webhook + auth middleware
│   └── auth.rs          → API key generation (aos_*) + validacion
│
├── vault/               → Encriptacion
│   └── vault.rs         → AES-256-GCM, PBKDF2 600K iterations, auto-migrate
│
├── security/            → Seguridad
│   ├── sandbox.rs       → 22 patrones bloqueados, timeout 30s, output 50KB max
│   ├── sanitizer.rs     → Input sanitization (XSS, SQL injection, path traversal)
│   └── rate_limiter.rs  → Per-key rate limiting (free/pro/team tiers)
│
├── integrations/        → Integraciones externas
│   ├── calendar.rs      → Google Calendar OAuth real + in-memory fallback
│   ├── email.rs         → Gmail API real + in-memory fallback
│   ├── database.rs      → SQLite connector (PostgreSQL/MySQL stubs)
│   └── api_registry.rs  → REST API orchestrator con 5 templates
│
├── memory/              → Memoria y persistencia
│   ├── database.rs      → SQLite (tasks, chains, triggers, daily_usage, etc.)
│   ├── store.rs         → RAG con OpenAI embeddings + cosine similarity
│   └── embeddings.rs    → Generacion de embeddings
│
├── billing/             → Facturacion
│   ├── plans.rs         → Free/Pro/Team con limites
│   ├── limits.rs        → UsageLimiter enforcement
│   └── stripe.rs        → Stripe Checkout Sessions reales + webhooks
│
├── compliance/          → GDPR y compliance
│   ├── gdpr.rs          → Export/delete all data + VACUUM
│   ├── retention.rs     → Auto-delete por antiguedad
│   ├── privacy.rs       → Privacy settings + SOC 2 checklist
│   └── reporter.rs      → Checks automaticos GDPR/SOX/HIPAA/ISO27001
│
├── enterprise/          → Enterprise
│   ├── audit.rs         → Audit log append-only en SQLite
│   ├── org.rs           → Multi-tenant organizations
│   ├── sso.rs           → OIDC SSO stub
│   ├── quotas.rs        → Department quotas
│   └── scim.rs          → SCIM provisioning stub
│
├── observability/       → Logging y alertas
│   ├── logger.rs        → Structured JSON logging con rotacion
│   ├── alerts.rs        → AlertManager con reglas configurables
│   └── health.rs        → Health dashboard (DB, LLM, API, disk)
│
├── debugger/            → Debugger de ejecucion
│   └── trace.rs         → 8-phase trace (classify→route→llm_call→execute→verify)
│
├── escalation/          → Handoff a humanos
│   └── detector.rs      → Detecta baja confianza, retries, acciones financieras
│
├── recording/           → Grabacion de pantalla
│   └── recorder.rs      → Frames JPEG a disco, metadata, cleanup
│
├── voice/               → Voz
│   ├── stt.rs           → OpenAI Whisper API real
│   └── tts.rs           → Windows SAPI real (System.Speech)
│
├── updater/             → Auto-update
│   └── checker.rs       → GitHub Releases checker + semver comparison
│
├── branding/            → White-label
│   └── config.rs        → branding.json con CSS variables
│
├── config/              → Configuracion
│   └── settings.rs      → 50+ settings con persistence JSON
│
├── platform/            → Cross-platform
│   ├── windows.rs       → Implementacion real
│   ├── macos.rs         → Stub cfg-gated
│   └── linux.rs         → Stub cfg-gated
│
├── plugins/             → Sistema de plugins
│   ├── manager.rs       → Carga y ejecuta .ps1/.py scripts
│   ├── manifest.rs      → plugin.json manifest
│   └── api_v2.rs        → Plugin UI pages + scoped storage
│
├── [30+ modulos mas]    → Verticals, economy, devices, autonomous, reasoning, etc.
│                          (estructura para futuras features)
└── lib.rs               → 11K+ lineas: AppState, 200+ IPC commands, setup
```

### Frontend (React + TypeScript — 7,618 lineas, 40 archivos)

```
frontend/src/
├── hooks/useAgent.ts       → 520+ funciones IPC (todas las capacidades del backend)
├── pages/
│   ├── Wizard.tsx          → Onboarding 3 pasos (Welcome → API Key → Ready)
│   ├── Dashboard.tsx       → Layout con sidebar navigation
│   └── dashboard/
│       ├── Home.tsx        → Dashboard con metricas
│       ├── Chat.tsx        → Chat principal con el agente
│       ├── Board.tsx       → Kanban de cadenas/subtareas
│       ├── Playbooks.tsx   → Playbooks CRUD + marketplace
│       ├── Analytics.tsx   → Graficos Recharts
│       ├── Mesh.tsx        → Red multi-PC
│       ├── Settings.tsx    → Configuracion (API keys, integraciones)
│       ├── Developer.tsx   → Debugger traces, shell, vision test
│       ├── Operations.tsx  → Health, alerts, logs, relay
│       ├── Readiness.tsx   → Metricas investor, partners, data room
│       ├── FeedbackInsights.tsx → Insights de feedback
│       ├── ScheduledTasks.tsx   → Triggers cron
│       └── Handoffs.tsx    → Escalaciones a humanos
├── components/
│   ├── Card.tsx, Button.tsx, Badge.tsx, Modal.tsx
│   ├── Toast.tsx, StatusDot.tsx, TimeAgo.tsx
│   └── EmptyState.tsx
├── i18n/                   → Traducciones en/es/pt
└── mocks/tauri.ts          → Mock layer para dev sin backend
```

### Mobile (React Native + Expo — 501 lineas)

```
mobile/
├── App.tsx                 → Navigator (Setup → Chat → Status)
├── src/api/client.ts       → HTTP client contra API port 8080
└── src/screens/
    ├── SetupScreen.tsx     → Config IP + API key
    ├── ChatScreen.tsx      → Chat con bubbles
    └── StatusScreen.tsx    → Estado + quick actions
```

---

## CAPACIDADES REALES (Runtime-Backed)

### Core
- Chat multi-provider: Anthropic Claude, OpenAI GPT, Google Gemini
- Agentic tool loop: el LLM decide que herramientas usar (hasta 25 iteraciones)
- 14 herramientas registradas con schemas JSON formales
- Sub-agent spawning (el LLM crea sub-agentes, depth max 3)
- Vision mode: captura pantalla + analisis LLM + control mouse/teclado
- PowerShell execution con sandbox (22 patrones bloqueados, timeout 30s)
- Orchestrator: descompone tareas complejas en subtareas secuenciales
- Clasificacion LLM (cheap tier) con cache + keyword fallback

### Integraciones
- Telegram bot (long-polling real)
- Discord bot (WebSocket Gateway real, heartbeat, embeds)
- WhatsApp Business (Meta Graph API real)
- Google Calendar (OAuth2 real, CRUD eventos)
- Gmail (OAuth2 real, list/send/search)
- Ollama (HTTP real a API local)
- API REST publica (Axum port 8080, auth con API keys)
- Mesh networking (TCP/UDP LAN, relay HTTP para cloud)

### Datos y Seguridad
- SQLite database (WAL mode, 15+ tablas)
- AES-256-GCM vault (PBKDF2 600K iterations)
- RAG con OpenAI embeddings + cosine similarity
- GDPR export/delete + retention policies
- Audit log append-only
- Structured JSON logging con rotacion
- Rate limiting per-plan (free: 100/min, pro: 1000/min)
- Stripe billing real (checkout sessions, webhooks)

### UX
- System tray (close-to-tray, context menu)
- i18n (English, Spanish, Portuguese)
- Desktop widgets (Tauri secondary windows)
- 30 seed playbooks con pasos PowerShell reales
- Marketplace (10 packages, ZIP install real)
- Voice STT (OpenAI Whisper) + TTS (Windows SAPI)
- Headless browser (Chrome/Edge --dump-dom)
- Auto-update checker (GitHub Releases)

---

## REPOS EXTERNOS DESCARGADOS

### 1. Claude Code (tanbiralam/claude-code)
**Ubicacion:** `C:\Users\AresE\Documents\repos\claude-code`
**Que es:** Source code leaked de Claude Code de Anthropic — el CLI agentico oficial
**Stack:** TypeScript, Bun, React/Ink (terminal UI)
**Lineas:** 512,000+ LOC, 1,884 archivos

**Patrones clave ya integrados en AgentOS:**
- Agentic tool loop (QueryEngine loop hasta end_turn) → **INTEGRADO** como agent_loop/runtime.rs
- Tool Registry con permission middleware → **INTEGRADO** como tools/registry.rs
- Sub-agent spawning (AgentTool) → **INTEGRADO** como tools/builtins/spawn_agent.rs
- Context compaction → **INTEGRADO** como agent_loop/compaction.rs
- Hook system (pre/post tool) → **INTEGRADO** como tools/hooks.rs

**Patrones que se podrian integrar en el futuro:**
- Coordinator Mode (main agent + restricted workers)
- Plan Mode (user approves plan, all tools auto-approved)
- Memory as attachment messages (semantic injection)
- Streaming responses (real-time token streaming)
- Session transcripts con resume
- ML-based auto-approval classifier
- MCP (Model Context Protocol) integration

### 2. Claw Code Parity (ultraworkers/claw-code-parity)
**Ubicacion:** `C:\Users\AresE\Documents\repos\claw-code`
**Que es:** Rewrite en Rust del harness de Claude Code — 100% safe Rust
**Stack:** Rust 2021, Tokio, reqwest, 9 crates en workspace
**Lineas:** ~50,000 LOC

**Patrones clave ya integrados en AgentOS:**
- ConversationRuntime::run_turn() loop → **INTEGRADO** como AgentRuntime::run_turn()
- ToolExecutor trait → **INTEGRADO** como Tool trait
- PermissionPolicy con regex rules → **PARCIALMENTE INTEGRADO** (permission levels)
- Hook system (PreToolUse/PostToolUse) → **INTEGRADO** como HookRegistry
- Session JSONL persistence → **INTEGRADO** como session.rs
- Auto-compaction por threshold → **INTEGRADO** como compaction.rs

**Patrones que se podrian integrar en el futuro:**
- SSE streaming parser (IncrementalSseParser)
- Prompt cache management (Anthropic cache_control)
- Plugin system via .so/.dll dynamic loading
- LSP integration (goto definition, references)
- MCP server management (stdio, SSE, HTTP)
- Bash security validators (18 submodules)
- Project context discovery (CLAUDE.md scanning)

---

## POSIBILIDADES DE INTEGRACION FUTURA

### Nivel 1 — Quick wins (1-2 dias cada uno)
1. **Streaming responses** — transmitir tokens en tiempo real al frontend (como Claude Code)
2. **Plan Mode** — el usuario aprueba un plan, luego el agente ejecuta sin pedir permiso
3. **MCP client** — conectar con MCP servers externos (GitHub, Slack, etc.)
4. **Prompt caching** — usar cache_control de Anthropic para reducir costos

### Nivel 2 — Medium effort (3-5 dias)
5. **Coordinator Mode** — un agente principal delega a workers con tools restringidos
6. **LSP integration** — goto definition, references, diagnostics en archivos de codigo
7. **Streaming SSE** — parser de Server-Sent Events para responses en real-time
8. **Dynamic plugin loading** — cargar .dll/.so plugins en runtime (como Claw Code)

### Nivel 3 — Major effort (1-2 semanas)
9. **Full MCP server** — exponer AgentOS como MCP server para otros agentes
10. **Autonomous goal management** — agente mantiene goals persistentes entre sesiones
11. **Multi-agent coordination** — equipos de agentes con roles y comunicacion
12. **Cross-platform real** — compilar y probar en macOS/Linux

---

## COMO CORRER EL PROYECTO

### Desarrollo
```bash
cd C:\Users\AresE\Documents\AgentOS
cargo tauri dev
# Frontend: http://localhost:5173
# API: http://localhost:8080
# App: ventana Tauri desktop
```

### Build instalador
```bash
cargo tauri build
# Output: src-tauri/target/release/bundle/nsis/AgentOS_1.0.0_x64-setup.exe
```

### Configuracion minima
1. Instalar AgentOS
2. Abrir Settings → pegar API key (Anthropic recomendado)
3. Escribir en Chat: "lista los archivos en mi Desktop"
4. El agente usa el tool loop para ejecutar `bash` y responder

### Endpoints
- `http://localhost:8080/health` — health check
- `http://localhost:8080/v1/message` — enviar tarea (requiere Bearer token)
- `http://localhost:8080/v1/status` — estado del agente
- `http://localhost:9100/aap/health` — Agent-to-Agent Protocol

---

## ESTADO HONESTO

| Categoria | Real | Estructura | Stub |
|-----------|------|-----------|------|
| Core (chat, vision, tools) | 35 features | 5 | 0 |
| Integraciones | 12 features | 3 | 5 |
| Enterprise | 8 features | 4 | 3 |
| Advanced AI | 5 features | 10 | 15 |
| Devices/IoT | 0 features | 0 | 10 |
| **Total** | **~60 reales** | **~22 estructura** | **~33 stubs** |

El 52% de las features son reales y funcionales.
El 19% tiene la estructura pero necesita integracion.
El 29% son stubs para futuras features.
