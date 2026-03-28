# UX Design: AOS-022 a AOS-027 — Setup Wizard y Dashboard

**Tickets:** AOS-022 (Wizard), AOS-023 (Tray), AOS-024-027 (Dashboard)
**Rol:** UX/UI Designer
**Fecha:** Marzo 2026

---

## Design system

### Colores

| Token | Valor | Uso |
|-------|-------|-----|
| `--bg-primary` | `#0a0a0f` | Fondo principal |
| `--bg-secondary` | `#12121a` | Cards, paneles |
| `--bg-tertiary` | `#1a1a2e` | Hover, inputs |
| `--accent-purple` | `#8b5cf6` | Primary actions, brand |
| `--accent-purple-hover` | `#7c3aed` | Hover de primary |
| `--text-primary` | `#f1f1f1` | Texto principal |
| `--text-secondary` | `#9ca3af` | Texto secundario |
| `--text-muted` | `#6b7280` | Texto dimmed |
| `--success` | `#22c55e` | Éxito, completado |
| `--error` | `#ef4444` | Error |
| `--warning` | `#f59e0b` | Warning, en progreso |
| `--border` | `#2a2a3e` | Bordes de cards |

### Typography

- Font: `Inter` (system fallback: `-apple-system, sans-serif`)
- H1: 24px semibold
- H2: 18px semibold
- Body: 14px regular
- Small: 12px regular
- Mono (code): `JetBrains Mono`, `monospace` — 13px

### Layout

- Ventana mínima: 800×600
- Ventana default: 1024×700
- Sidebar: 240px fija
- Content: el resto

---

## AOS-022 — Setup Wizard

### Estructura

```
┌─────────────────────────────────────────────────┐
│                                                  │
│                   [Logo AgentOS]                  │
│                                                  │
│  ● ○ ○ ○ ○    (progress dots — paso actual)     │
│                                                  │
│  ┌────────────────────────────────────────────┐  │
│  │                                            │  │
│  │          [Contenido del paso]              │  │
│  │                                            │  │
│  └────────────────────────────────────────────┘  │
│                                                  │
│              [Back]           [Next →]            │
│                                                  │
└─────────────────────────────────────────────────┘
```

### Paso 1: Welcome
- Logo grande
- "Welcome to AgentOS"
- "Your AI agent, running on your PC. Private, powerful, yours."
- Botón: "Get Started →"

### Paso 2: AI Provider
- Dos opciones con radio buttons:
  - **Managed Plan** (default, selected): "We handle everything. Pay per use."
    - Texto: "No API keys needed. Start immediately."
  - **Bring Your Own Keys**: "Use your own API keys. Full control."
    - Se expande: campos para Anthropic, OpenAI, Google keys
    - Cada campo: input type=password + botón "Test" (icono ✓ o ✗)
    - El "Test" llama a `health_check` via IPC

### Paso 3: Messaging
- "Connect your messaging platforms"
- Telegram: campo para bot token + botón "Test Connection"
  - Link: "How to create a Telegram bot" (abre browser)
- WhatsApp: "Coming soon" (disabled)
- Discord: "Coming soon" (disabled)
- Skip button: "I'll set this up later"

### Paso 4: Permissions
- "What can AgentOS do on your PC?"
- 4 toggles con explicación:
  - ☑ **Run terminal commands** — "Execute shell commands (ls, git, pip, etc.)"
  - ☐ **Control the screen** — "Move mouse, type, click (for visual automation)"
  - ☐ **Access files** — "Read and write files on your computer"
  - ☐ **Network access** — "Make HTTP requests to external services"
- Nota: "You can change these anytime in Settings."

### Paso 5: Finish
- Barra de progreso animada: "Setting up your agent..."
  - ✓ Initializing database
  - ✓ Connecting to AI providers
  - ✓ Starting Telegram bot (if configured)
  - ✓ Loading default playbooks
- Cuando completa: "🎉 You're all set!"
- Botón: "Open Dashboard →"

---

## AOS-023 — System Tray

### Estados del ícono

