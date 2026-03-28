# AgentOS — Estado del Proyecto

**Fecha**: 28 de marzo de 2026
**Propósito de este documento**: Describir el estado completo del proyecto antes y después de la reescritura, para evaluar si el rumbo es correcto.

---

## 1. QUE ES AGENTOS (la visión original)

Según el documento `AgentOS_Esquema_Funcional.pdf` (la spec del producto):

> Un programa que se instala en tu computadora y funciona como un equipo de empleados virtuales impulsados por IA. Le hablás por Telegram o WhatsApp, él controla tu PC y hace el trabajo por vos.

### Los 4 pilares definidos en la spec:

| Pilar | Descripción | Estado actual |
|-------|------------|---------------|
| 1. Cerebro inteligente | Router multi-LLM que elige la IA justa para cada tarea | ✅ Funcional |
| 2. Equipo de +40 especialistas | Perfiles con system prompts especializados | ✅ 40 perfiles creados |
| 3. Playbooks visuales | Grabar tareas y que la IA las repita sola | ⚠️ Estructura existe, sin UI |
| 4. Red Mesh multi-PC | Varias PCs trabajando como un equipo | ⚠️ Estructura existe, sin comunicación real |

### Prioridad de ejecución definida en la spec:
1. API directa (lo más rápido)
2. Terminal (si no hay API)
3. Pantalla (último recurso: mira y hace click)

---

## 2. STACK ORIGINAL (Python + Tauri v1)

### Tecnologías:
- **Backend**: Python 3.13, asyncio, litellm, python-telegram-bot
- **Frontend**: React 18, TypeScript, Tailwind CSS, Vite
- **Desktop**: Tauri v1 (Rust shell que ejecutaba Python como subprocess)
- **DB**: SQLite via aiosqlite
- **IPC**: JSON-RPC 2.0 sobre stdin/stdout (Python ↔ Rust)

### Módulos Python que existían (76 archivos, ~8,800 LOC):
- `agentos/core/agent.py` — Pipeline de 6 pasos
- `agentos/gateway/` — LLM Gateway con LiteLLM, router, classifier, cost tracker
- `agentos/executor/` — CLI executor con safety guard
- `agentos/store/` — SQLite con tasks, execution_log, llm_usage, chain_log
- `agentos/messaging/telegram.py` — Bot de Telegram funcional
- `agentos/screen/` — Captura con mss, pyautogui (scaffolded)
- `agentos/hierarchy/` — Cadena de agentes (scaffolded)
- `agentos/context/parser.py` — Parser de playbooks v1
- `agentos/mesh/` — 7 módulos (scaffolded, sin funcionalidad)
- `agentos/ipc_server.py` — Servidor JSON-RPC para bridge con Tauri

### Problemas del stack original:
1. **Bridge frágil**: stdin/stdout entre Rust y Python se rompía frecuentemente
2. **Pantalla negra en Tauri**: CSP bloqueaba scripts, el frontend no cargaba
3. **CMD visible**: Python subprocess creaba ventanas de CMD visibles
4. **Sin PC control real**: Screen capture existía pero no estaba integrado al pipeline
5. **Dependencia de Python**: Requería Python instalado en la PC del usuario
6. **Peso**: Python empaquetado pesaba ~100MB+

### Qué funcionaba realmente:
- ✅ Chat con LLM (via litellm)
- ✅ Clasificador de tareas
- ✅ Router de modelos
- ✅ Telegram bot
- ✅ SQLite storage
- ❌ PC control (solo scaffolded)
- ❌ Frontend en Tauri (pantalla negra)
- ❌ Playbooks (parser existía, no reproducía)
- ❌ Mesh (solo estructura)
- ❌ Instalación limpia (requería Python + CMD)

---

## 3. STACK ACTUAL (Rust + Tauri v2)

