# AgentOS v2 — Full Rust + Tauri v2 Architecture Design

**Date**: 2026-03-28
**Status**: Draft — pending approval
**Source spec**: `AgentOS_Esquema_Funcional.pdf` (5 pages)

---

## Vision

> Un programa que se instala en tu computadora y funciona como un equipo de
> empleados virtuales impulsados por IA. Le hablas por Telegram o WhatsApp,
> el controla tu PC y hace el trabajo por vos.

## Decision: Full Rust rewrite

The current codebase (Python backend + Tauri v1 + stdin/stdout bridge) is
fundamentally broken — the IPC is fragile, the Tauri CSP blocks scripts, and
the Python subprocess creates visible CMD windows. A professional product
requires a single-binary, zero-dependency architecture.

**Stack**: Rust + Tauri v2 + React/TypeScript/Tailwind
**DB**: SQLite embedded (rusqlite) — offline-first
**Distribution**: Single `.msi` installer, ~20MB

---

## Module Map (aligned to PDF spec)

```
agentos/
├── brain/          # Pilar 1: Cerebro inteligente
│   ├── gateway.rs        # Multi-provider LLM abstraction (reqwest HTTP)
│   ├── router.rs         # Task → tier → model selection (YAML config)
│   ├── classifier.rs     # Classify task type + complexity
│   ├── orchestrator.rs   # Decompose complex tasks, assign agent levels
│   ├── cost.rs           # Budget control per-task, daily, monthly
│   └── local.rs          # Local model support (llama-cpp-rs bindings)
│
├── agents/         # Pilar 2: Equipo de +40 especialistas
│   ├── registry.rs       # Load/manage agent profiles from YAML
│   ├── profiles/         # 40+ YAML files (programmer, designer, accountant...)
│   │   ├── programmer.yaml
│   │   ├── designer.yaml
│   │   ├── accountant.yaml
│   │   ├── marketing.yaml
│   │   ├── lawyer.yaml
│   │   ├── analyst.yaml
│   │   ├── sales.yaml
│   │   └── ... (+30 more)
│   ├── hierarchy.rs      # Junior → Specialist → Senior → Manager levels
│   └── chain.rs          # Multi-agent collaboration for complex tasks
│
├── playbooks/      # Pilar 3: Playbooks visuales
│   ├── recorder.rs       # Record user actions (screenshots + clicks + keys)
│   ├── player.rs         # Replay a playbook autonomously
│   ├── parser.rs         # Load playbook format (YAML + screenshots)
│   ├── builder.rs        # Create playbook from recorded session
│   └── marketplace.rs    # Package, sign, publish, download playbooks
│
├── mesh/           # Pilar 4: Red Mesh multi-PC
│   ├── discovery.rs      # mDNS for LAN auto-discovery
│   ├── transport.rs      # gRPC/WebSocket communication layer
│   ├── relay.rs          # Cloud relay for WAN connectivity
│   ├── protocol.rs       # Task request/response/streaming protocol
│   ├── sync.rs           # Shared state sync (tasks, skills)
│   ├── failover.rs       # Task reassignment when a PC disconnects
│   └── security.rs       # E2E encryption, token auth, never share passwords
│
├── eyes/           # Perception system
│   ├── capture.rs        # Windows Graphics Capture API (screen capture)
│   ├── ui_auto.rs        # Windows UI Automation (read any window's elements)
│   ├── vision.rs         # Send screenshots to vision LLM for understanding
│   ├── files.rs          # Read any file type (PDF, Word, Excel, images, CSV)
│   └── ocr.rs            # Fallback OCR when UI Automation not available
│
├── hands/          # Action system
│   ├── input.rs          # Mouse + keyboard via Windows SendInput API
│   ├── windows.rs        # Window management (open, close, resize, switch)
│   ├── browser.rs        # Browser control via Chrome DevTools Protocol
│   ├── cli.rs            # PowerShell/CMD command execution
│   ├── com.rs            # COM Automation (Office apps, native Windows APIs)
│   └── safety.rs         # Action validation, blacklist, kill switch
│
├── memory/         # Persistent memory
│   ├── store.rs          # SQLite schema + CRUD (tasks, steps, screenshots)
│   ├── screenshots.rs    # Capture, compress (WebP), store on disk
│   ├── search.rs         # Search through task history
│   └── retention.rs      # Auto-cleanup old data, configurable TTL
│
├── pipeline/       # Task orchestration
│   ├── engine.rs         # Main loop: receive → plan → execute → verify → complete
│   ├── planner.rs        # LLM-powered task planning and step decomposition
│   ├── executor.rs       # Priority execution: API → Terminal → Screen
│   ├── verifier.rs       # Check if step succeeded (screenshot comparison)
│   ├── recovery.rs       # Retry logic, fallback strategies
│   └── queue.rs          # Concurrent task queue with priority
│
├── channels/       # Communication channels
│   ├── telegram.rs       # Telegram bot (async, message splitting)
│   ├── whatsapp.rs       # WhatsApp Business API integration
│   ├── discord.rs        # Discord bot
│   └── dashboard.rs      # Tauri IPC commands for frontend
│
├── config/         # Configuration
│   ├── settings.rs       # App settings (API keys, limits, preferences)
│   ├── routing.yaml      # Model routing table
│   ├── safety.yaml       # CLI safety rules
│   └── profiles/         # Agent profile YAMLs
│
└── main.rs         # Tauri v2 app entry point
    ├── System tray (minimize to tray)
    ├── Window management
    ├── IPC command registration
    └── Startup initialization
```

