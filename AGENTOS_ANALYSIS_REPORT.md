# AgentOS - Complete Technical Analysis Report

**Generated:** 2026-04-01
**Branch:** master (latest — all feature branches merged)
**Last Commit:** 2026-03-31 — "docs: GTM validation phase A-F"

---

## 1. EXECUTIVE SUMMARY

AgentOS is a **desktop-first AI agent orchestration platform** built on **Tauri v2 (Rust backend) + React 18 (TypeScript frontend)**. It enables users to run teams of AI agents locally on their PC with multi-provider LLM support, automated workflows, secure credential management, and enterprise-grade trust/permissions.

**Key differentiators:**
- Local-first architecture — data stays on user's machine (SQLite embedded)
- Multi-platform: Desktop (Windows/macOS), Mobile (iOS/Android), Browser Extension, Python SDK
- 40+ specialized agent profiles with cost-optimized routing
- Creator economy with marketplace for playbooks, plugins, personas
- Enterprise features: SSO, audit logs, department quotas, SCIM provisioning

---

## 2. ARCHITECTURE OVERVIEW

```
┌──────────────────────────────────────────────────────────────────┐
│                     MULTI-PLATFORM CLIENTS                       │
│                                                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────┐  ┌───────────┐ │
│  │ Desktop App │  │ Mobile App  │  │ Browser  │  │ Python    │ │
│  │ Tauri+React │  │ React Native│  │Extension │  │ SDK       │ │
│  │ (Win/Mac)   │  │ Expo (iOS/  │  │ Chrome   │  │           │ │
│  │             │  │  Android)   │  │ MV3      │  │           │ │
│  └──────┬──────┘  └──────┬──────┘  └────┬─────┘  └─────┬─────┘ │
│         │                │              │               │       │
│         └────────────────┴──────┬───────┴───────────────┘       │
│                                 │                                │
│                    ┌────────────▼────────────┐                   │
│                    │  RUST BACKEND (Tauri v2) │                   │
│                    │  60+ modules             │                   │
│                    │  REST API (Axum :8080)   │                   │
│                    │  WebSocket support       │                   │
│                    │  Tauri IPC bridge        │                   │
│                    └────────────┬────────────┘                   │
│                                 │                                │
│                    ┌────────────▼────────────┐                   │
│                    │  SQLite (embedded, WAL) │                   │
│                    │  Encrypted Vault        │                   │
│                    └────────────┬────────────┘                   │
│                                 │                                │
│              ┌──────────────────┼──────────────────┐             │
│              ▼                  ▼                  ▼             │
│         Anthropic          OpenAI/GPT       Google/Gemini        │
│         Claude             4o, 4o-mini      Flash, Pro           │
│                                                                  │
│                        Ollama (local)                            │
│                        Llama3, Mistral                           │
└──────────────────────────────────────────────────────────────────┘
```

---

## 3. TECHNOLOGY STACK

| Layer | Technology | Version |
|-------|-----------|---------|
| **Frontend** | React + TypeScript | 18.2.0 |
| **Build Tool** | Vite | 5.1.6 |
| **Styling** | Tailwind CSS | 3.4.1 |
| **Charts** | Recharts | 3.8.1 |
| **Animations** | Framer Motion | 12.38 |
| **Icons** | Lucide React | 1.7 |
| **Desktop Framework** | Tauri | 2.x |
| **Backend Language** | Rust | 2021 edition |
| **Web Framework** | Axum | 0.7 |
| **Async Runtime** | Tokio | 1.x (full) |
| **Database** | SQLite (rusqlite) | 0.32 |
| **Encryption** | AES-256-GCM + PBKDF2 | — |
| **Mobile** | React Native + Expo | 0.74.5 / 51.0 |
| **Browser Extension** | Chrome Manifest V3 | 3.1.0 |
| **Python SDK** | Custom | — |