### Tecnologías:
- **Backend**: Rust (tokio, reqwest, rusqlite, serde)
- **Frontend**: React 18, TypeScript, Tailwind CSS, Vite (mismo que antes)
- **Desktop**: Tauri v2 (WebView2 nativo, un solo binario)
- **DB**: SQLite via rusqlite (embebido)
- **IPC**: Tauri v2 type-safe commands (sin bridge, sin subprocess)
- **Windows API**: `windows` crate (GDI, SendInput, UI Automation, COM)

### Módulos Rust actuales (37 archivos, ~4,468 LOC):

```
src-tauri/src/
├── agents/          (3 archivos) — 40 specialist profiles, hierarchy, registry
├── brain/           (6 archivos) — LLM gateway, classifier, router, providers (text + vision)
├── channels/        (3 archivos) — Telegram bot, Discord bot
├── config/          (3 archivos) — Settings, routing de modelos
├── eyes/            (4 archivos) — Screen capture GDI, UI Automation COM, vision LLM
├── hands/           (4 archivos) — Mouse/keyboard SendInput, CLI PowerShell, safety guard
├── memory/          (2 archivos) — SQLite (tasks, steps, llm_calls)
├── mesh/            (4 archivos) — Discovery mDNS, protocol, security
├── pipeline/        (3 archivos) — Engine multi-turn, executor prioridad, modos de ejecución
├── playbooks/       (3 archivos) — Recorder, player
├── types.rs         — Tipos compartidos (AgentAction, ExecutionResult, etc.)
├── lib.rs           — 24 IPC commands, AppState, setup de Tauri
└── main.rs          — Entry point
```

### Binario:
- **agentos.exe**: 17MB (un solo archivo, cero dependencias)
- **Instalador NSIS**: 4.2MB
- **Instalador MSI**: 6.0MB

---

## 4. QUE FUNCIONA HOY (probado y verificado)

| Feature | Estado | Evidencia |
|---------|--------|-----------|
| Instalar la app | ✅ Funciona | MSI/NSIS se instala, app abre sin CMD |
| Chat conversacional | ✅ Funciona | "bueno como estás" → respuesta del LLM |
| Selección de agente | ✅ Funciona | Auto-selecciona entre 40 especialistas |
| Multi-provider LLM | ✅ Funciona | Anthropic con fallback (OpenAI, Google si hay key) |
| Clasificador de tareas | ✅ Funciona | Detecta tipo y complejidad |
| Ejecutar comandos por voz | ✅ Funciona | "abre cmd" → CMD se abre |
| Instalar software | ✅ Funciona | "descarga e instala VLC" → winget install |
| Listar archivos | ✅ Funciona | "qué archivos hay en mis fotos" → lista real |
| Info del sistema | ✅ Funciona | "cuánto espacio tengo" → datos reales del disco |
| Auto-retry de errores | ✅ Funciona | Si PowerShell falla, LLM corrige y reintenta |
| SQLite persistence | ✅ Funciona | Tasks y steps se guardan |
| Kill switch / STOP | ✅ Funciona | Botón rojo para detener tareas |
| Telegram bot | ⚠️ Estructura lista | Inicia si hay token, necesita testing |
| Screen capture | ⚠️ Código existe | GDI capture compilado, sin testing e2e |
| Mouse/keyboard | ⚠️ Código existe | SendInput compilado, sin testing e2e |
| UI Automation | ⚠️ Código existe | COM interface compilada, sin testing e2e |
| Vision (screen mode) | ⚠️ Código existe | Requiere API key con vision (Sonnet), no testeado e2e |
| Playbook recorder | ⚠️ Solo estructura | Backend existe, sin UI en frontend |
| Playbook player | ⚠️ Solo estructura | Backend existe, sin UI |
| Mesh discovery | ⚠️ Solo self-register | No descubre otros nodos |
| Mesh comunicación | ❌ No existe | Solo protocol definido, sin transporte |
| Discord bot | ⚠️ Código existe | No wired al startup |
| WhatsApp | ❌ No existe | |
| Marketplace | ❌ No existe | |

---

## 5. QUE SE PERDIÓ EN LA REESCRITURA