| Estado | Color | Tooltip |
|--------|-------|---------|
| Idle | Gris (#6b7280) | "AgentOS — Idle" |
| Working | Violeta pulsante (#8b5cf6) | "AgentOS — Processing task..." |
| Error | Rojo (#ef4444) | "AgentOS — Error (click for details)" |

### Menú (click derecho)

```
┌───────────────────────┐
│ ▶ Start Agent         │  (o "⏸ Pause Agent")
│ ─────────────────     │
│ 📊 Open Dashboard     │
│ 📋 Recent Tasks   →   │  (submenu: últimas 3 tareas)
│ ⚙ Settings            │
│ ─────────────────     │
│ ✖ Quit AgentOS        │
└───────────────────────┘
```

---

## AOS-024 a AOS-027 — Dashboard

### Layout

```
┌────────┬──────────────────────────────────────┐
│        │                                       │
│  SIDE  │           CONTENT AREA                │
│  BAR   │                                       │
│        │                                       │
│  🏠 Home│                                       │
│  📚 Play│                                       │
│  💬 Chat│                                       │
│  ⚙ Set │                                       │
│        │                                       │
│        │                                       │
│ ─────  │                                       │
│ v0.1.0 │                                       │
│ status │                                       │
└────────┴──────────────────────────────────────┘
```

### Home (AOS-024)

```
┌─────────────────────────────────────────┐
│ Good morning! 👋                         │
│                                          │
│ ┌──────────┐ ┌──────────┐ ┌──────────┐  │
│ │ 12       │ │ 1,240    │ │ $0.34    │  │
│ │ tasks    │ │ tokens   │ │ cost     │  │
│ │ today    │ │ used     │ │ today    │  │
│ └──────────┘ └──────────┘ └──────────┘  │
│                                          │
│ ┌─── Quick Message ───────────────────┐  │
│ │ Ask AgentOS anything...         [→] │  │
│ └─────────────────────────────────────┘  │
│                                          │
│ Recent Tasks                             │
│ ┌─────────────────────────────────────┐  │
│ │ ✅ "check disk space"  gpt-4o-mini │  │
│ │    $0.001 · 1.2s · 2 min ago       │  │
│ │ ✅ "list running processes"  haiku  │  │
│ │    $0.002 · 0.8s · 5 min ago       │  │
│ │ ❌ "install numpy"  — error        │  │
│ │    Command blocked · 8 min ago     │  │
│ └─────────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

### Playbooks (AOS-025)

```
┌─────────────────────────────────────────┐
│ Playbooks                     [+ New]   │
│                                          │
│ ┌─── Active ─────────────────────────┐  │
│ │ 📘 System Monitor                  │  │
│ │    Monitors PC health              │  │
│ │    Tier: 1 · Perms: cli           │  │
│ │    [Deactivate] [Record Steps]     │  │
│ └─────────────────────────────────────┘  │
│                                          │
│ ┌─── Installed ──────────────────────┐  │
│ │ 📗 Hello World           [Activate]│  │
│ │ 📗 Code Reviewer         [Activate]│  │
│ └─────────────────────────────────────┘  │
│                                          │
│ ┌─── Marketplace ────────────────────┐  │
│ │ 🏪 Coming in Phase 5               │  │
│ └─────────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

### Chat (AOS-026)

```
┌─────────────────────────────────────────┐
│ Chat with AgentOS                        │
│                                          │
│   ┌──────────────────────────────┐       │
│   │ 🤖 Hello! I'm AgentOS.      │       │
│   │ How can I help you?          │       │
│   └──────────────────────────────┘       │
│                                          │
│           ┌──────────────────────┐       │
│           │ Check my disk space  │ 👤    │
│           └──────────────────────┘       │
│                                          │
│   ┌──────────────────────────────┐       │
│   │ 🤖 Here's your disk usage:  │       │
│   │ ┌────────────────────────┐   │       │
│   │ │ Filesystem  Size  Used │   │ [📋]  │
│   │ │ /dev/sda1   500G  320G│   │       │
│   │ └────────────────────────┘   │       │
│   │ _haiku · $0.001 · 0.8s_     │       │
│   └──────────────────────────────┘       │
│                                          │
│ ┌─────────────────────────────────[→]─┐  │
│ │ Type a message...                    │  │
│ └──────────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

### Settings (AOS-027)

```
┌─────────────────────────────────────────┐
│ Settings                                 │
│                                          │
│ ── AI Providers ──────────────────────   │
│ Anthropic:  sk-***3xyz  [Test ✓] [Edit] │
│ OpenAI:     Not configured    [Add Key]  │
│ Google:     Not configured    [Add Key]  │
│                                          │
│ ── Messaging ─────────────────────────   │
│ Telegram:   Connected ✅      [Disconnect]│
│ WhatsApp:   Coming soon                  │
│                                          │
│ ── Permissions ───────────────────────   │
│ ☑ Terminal commands                      │
│ ☐ Screen control                         │
│ ☐ File access                            │
│ ☐ Network access                         │
│                                          │
│ ── Agent ─────────────────────────────   │
│ Default tier:    [1 - Cheap      ▼]     │
│ Max cost/task:   [$1.00          ]      │
│ Log level:       [INFO           ▼]     │
│                                          │
│ ── About ─────────────────────────────   │
│ AgentOS v0.1.0                           │
│ [Re-run Setup Wizard] [Reset All Data]   │
└─────────────────────────────────────────┘
```

---

## Nota sobre accesibilidad

- Todos los botones con focus ring visible
- Colores con contraste mínimo 4.5:1
- Keyboard navigation en wizard y dashboard
- Labels en todos los inputs