### Key Rust Dependencies
- `axum` + `tower` + `tower-http` — HTTP server with CORS
- `tokio-tungstenite` — WebSocket
- `reqwest` — HTTP client (LLM API calls)
- `serde` + `serde_json` + `serde_yaml` — Serialization
- `aes-gcm` + `pbkdf2` + `sha2` — Cryptography
- `chrono`, `uuid`, `regex`, `base64`, `zip`, `image` — Utilities
- `tracing` + `tracing-subscriber` — Observability
- `windows` (Win32 API) — Windows-specific platform integration

---

## 4. RUST BACKEND MODULES (60+)

The core backend at `src-tauri/src/` contains 258 Rust files organized into:

| Domain | Modules | Description |
|--------|---------|-------------|
| **Agent System** | agents, personas, chains, swarm, reasoning, predictions | Agent profiles, selection, multi-agent orchestration |
| **Automation** | automation, playbooks, workflows, templates, recording | Task recording, playback, scheduled triggers |
| **PC Control** | eyes (vision), hands (control), terminal, recording | Screen capture, keyboard/mouse automation |
| **LLM Integration** | brain, gateway, classifier | Multi-provider routing, cost optimization |
| **Security** | vault, security, sandbox, approvals | AES-256 vault, permission system, command sandboxing |
| **Enterprise** | enterprise, compliance, users, teams, approvals, escalation | Org management, SSO, audit, quotas, SCIM |
| **Data** | knowledge, memory, brain, training, conversations | Knowledge base, conversation history |
| **Analytics** | analytics, metrics, observability, monitoring, billing, revenue | Usage tracking, cost analytics |
| **Marketplace** | economy, marketplace, plugins, widget/widgets | Creator studio, plugin system |
| **Communications** | integrations, webhooks, voice, email_client | Discord, Telegram, WhatsApp, Gmail, Calendar |
| **Networking** | mesh, federated, multimodal, ondevice, offline | Agent mesh, P2P networking |
| **Growth** | growth, partnerships, ipo | Partner registry, distribution bundles |
| **Autonomous** | autonomous/* | compliance, data_entry, inbox, procurement, qa, reconciliation, reporting, scheduling, support |
| **API** | api (routes, auth, server) | REST API, API key management, Stripe webhooks |
| **Config** | config (settings, routing, levels, cli_safety) | YAML-driven configuration |

---

## 5. DATABASE SCHEMA (SQLite)

### Core Tables

```sql
-- Task execution tracking
tasks (id, source, input_text, output_text, status, task_type, tier,
       model_used, provider, tokens_in, tokens_out, cost, duration_ms,
       created_at, completed_at)

-- Step-by-step task execution records
task_steps (id, task_id FK, step_number, action_type, description,
            screenshot_path, execution_method, success, duration_ms)

-- LLM API call logging
llm_calls (id, task_id FK, provider, model, tokens_in, tokens_out,
           cost, latency_ms, success, created_at)

-- Multi-agent chain orchestration
chains (id, original_task, status, total_cost, created_at, completed_at)
chain_subtasks (id, chain_id, seq, description, status, agent_name,
                model, progress, message, cost, duration_ms, output)
chain_log (id, chain_id, timestamp, agent_name, agent_level,
           event_type, message, metadata JSON)

-- Automation triggers (cron-based)
triggers (id, name, trigger_type, config JSON, task_text, enabled,
          last_run, created_at)

-- Billing enforcement
daily_usage (date PK, tasks_count, tokens_used, plan_type)

-- API authentication
api_keys (id, name, key UNIQUE, created_at, last_used, enabled)

-- Permissions & trust
permission_grants (id, user_id, org_id, agent_name, capability,
                   granted_by, granted_at, expires_at)

-- Enterprise
organizations, org_members, audit_log, department_quotas

-- Creator economy
creator_projects, creator_analytics, reputation_scores

-- Human handoffs
handoff_cases (id, task_id, chain_id, reason, status, context JSON,
               human_notes, assigned_to, audit_trail JSON)
```

### Database Configuration
- WAL mode enabled for concurrent reads
- Foreign keys enforced
- Indexed on: task status, creation date, chain IDs, trigger enabled status

---

## 6. API ENDPOINTS

**Base:** `http://localhost:8080` | **Auth:** Bearer token (`aos_*` format)

| Method | Endpoint | Auth | Description |
|--------|----------|------|-------------|
| GET | `/health` | No | Server status & version |
| GET | `/v1/status` | Yes | Agent status + queued task count |
| POST | `/v1/message` | Yes | Submit a task (text input) |
| GET | `/v1/tasks?limit=N&status=X` | Yes | List tasks with filters |
| GET | `/v1/task/:id` | Yes | Get specific task result |
| POST | `/webhooks/stripe` | Signature | Stripe billing webhook |

### API Key Format
- Pattern: `aos_{32-char-hex}` (e.g., `aos_a1b2c3d4e5f6...`)
- Management: create, list, revoke via IPC commands

### Stripe Webhook Events
- `checkout.session.completed` → upgrade plan
- `customer.subscription.deleted/canceled` → downgrade to free
- `customer.subscription.updated` → update plan if canceled/unpaid

---

## 7. AGENT SYSTEM

### Agent Hierarchy

| Level | Tier | Cost/Call | Max Tokens | Temperature | Tools |
|-------|------|-----------|------------|-------------|-------|
| Junior | 1 | ~$0.001 | 1,024 | 0.3 | CLI only |
| Specialist | 2 | ~$0.01 | 4,096 | 0.5 | CLI, Files |
| Senior | 2 | ~$0.05 | 8,192 | 0.7 | CLI, Screen, Files |
| Manager | 3 | ~$0.10 | 8,192 | 0.7 | CLI, Screen, Files, Network |
| Orchestrator | 3 | — | — | — | All tools |

### 40+ Specialized Agent Profiles

| Category | Agents |
|----------|--------|
| **Development** | Programmer, Code Reviewer, DevOps, Sysadmin, QA Tester |
| **Data** | Data Analyst, ML Engineer, Database Admin |
| **Finance** | Financial Analyst, Accountant |
| **Business** | Project Manager, Product Manager, Business Analyst, Strategy Consultant |
| **Creative** | Designer, Content Writer, UX Researcher, Copywriter |
| **Legal** | Legal Analyst, Compliance Officer |
| **IT** | Security Analyst, Network Engineer, Cloud Architect |
| **Research** | Research Analyst, Academic Researcher |
| **Communication** | PR Specialist, Community Manager, Customer Support |
| **Automation** | Automation Engineer, RPA Developer |
| **Education** | Tutor, Curriculum Designer |
| **Operations** | Operations Manager, Supply Chain Analyst, HR Specialist |

Each agent has:
- Specialized system prompt (100-200+ words with domain best practices)
- Keyword matching (English + Spanish)
- Tool access based on level
- LLM fallback for ambiguous task routing

### LLM Provider Routing

| Provider | Models | Cost (input/output per M tokens) |
|----------|--------|----------------------------------|
| **Anthropic** | Haiku, Sonnet, Opus | $3-$75 |
| **OpenAI** | GPT-4o-mini, GPT-4o | $0.15-$10 |
| **Google** | Flash, Pro | $0.10-$5 |
| **Local (Ollama)** | Llama3, Mistral | Free |

Routing tiers optimize cost based on task complexity (text, code, vision, generation, data).

---

## 8. SECURITY MODEL

### Credential Vault
- **Encryption:** AES-256-GCM (authenticated encryption)
- **Key Derivation:** PBKDF2-HMAC-SHA256 (600,000 iterations)
- **Storage:** Encrypted file at `~/.agentos/vault.enc`
- **Managed Secrets:** API keys for Anthropic, OpenAI, Google, Telegram, WhatsApp, Discord, Stripe, Google OAuth

### CLI Safety
- **Blocked commands:** `rm -rf /`, `mkfs`, fork bombs, etc.
- **Regex filtering** for dangerous patterns
- **Execution limits:** 300s timeout, 1MB output, max 5 concurrent
- **Environment stripping:** API keys/tokens never passed to child processes

### Permission System
- **8 capabilities:** VaultRead, VaultWrite, TerminalExecute, SandboxManage, PluginManage, PluginExecute, ShellExecute, VaultMigrate
- **Risk classification:** Low → Medium → High → Critical
- **Trust boundaries:** Secret, System, Network, Containment, Extension zones
- **Approval workflow:** Request → Pending → Approved/Rejected/Modified/Timeout
- **Audit trail** on every permission decision

---

## 9. BILLING & PLANS

| Feature | Free | Pro | Team |
|---------|------|-----|------|
| Tasks/day | 20 | 500 | Unlimited |
| Tokens/day | 50K | 2M | Unlimited |
| Mesh nodes | 1 | 5 | 50 |
| Triggers | No | Yes | Yes |
| Marketplace | No | Browse | Full access |
| Audit logs | No | No | Yes |
| Department quotas | No | No | Yes |
| SSO/SCIM | No | No | Yes |

- **Stripe integration** for checkout, billing portal, subscription management
- **Daily usage tracking** with enforcement at operation level

---

## 10. FRONTEND UI

### Pages (11 main views)

| Page | Description |
|------|-------------|
| **Home** | Dashboard with KPIs (tasks/tokens/cost today), suggestions, quick message, recent tasks |
| **Chat** | Conversational AI interface with code blocks, subtask expansion, feedback buttons |
| **Board** | Kanban (Queued → In Progress → Review → Done) with chain timeline and agent log |
| **Playbooks** | Record, play, manage automated task sequences |
| **Mesh** | Network visualization — local and remote agent nodes |
| **Analytics** | Charts: tasks over time, cost by provider, task distribution |
| **Developer** | Debug traces, shell registration, pending invocations |
| **Triggers** | Cron-based scheduled task management |
| **Settings** | Provider keys, permissions toggles, agent config, channel status |
| **Feedback** | Ratings and model performance insights |
| **Handoffs** | Human escalation cases: pending, assigned, resumed, completed |

### Components (20+ reusable)
- **UI Primitives:** Button (primary/secondary/danger), Input (with password toggle), Card, Toggle
- **Data Display:** StatCard (with sparkline), ChatBubble (with metadata), CodeBlock (with copy), TaskBoardCard, ChainTimeline, AgentLogPanel
- **States:** SkeletonLoader, ErrorState, EmptyState
- **Badges:** AgentLevelBadge, TierBadge, PermissionBadge, SectionLabel, StarRating

### Design System
- **Theme:** Dark-only, cyan accent (#00E5E5)
- **Backgrounds:** #0A0E14 (primary), #0D1117 (surface), #080B10 (deep)
- **Fonts:** Inter (UI), JetBrains Mono (code)
- **Animations:** pulse-cyan, bounce-dot (typing), shimmer (skeleton), fade-in
- **State management:** React hooks only (useState, useEffect, useCallback, useRef)
- **Backend bridge:** `useAgent()` custom hook → Tauri IPC invoke

---

## 11. CREATOR ECONOMY & MARKETPLACE

### Creator Studio
- **Project Types:** Playbooks, Personas, Plugins, Templates
- **Lifecycle:** Draft → Published → Archived
- **Validation:** Must pass automated tests before publishing
- **Packaging:** Bundled with metadata for distribution
- **Analytics:** Views, trials, hires, revenue, ratings per project

### Reputation System
- Builder trust/reliability score
- Community ratings
- Affiliation and hiring monetization

---

## 12. ENTERPRISE FEATURES

- **Organization management** with member roles
- **Audit logging** with event type filtering and details JSON
- **SSO integration** (SAML/OAuth)
- **Department quotas** for token/task budgets
- **SCIM provisioning** for user lifecycle management
- **Compliance automation** with retention policies

---

## 13. ESCALATION & HUMAN HANDOFF

- **Auto-escalation triggers:** confidence < 0.3, retries > 3, financial/auth/system tasks
- **Escalation reasons:** LowConfidence, RepeatedRetries, FinancialAction, MissingCredentials, SystemUnavailable, UserRequest
- **Handoff workflow:** PendingHandoff → AssignedToHuman → Resumed → CompletedByHuman
- **Context bundling:** task_id, chain_id, original_input, all steps, evidence, human notes, audit trail

---

## 14. NETWORKING & MESH

- **Agent-to-Agent Protocol (AAP)** on port 8081
- **Cloud Mesh Relay** via `relay.agentos.ai`
- **Mesh networking** with node discovery and health monitoring
- **P2P capabilities** for distributed agent execution
- **Port 9090** for mesh communication (configurable via `MESH_PORT`)

---

## 15. COMMUNICATION CHANNELS

| Channel | Integration Type |
|---------|-----------------|
| **Discord** | Bot with token auth |
| **Telegram** | Bot with webhook |
| **WhatsApp** | Business API (phone number ID, access token) |
| **Gmail** | OAuth2 (client ID/secret, refresh token) |
| **Google Calendar** | OAuth2 integration |
| **Voice** | Text-to-speech with configurable language, rate, volume |

---

## 16. AUTONOMOUS WORKFLOWS

Pre-built automation modules:
- `compliance_auto` — Automated compliance checks
- `data_entry` — Structured data input automation
- `inbox` — Email/message processing
- `procurement` — Purchase order automation
- `qa` — Quality assurance testing
- `reconciliation` — Data reconciliation
- `reporting` — Automated report generation
- `scheduling` — Calendar/task scheduling
- `support` — Customer support automation

---

## 17. CONFIGURATION SYSTEM

### YAML Configuration Files (`config/`)

- **`routing.yaml`** — LLM provider routing with cost tiers
- **`levels.yaml`** — Agent hierarchy: junior → specialist → senior → manager → orchestrator
- **`cli_safety.yaml`** — Blocked commands, regex filters, execution limits

### Settings (`~/.agentos/config.json`)
40+ configurable fields covering:
- API keys (Anthropic, OpenAI, Google)
- Execution limits (cost, timeout, concurrency)
- Billing (Stripe keys, plan type)
- Channels (Discord, Telegram, WhatsApp tokens)
- Compliance (retention days, auto-delete, analytics opt-in)
- Voice (language, rate, volume)
- Mesh (relay URL, auth token)
- Local LLM (Ollama URL, model selection)

---

## 18. MOBILE APP

- **Framework:** React Native 0.74.5 + Expo 51
- **Navigation:** React Navigation (stack-based)
- **API Client:** Connects to desktop REST API over LAN
- **Default Host:** `http://192.168.1.100:8080`
- **Features:** Health check, send message, poll task results, agent status
- **Bundle IDs:** `com.agentos.mobile` (iOS + Android)

---

## 19. BROWSER EXTENSION

- **Chrome Manifest V3**
- **Version:** 3.1.0
- **Features:** Right-click context menu (summarize, translate, explain, send to agent)
- **Permissions:** activeTab, contextMenus, storage
- **Communication:** Connects to desktop app at `localhost:8080`

---

## 20. PYTHON SDK

Located at `sdk/python/` — provides programmatic access to AgentOS capabilities for external scripts and integrations.

---

## 21. PROJECT METRICS

| Metric | Value |
|--------|-------|
| Rust source files | 258 |
| Frontend TypeScript files | 45+ |
| Rust backend modules | 60+ |
| Agent profiles | 40+ |
| UI pages | 11 |
| UI components | 20+ |
| Database tables | 15+ |
| API endpoints | 6 |
| Configuration fields | 40+ |
| Supported LLM providers | 4 (Anthropic, OpenAI, Google, Local) |
| Communication channels | 5 (Discord, Telegram, WhatsApp, Gmail, Voice) |
| Autonomous workflow modules | 9 |

---

## 22. GTM & READINESS STATUS

The latest commits include GTM (Go-To-Market) validation documentation:
- Test plans and demo preparation
- Commercial narrative and spec folders
- Real state audit with demo fixtures
- Setup guides for deployment
- Partner certification tracking
- Investor metrics (ARR, MRR, burn rate, runway)
- Infrastructure health monitoring (regional latency, uptime)
- Data room documentation for due diligence

---

*Report generated from master branch analysis — all codex/* feature branches fully merged.*