Al migrar de Python a Rust, se ganó en rendimiento y distribución pero:

| Feature Python | Estado en Rust |
|----------------|---------------|
| LiteLLM (wrapper multi-provider) | Reemplazado por HTTP directo (más control, menos features) |
| python-telegram-bot (completo) | Reescrito básico (send_message, get_updates, no inline keyboards) |
| aiosqlite async | rusqlite sync (funcional pero no async nativo) |
| pytest suite (74 tests) | Cero tests en Rust |
| pyproject.toml + Makefile | Cargo.toml (build funciona pero no hay CI/CD) |
| 76 módulos Python | 37 archivos Rust (más densos, menos scaffolding) |
| Playbook parser v1 (markdown) | Playbook recorder/player (JSON, sin UI) |

---

## 6. PROBLEMAS ACTUALES Y PREOCUPACIONES

### Problemas técnicos:
1. **Web scraping limitado**: MercadoLibre y sitios SPA no funcionan con Invoke-WebRequest (necesitan JavaScript). El scraping solo sirve para sitios estáticos.
2. **Vision mode no testeado**: El modo screen (captura + click) requiere Claude Sonnet con vision. Nunca se probó end-to-end con un instalador real.
3. **Browser spam bug**: El modo `command_then_screen` puede entrar en loop abriendo ventanas. Se mitigó pero no se eliminó completamente.
4. **Sin tests**: Cero tests unitarios o de integración en el código Rust.
5. **Frontend desactualizado**: Muchas páginas del dashboard (Playbooks, Board, Mesh, Analytics, Recorder, Triggers) muestran datos mock o están vacías.

### Preocupaciones de rumbo:
1. **¿Es esto un agente de PC o un chatbot con PowerShell?** El motor actual traduce instrucciones a comandos PowerShell. Funciona para tareas simples pero no es un agente autónomo que pueda navegar UIs complejas.
2. **La vision pipeline nunca se probó de verdad**. El modo `screen` que lee la pantalla y hace clicks existe en código pero no se validó con un caso real completo.
3. **Los 4 pilares de la spec están desbalanceados**: El cerebro funciona bien, los especialistas existen pero solo como system prompts, los playbooks y el mesh son estructura sin funcionalidad.
4. **No hay wizard de primera ejecución real**: La app debería guiar al usuario, pero salta directo al dashboard con datos mock si el IPC falla.

---

## 7. ARQUITECTURA ACTUAL (diagrama)

```
AgentOS.exe (17MB, un solo binario)
│
├── Frontend (React/Tailwind en WebView2)
│   ├── Chat.tsx — interfaz principal (todo pasa por acá)
│   ├── Wizard.tsx — setup de API keys
│   ├── Home/Settings/Analytics — páginas del dashboard
│   └── useAgent.ts — 24 IPC commands al backend
│
├── Tauri v2 IPC (type-safe, sin bridge)
│   └── lib.rs — 24 commands registrados
│
├── Motor de ejecución (pipeline/engine.rs)
│   ├── LLM decide: command | multi | screen | command_then_screen | chat | done
│   ├── PowerShell: ejecuta comandos con auto-retry (2 reintentos)
│   ├── Vision: captura pantalla → LLM decide click → ejecuta (hasta 15 pasos)
│   └── Híbrido: PowerShell primero, luego vision para instaladores
│
├── Brain (brain/)
│   ├── Gateway: Anthropic, OpenAI, Google (text + vision)
│   ├── Router: cheap → standard → premium (fallback chain)
│   └── Classifier: tipo de tarea + complejidad + tier
│
├── Agents (agents/)
│   └── 40 perfiles: Programmer, Designer, Accountant, Marketing, etc.
│
├── Eyes (eyes/)
│   ├── Capture: GDI BitBlt → JPEG → base64
│   ├── UI Automation: COM IUIAutomation (lee cualquier ventana)
│   └── Vision: envía screenshot al LLM, recibe AgentAction JSON
│
├── Hands (hands/)
│   ├── Input: SendInput (mouse + keyboard)
│   ├── CLI: PowerShell/CMD con timeout y CREATE_NO_WINDOW
│   └── Safety: blacklist de comandos peligrosos
│
├── Memory (memory/)
│   └── SQLite: tasks, task_steps, llm_calls
│
├── Channels (channels/)
│   ├── Telegram: bot con polling
│   └── Discord: bot básico
│
├── Mesh (mesh/)
│   ├── Discovery: mDNS (solo self-register)
│   ├── Protocol: mensajes definidos
│   └── Security: pairing codes
│
└── Playbooks (playbooks/)
    ├── Recorder: graba acciones + screenshots
    └── Player: reproduce playbooks
```