---

## Pilar 1: Cerebro Inteligente (brain/)

### What the PDF says:
- Chooses the right AI for each task automatically
- Orchestrator analyzes what you need and assembles the team
- Simple task → 1 Junior (~$0.001) | Intermediate → 1 Specialist (~$0.01) | Complex → Full team (~$0.10)
- Providers: Claude, ChatGPT, Gemini, Local AI (free)
- User never chooses — it's automatic

### Implementation:

**gateway.rs** — HTTP calls to LLM providers via `reqwest`
- Anthropic: `/v1/messages` (Claude)
- OpenAI: `/v1/chat/completions` (GPT)
- Google: Gemini API
- Local: llama-cpp-rs for Llama/Mistral running on user's GPU/CPU
- Each provider implements trait `LLMProvider` with `complete()` method
- Automatic fallback: if provider fails, try next in chain

**router.rs** — Loads `routing.yaml`, maps task_type + tier → ordered model list
- Tier 1 (Junior): cheapest models first (local → haiku → gpt4o-mini → flash)
- Tier 2 (Specialist): mid-range (sonnet → gpt4o → pro)
- Tier 3 (Senior/Manager): premium (opus → gpt4o → pro)

**classifier.rs** — Analyzes input text, returns TaskType + complexity + tier
- Rule-based first (keyword matching, pattern detection)
- Optionally LLM-backed for ambiguous cases

**orchestrator.rs** — For complex tasks:
1. Send task to Manager-level LLM
2. LLM decomposes into subtasks
3. Each subtask assigned to appropriate agent level
4. Execute in parallel or sequence as needed
5. Aggregate results

**cost.rs** — Budget enforcement
- Estimate cost before each LLM call
- Per-task limit (default $1.00)
- Daily/monthly budget caps
- Alert user when approaching limits

---

## Pilar 2: Equipo de +40 Especialistas (agents/)

### What the PDF says:
- +40 professionals included: Programmer, Designer, Accountant, Marketing,
  Lawyer, Analyst, Sales, HR, DevOps, etc.
- Like having a team of professionals ready to work

### Implementation:

Each specialist is a **YAML profile** defining:
```yaml
# agents/profiles/accountant.yaml
name: Contador
category: finance
level: specialist
system_prompt: |
  Eres un contador profesional. Analizas facturas,
  procesas datos financieros, generas reportes contables.
  Eres meticuloso con los números y sigues normas contables.
tools:
  - file_read     # Read invoices, spreadsheets
  - cli           # Run calculations
  - screen        # Navigate accounting software
temperature: 0.3
max_tokens: 4096
preferred_models:
  - anthropic/claude-sonnet
  - openai/gpt-4o
```

**registry.rs** — Loads all profiles at startup, provides `get_agent(name)` and
`find_best_agent(task_description)` which uses the classifier to match tasks
to the most appropriate specialist.

**hierarchy.rs** — Defines 4 levels matching the PDF:
- Junior: Simple tasks, cheap models, single-step
- Specialist: Domain-specific, mid-range models
- Senior: Complex analysis, premium models, multi-step
- Manager: Orchestrates teams of agents, decomposes work

---

## Pilar 3: Playbooks Visuales (playbooks/)

### What the PDF says:
1. User does the task → AgentOS records screenshots of each step
2. Playbook is created → Instructions + screenshots of the complete process
3. AI repeats it alone → Every time needed, without asking