---

## 8. COMMITS (historial completo)

```
5ef3ffe STOP button + fix browser loop bug
469f081 fix: wrong model IDs — sonnet/opus returning 404
c9d884e command_then_screen mode — download, install, configure anything
8662bcd multi-turn chain execution engine
a5418c3 definitive engine v2 — auto-retry, better PowerShell
8f3e278 graceful fallback when LLM returns non-JSON
057d9fe output now shows in chat — save to DB and poll
8477055 route everything through PC engine — LLM decides
76c4468 definitive PC control engine — complete Windows automation
387a713 LLM-powered command engine — any instruction becomes command
5985b84 7 bugs fixed in PC control pipeline
bfdca07 smart direct execution — simple commands bypass vision
4a08bf1 connect PC control to Chat
fec45d5 fix: crash on startup (tauri::async_runtime::spawn)
75b189d wire everything together — agents, Telegram, system prompts
9afddc7 Phases 3-7 — Agents, Playbooks, Mesh, Channels
3025d16 Phase 2 — PC Control (eyes + hands + pipeline)
f671f3a Phase 1 — Full Rust + Tauri v2 foundation
d1fce17 remove Python backend and Tauri v1
54f6c30 backup: Python+Tauri v1 codebase
```

---

## 9. ARCHIVOS DE REFERENCIA

- `AgentOS_Esquema_Funcional.pdf` — Spec funcional del producto (5 páginas)
- `AgentOS_Product_Specification.docx` — Spec técnica original
- `AgentOS_Team_Protocol.md` — Protocolo del equipo de 14 roles IA
- `AgentOS_Sprint_Plan_Phase1.md` — Plan de sprints original
- `docs/plans/2026-03-28-agentos-v2-full-rust-design.md` — Design doc de la reescritura
- `docs/plans/2026-03-28-phase1-foundation.md` — Plan de implementación Phase 1

---

## 10. CONCLUSIÓN Y EVALUACIÓN HONESTA

### Lo que se logró:
- App nativa de 17MB que se instala como cualquier programa
- Motor de PC control que ejecuta comandos PowerShell por lenguaje natural
- LLM multi-provider con fallback automático
- Auto-retry cuando comandos fallan
- Base sólida de código Rust compilado y estable

### Lo que falta para cumplir la visión de la spec:
1. **Vision mode real** — El agente necesita poder ver la pantalla y hacer clicks de verdad (instalar software navegando wizards). Esto requiere testing con Claude Sonnet vision.
2. **Playbooks funcionales** — Grabar una tarea y que se repita. Necesita UI en el frontend.
3. **Mesh real** — Comunicación entre PCs. Necesita gRPC o WebSocket.
4. **Canales funcionales** — Telegram y Discord probados e integrados.
5. **Tests** — Cero tests. Cualquier cambio puede romper algo sin saberlo.
6. **Frontend completo** — La mayoría de las páginas del dashboard están vacías o con mocks.

### La pregunta central:
¿Estamos construyendo un agente de PC autónomo (como la spec dice) o un wrapper de PowerShell con LLM? Para ser lo primero, el modo vision necesita funcionar de verdad — el agente tiene que poder ver la pantalla, entender lo que hay, y actuar. Esa capacidad existe en código pero nunca se validó end-to-end.