Plus Marketplace: share free, sell (70% to creator), subscriptions.

### Implementation:

**recorder.rs** — When user activates recording mode:
1. Hook into input events (mouse clicks, keyboard)
2. On each significant action: capture screenshot + log the action
3. Actions: click(x,y), type(text), key(shortcut), scroll, wait
4. Store as ordered list of `RecordedStep { screenshot, action, timestamp }`

**builder.rs** — After recording:
1. Send screenshots + actions to LLM
2. LLM generates natural language descriptions for each step
3. Bundle into PlaybookFile: `{ name, description, steps[], created_at }`
4. Save as `.aos-playbook` file (YAML + screenshots directory)

**player.rs** — To replay a playbook:
1. Load playbook steps
2. For each step:
   a. EYES: capture current screen
   b. BRAIN: compare current screen with expected (from recording)
   c. Determine if we're at the right state
   d. HANDS: execute the recorded action
   e. MEMORY: screenshot after action
   f. Verify success, retry or adapt if screen differs
3. Complete when all steps done

**marketplace.rs** — Playbook distribution:
- Package: bundle playbook + metadata into signed archive
- Publish: upload to central registry (HTTPS API)
- Browse: search/filter available playbooks
- Install: download + verify signature + register locally
- Revenue: creator gets 70% of sales (tracked by registry)

### Playbook file format:
```yaml
name: "Enviar factura por email"
description: "Abre Gmail, adjunta factura, envia al cliente"
version: 1
author: "usuario@email.com"
steps:
  - action: "open_app"
    target: "chrome"
    description: "Abrir el navegador"
    screenshot: "step_001.webp"
  - action: "navigate"
    target: "https://mail.google.com"
    description: "Ir a Gmail"
    screenshot: "step_002.webp"
  - action: "click"
    target: { x: 120, y: 340 }
    fallback_selector: "button[aria-label='Compose']"
    description: "Click en Redactar"
    screenshot: "step_003.webp"
  # ... more steps
```

---

## Pilar 4: Red Mesh Multi-PC (mesh/)

### What the PDF says:
- Multiple PCs = one team. Example: PC1 development, PC2 design, PC3 operations
- Auto-discovery on local network
- E2E encryption
- Task reassignment if PC disconnects
- Skills auto-copy between PCs
- Passwords NEVER shared between machines

### Implementation:

**discovery.rs** — LAN discovery via mDNS (`mdns-sd` crate)
- Each AgentOS instance broadcasts `_agentos._tcp.local`
- Service record includes: node_id, display_name, capabilities, version
- Automatic discovery — no manual IP configuration needed

**transport.rs** — Communication layer
- LAN: direct gRPC (`tonic` crate) between peers
- WAN: WebSocket relay server for cross-network connectivity
- Protocol: protobuf-encoded messages

**protocol.rs** — Message types:
```
TaskRequest  { task_id, description, required_skills, priority }
TaskAccept   { task_id, node_id, estimated_time }
TaskProgress { task_id, step, screenshot, percent_complete }
TaskResult   { task_id, status, output, screenshots }
SkillSync    { playbook_hash, playbook_data }
Heartbeat    { node_id, timestamp, load, active_tasks }
```

**failover.rs** — Resilience:
- Heartbeat every 10s between connected nodes
- If a node misses 3 heartbeats → mark offline
- Reassign its pending tasks to other available nodes
- When node comes back → sync missed updates

**security.rs** — Security:
- TLS for all connections
- Shared secret for mesh authentication (set during initial pairing)
- E2E encryption for task payloads (AES-256-GCM)
- Credentials (API keys) NEVER leave the local machine
- Only task descriptions and results are transmitted

**sync.rs** — Shared state:
- Playbooks sync between nodes when needed
- Task history visible from any node in the mesh
- CRM-like shared data accessible from any PC
- Conflict resolution: last-write-wins with vector clocks

---

## Priority Execution System (pipeline/executor.rs)

### What the PDF says:
> 1ro: API directa — Si el servicio tiene conexion directa, la usa. Es lo mas rapido.
> 2do: Terminal — Si no hay API, usa la linea de comandos. Rapido y confiable.
> 3ro: Pantalla — Ultimo recurso: mira la pantalla y usa mouse/teclado.
> Si uno falla, pasa al siguiente automaticamente.

### Implementation:

```rust
enum ExecutionMethod {
    Api,      // Direct HTTP call to service API
    Terminal, // PowerShell/CMD command
    Screen,   // Visual automation (capture + click)
}

async fn execute_action(action: &Action) -> Result<ActionResult> {
    // Try methods in priority order
    if let Some(api) = action.api_endpoint() {
        match execute_via_api(api).await {
            Ok(result) => return Ok(result),
            Err(_) => log("API failed, trying terminal"),
        }
    }

    if let Some(cmd) = action.cli_command() {
        match execute_via_terminal(cmd).await {
            Ok(result) => return Ok(result),
            Err(_) => log("Terminal failed, trying screen"),
        }
    }

    execute_via_screen(action).await
}
```

---

## Perception System (eyes/)

**capture.rs** — Windows Graphics Capture API
- Full screen capture
- Specific window capture
- Region capture
- Configurable FPS (1-10 for monitoring, on-demand for tasks)

**ui_auto.rs** — Windows UI Automation API (`windows-rs`)
- Read accessibility tree of any window
- Get all UI elements: buttons, text fields, labels, menus
- Get element positions, text content, state (enabled/disabled)
- This is how AgentOS "sees" without OCR — native Windows API

**files.rs** — Universal file reader
- PDF: `pdf-extract` crate or spawn system PDF reader
- Word/Excel: COM automation to open in Office and read
- Images: `image` crate + send to vision LLM
- CSV/JSON/YAML: native Rust parsers
- Any other: open with default app + read screen via UI Automation

---

## Action System (hands/)

**input.rs** — Windows SendInput API
- Mouse: move, click, double-click, drag, scroll
- Keyboard: type text, press shortcuts, hold modifiers
- Smooth movement (bezier interpolation) for natural behavior

**safety.rs** — Protection layer
- Blacklist: never run destructive commands (rm -rf, format, etc.)
- Confirmation: ask user before irreversible actions (delete, send email)
- Kill switch: Ctrl+Shift+Esc immediately stops all agent actions
- Sandbox mode: agent can only observe, not act

---

## Memory System (memory/)

### SQLite Schema:

```sql
-- Core tables
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL,           -- telegram|whatsapp|discord|dashboard|mesh
    input_text TEXT NOT NULL,
    status TEXT NOT NULL,           -- pending|planning|running|completed|failed
    agent_profile TEXT,             -- which specialist handled it
    agent_level TEXT,               -- junior|specialist|senior|manager
    total_cost REAL DEFAULT 0,
    total_tokens INTEGER DEFAULT 0,
    created_at TEXT NOT NULL,
    completed_at TEXT,
    duration_ms INTEGER
);

CREATE TABLE task_steps (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id),
    step_number INTEGER NOT NULL,
    action_type TEXT NOT NULL,      -- api_call|cli_command|screen_action|llm_call
    description TEXT,
    input_data TEXT,                -- JSON: what was sent/done
    output_data TEXT,               -- JSON: what came back
    screenshot_path TEXT,           -- path to WebP screenshot
    execution_method TEXT,          -- api|terminal|screen
    success INTEGER NOT NULL,
    duration_ms INTEGER,
    created_at TEXT NOT NULL
);

CREATE TABLE llm_calls (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id),
    step_id TEXT REFERENCES task_steps(id),
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    tokens_in INTEGER,
    tokens_out INTEGER,
    cost REAL,
    latency_ms INTEGER,
    success INTEGER NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE playbooks (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    author TEXT,
    version INTEGER DEFAULT 1,
    steps_json TEXT NOT NULL,       -- JSON array of steps
    source TEXT,                    -- local|marketplace
    installed_at TEXT NOT NULL
);

CREATE TABLE mesh_nodes (
    node_id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    last_seen TEXT NOT NULL,
    capabilities TEXT,              -- JSON array of skill names
    status TEXT NOT NULL            -- online|offline
);

-- Indexes
CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_created ON tasks(created_at);
CREATE INDEX idx_steps_task ON task_steps(task_id);
CREATE INDEX idx_llm_task ON llm_calls(task_id);
```

**screenshots.rs** — Screenshot storage:
- Location: `~/.agentos/captures/{YYYY-MM-DD}/{task_id}/`
- Format: WebP (80% quality, ~50KB per screenshot vs ~500KB PNG)
- Retention: 30 days default, configurable
- Cleanup: background task removes expired screenshots

---

## Frontend (React + TypeScript + Tailwind in Tauri v2 WebView)

### Tabs (aligned to PDF functionality):

| Tab | Purpose |
|-----|---------|
| Home | Agent status, active tasks, quick stats |
| Chat | Talk to agent in natural language |
| Tasks | History with step-by-step screenshots |
| Board | Kanban view of active multi-step tasks |
| Specialists | Browse +40 agent profiles, see capabilities |
| Playbooks | Browse, record, import playbooks |
| Mesh | See connected PCs, send cross-PC tasks |
| Marketplace | Browse, install, publish playbooks |
| Analytics | Costs, token usage, task success rates |
| Settings | API keys, model preferences, budgets, kill switch |

### Tauri v2 IPC Commands:
All frontend↔backend communication via type-safe Tauri v2 commands.
No stdin/stdout, no JSON-RPC hacks, no subprocess bridge.

---

## Key Rust Crates

| Purpose | Crate | Why |
|---------|-------|-----|
| Async runtime | `tokio` | Industry standard async for Rust |
| HTTP client | `reqwest` | LLM API calls, marketplace, relay |
| SQLite | `rusqlite` | Embedded DB, zero config |
| Windows APIs | `windows-rs` | Screen capture, UI Automation, SendInput |
| Local LLM | `llama-cpp-rs` | Run Llama/Mistral locally |
| Screen capture | `win-screenshot` | Windows Graphics Capture |
| Input simulation | `enigo` | Cross-platform mouse/keyboard |
| gRPC | `tonic` | Mesh communication |
| mDNS | `mdns-sd` | LAN peer discovery |
| Serialization | `serde` + `serde_json` + `serde_yaml` | Config, data, IPC |
| Image processing | `image` + `webp` | Screenshot compression |
| Encryption | `aes-gcm` + `rustls` | E2E encryption, TLS |
| Logging | `tracing` | Structured logging |
| Desktop app | `tauri` v2 | Window, tray, WebView, IPC |
| CLI parsing | `clap` | Advanced mode CLI interface |
| Telegram | `teloxide` | Telegram bot in Rust |
| Browser control | `chromiumoxide` | CDP for Chrome/Edge |

---

## Distribution

### Installer:
- Windows: `.msi` via WiX or NSIS (bundled in Tauri v2)
- Single file, ~20-30MB
- Installs to `C:\Program Files\AgentOS\`
- Creates Start Menu shortcut
- Registers as Windows Service for background operation
- System tray icon for quick access

### First Run:
1. Wizard opens: "Welcome to AgentOS"
2. Step 1: Enter API key (Anthropic/OpenAI/Google) OR download local model
3. Step 2: Connect messaging (Telegram QR code, optional)
4. Done. Agent starts working.

### Updates:
- Auto-update via Tauri v2 updater (checks on startup, silent background update)
- Rollback if update fails

---

## What gets deleted from current codebase

Everything in `agentos/` (Python) is replaced by Rust.
Everything in `src-tauri/` is rewritten for Tauri v2.
Frontend (`frontend/`) is kept and evolved — it's already React/TS/Tailwind.
All documentation (`.md` files) stays as reference.

---

## Implementation phases (high level)

### Phase 1: Foundation (Sprint 1-3)
- Tauri v2 scaffold + React frontend migration
- SQLite schema + basic CRUD
- LLM Gateway (reqwest → Anthropic/OpenAI/Google)
- Basic pipeline: receive → classify → LLM → respond
- Dashboard IPC commands
- Working app that can chat with LLMs

### Phase 2: PC Control (Sprint 4-6)
- Screen capture (Windows Graphics Capture)
- UI Automation (read any window)
- Input simulation (mouse + keyboard)
- CLI executor (PowerShell)
- Priority execution: API → Terminal → Screen
- Safety layer

### Phase 3: Intelligence (Sprint 7-9)
- Agent hierarchy (Junior → Manager)
- 40+ specialist profiles
- Task decomposition + orchestration
- Cost control + budgets
- Local model support (llama-cpp-rs)

### Phase 4: Playbooks (Sprint 10-12)
- Action recorder
- Playbook builder (LLM-assisted)
- Playbook player (replay with vision verification)
- Playbook file format

### Phase 5: Mesh Network (Sprint 13-15)
- mDNS discovery
- gRPC transport
- Task distribution protocol
- Failover + reassignment
- E2E encryption
- Skill sync

### Phase 6: Channels + Marketplace (Sprint 16-18)
- Telegram bot (teloxide)
- WhatsApp Business API
- Discord bot
- Marketplace API (publish, browse, install)
- Payment integration for paid playbooks

### Phase 7: Polish + Distribution (Sprint 19-20)
- MSI installer
- Auto-updater
- Onboarding wizard
- Performance optimization
- Security audit
