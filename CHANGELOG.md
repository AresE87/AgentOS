# Changelog

## [8.0.0] - 2026-04-04 -- Marketing Autonomo

### Added
- M8-3: Marketing Command Center frontend dashboard with 4 tabs:
  - Overview: KPI cards (followers, engagement, posts, mentions), platform status grid, activity feed
  - Content: AI content generator modal (topic, platforms, tone), weekly plan builder
  - Menciones: unified inbox with AI-suggested replies, classification filters, reply/edit/ignore
  - Campanas: campaign CRUD, timeline, metrics dashboard
- M8-3: Marketing hooks in useAgent.ts (generateContent, generateWeeklyPlan, processMentions,
  getCalendar, schedulePost, createCampaign, getCampaign, listCampaigns)
- M8-3: Marketing tab in Dashboard sidebar with Megaphone icon
- M8-5: Self-Promotion Mode with auto-generate promotional content about AgentOS
  - `self_promotion.rs`: product context, 8 promotion topics, weekly promo generation
  - `cmd_generate_promo_content` IPC command
  - "self_promotion" coordinator mission template (4-agent DAG: Content Writer, SEO Specialist,
    Social Media Manager, Community Manager)
  - Auto-Promocion toggle in Marketing Overview tab with frequency and platform selectors

### Changed
- Version bump to 8.0.0 across Cargo.toml, frontend, and mobile packages

## [7.0.0] - 2026-04-04 -- Docker Sandbox Workers, Local AI, Distributed Swarm

### Added
- S4: Mesh Remote Workers -- deploy, exec, stop worker containers on any LAN node
  - `remote_worker.rs` coordinator module with RemoteWorkerManager
  - Worker host API routes: POST /workers/deploy, POST /workers/:id/exec, DELETE /workers/:id, GET /workers/:id/status, GET /workers/status
  - IPC commands: cmd_deploy_remote_worker, cmd_list_mesh_nodes_with_docker
  - Frontend hooks: deployRemoteWorker, listMeshNodesWithDocker
- S6: All-in-One Installer -- single-script setup for Docker Desktop + Ollama + worker image + AI models
  - `installer/setup_docker.ps1` PowerShell script (idempotent)
  - Downloads and installs Docker Desktop and Ollama if missing
  - Builds agentos-worker:latest image from Dockerfile
  - Pulls phi3:mini and llama3.2:1b local models

### Changed
- Version bump to 7.0.0 across Cargo.toml, frontend, and mobile packages
- API server now exposes mesh worker endpoints for cross-node orchestration

## [5.0.0] - 2026-04-03 -- Zero Gaps Release

### Added
- H2: Structured logging with trace ID propagation across all request paths
- H2: Trace IDs in cmd_process_message, chain, PC task, and agentic loop responses
- H4: Performance audit -- WAL mode, 20+ SQLite indexes, no unbounded collections
- AUDIT_V5.md: comprehensive final state assessment for v5.0.0 release

### Changed
- Version bump to 5.0.0 across Cargo.toml, frontend, and mobile packages
- Sessions E1-E4, F1-F4, G1-G4, H1-H4, I1-I2, J1-J2 all completed (20/20)

### Security
- 6-layer bash validator, workspace boundary enforcer, 100KB input cap
- AES-256-GCM vault with 11 secret types, secret scrubbing on persist
- API auth on all HTTP endpoints, rate limiter, injection detection
- Verified: no API keys or secrets in logs

### Verified
- cargo check: PASS (39 warnings, 0 errors)
- cargo test: 10 tests passing
- tsc --noEmit: PASS
- Binary size: 44 MB
- 46,556 lines Rust, 13,308 lines TypeScript

## [4.2.0] - 2026-03-29 — The Agent Economy

### Added
- R141: Agent Hiring — post jobs, apply, hire agents with per-task/hour/month pricing
- R142: Reputation System — agent scores, reviews, leaderboard, badge system
- R143: Cross-User Collaboration — shared project rooms, participant agents, result sharing
- R144: Microtasks Marketplace — post/claim/complete micro-services with reward tracking
- R145: Escrow — held payments, release/refund/dispute workflow for high-value tasks
- R146: Agent Insurance — Basic/Standard/Premium/Enterprise coverage, claim filing
- R147: Creator Studio — create/publish/unpublish projects (playbooks, personas, plugins, templates)
- R148: Creator Analytics — revenue history, download trends, creator metrics dashboard
- R149: Affiliate Program — referral links, click/conversion tracking, tiered commissions (10-25%)
- R150: v4.2 Economy Release

### Changed
- Version bump to 4.2.0 across Cargo.toml, frontend, and mobile packages
- New `economy` module with 9 sub-engines for agent marketplace economy
- Full creator-to-consumer pipeline: build, publish, hire, pay, insure, review

## [4.1.0] - 2026-03-29 -- Industry Verticals Pro

### Added
- R131: Legal Suite -- case management, document analysis, search across legal cases
- R132: Medical Assistant -- patient records, drug interaction checks, history summaries
- R133: Accounting Engine -- transaction tracking, balance reports, auto-categorization
- R134: Real Estate Agent -- property listings, ROI calculations, listing generation
- R135: Education Assistant -- course creation, quiz generation, grading, progress tracking
- R136: HR Manager -- employee records, offer letter generation, benefits calculation
- R137: Supply Chain Manager -- shipment tracking, route optimization, demand forecasting
- R138: Construction Manager -- project milestones, budget tracking, safety checklists
- R139: Agriculture Assistant -- crop planning, weather impact, irrigation scheduling, yield forecasting
- R140: v4.1 Industry Verticals Pro Release

### Changed
- Nine new vertical sub-modules under `verticals/` with 36 IPC commands total
- Full industry-specific domain logic: legal, medical, accounting, real estate, education, HR, supply chain, construction, agriculture
- Frontend hooks for all vertical modules in useAgent.ts

## [4.0.0] - 2026-03-29

### Added
- R121: Reasoning Chains -- step-by-step reasoning engine with chain-of-thought tracking
- R122: Self-Correction -- automatic verification and correction of reasoning outputs
- R123: Multimodal Reasoning -- cross-modal analysis combining text, image, and data evidence
- R124: Causal Inference -- causal graph construction, counterfactual analysis
- R125: Knowledge Graph -- SQLite-backed entity-relationship graph with search and traversal
- R126: Hypothesis Generation -- generate and evaluate hypotheses with Bayesian-like probability updates
- R127: Confidence Calibration -- track prediction confidence, calibration stats, auto-verify low-confidence tasks
- R128: Transfer Learning -- register learned patterns, apply across domains, track helpfulness
- R129: Meta-Learning -- domain learning curves, accuracy prediction, fastest-learning domain tracking
- R130: v4.0 Intelligent Agent Release

### Changed
- Version bump to 4.0.0 across Cargo.toml, frontend, and mobile packages
- New `reasoning` module with 8 sub-engines for intelligent decision-making
- New `knowledge` module with persistent graph storage
- Intelligence-first architecture: hypothesis-driven reasoning, confidence calibration, cross-domain transfer

## [3.2.0] - 2026-03-29

### Added
- R111: Autonomous Inbox -- keyword-based rule engine, auto-reply/forward/archive/label/escalate
- R112: Autonomous Scheduling -- calendar optimization, time-block analysis, smart slot finder
- R113: Autonomous Reporting -- scheduled report generation with configurable data sources and templates
- R114: Autonomous Data Entry -- structured data extraction from invoices, receipts, forms with validation
- R115: Autonomous QA -- test plan generation, automated check execution, coverage reporting
- R116: Autonomous Support -- L1/L2 ticket processing, knowledge-base auto-reply, SLA tracking
- R117: Autonomous Procurement -- purchase requests, auto-approval under threshold, spend summary
- R118: Autonomous Compliance -- regulatory requirement registration, automated checks, auto-remediation
- R119: Autonomous Reconciliation -- multi-source comparison, mismatch detection, auto-resolution
- R120: v3.2 Autonomous Operations Release

### Changed
- Version bump to 3.2.0 across Cargo.toml, frontend, and mobile packages
- New `autonomous` module with 9 sub-engines for end-to-end business process automation
- Estimated 120+ hours/month savings per company through autonomous operations

## [3.1.0] - 2026-03-29

### Added
- R101: AR/VR Agent -- WebXR app, passthrough camera analysis, spatial UI
- R102: Wearable Integration -- Apple Watch + Wear OS: voice commands, haptic notifications
- R103: IoT Controller -- Home Assistant, Philips Hue, Tuya: control devices by voice
- R104: Tablet Mode -- Stylus annotation, touch workflow builder, document signing
- R105: TV Display Mode -- Large display dashboard, ambient mode, team mission control
- R106: Car Integration -- Android Auto + CarPlay: hands-free briefings and voice tasks
- R107: Browser Extension -- Chrome/Firefox extension: right-click context menu, native messaging bridge
- R108: Email Client -- Full IMAP/SMTP inbox with AI triage, smart compose, follow-ups
- R109: Hardware Partnerships -- OEM installer, partner registry, certification program
- R110: v3.1 Hardware & Surfaces Release

### Changed
- Version bump to 3.1.0 across Cargo.toml, frontend, and mobile packages
- Browser extension manifest v3 stub with context menus and native messaging
- Hardware partnership documentation with tiers (Basic/Premium/Exclusive)

## [3.0.0] - 2026-03-29

### Added
- R91: OS Integration — shell context-menu file and text actions
- R92: Federated Learning — privacy-preserving weight delta client
- R93: Human Handoff — escalation detector and handoff manager
- R94: Compliance Automation — multi-framework reporter (GDPR, HIPAA, SOC2, PCI-DSS)
- R95: White-Label Org Marketplace — org-scoped agent publishing, approval, search
- R96: Agent Debugger — execution traces, step-through timeline, 8-phase tracking
- R97: Revenue Optimization — MRR/ARR metrics, churn prediction, upsell engine
- R98: Global Infrastructure — multi-region status (us-east, eu-west, ap-southeast), latency monitoring
- R99: IPO Readiness — investor dashboard, data room index, financial projections
- R100: v3.0 The Standard — AAP Protocol v2.0 specification, Open Agent Foundation charter

### Changed
- Version bump to 3.0.0 across Cargo.toml, frontend, and mobile packages
- AAP Protocol upgraded to v2.0 with streaming, conversations, swarm coordination
- Open Agent Foundation charter established for open agent standards governance

## [2.0.0] - 2026-03-29

### Added
- R81: On-Device AI — local model management, ONNX runtime, model download/load/unload
- R82: Predictive Actions — user behavior patterns, action predictions, smart suggestions
- R83: Cross-App Automation — app bridge registry, deep linking, cross-app workflows
- R84: Multi-Modal Input — image/audio/video processing pipeline, drag-and-drop support
- R85: Agent Swarm — swarm orchestrator, task decomposition, parallel agent execution
- R86: Real-time Translation — multi-language translation engine, auto-detect, LLM-powered (15+ languages)
- R87: Accessibility — high contrast mode, font scaling, screen reader hints, reduce motion, keyboard nav
- R88: Industry Verticals — 5 built-in verticals (healthcare, legal, finance, education, e-commerce)
- R89: Offline First — connectivity detection, response caching, sync queue, SQLite persistence
- R90: v2.0 Platform Release — version bump, migration guide, full changelog

### Changed
- Version bump to 2.0.0 across Cargo.toml, frontend, and mobile packages
- Platform rebrand: "Your AI workforce. On your terms."

## [1.3.0] - 2026-03-29

### Added
- R71: Visual Workflow Builder — drag-and-drop workflow canvas, node types, execution engine
- R72: Webhook Actions — inbound webhook triggers with task templates and filters
- R73: Fine-Tuning Pipeline — training data export, fine-tune job management, preview
- R74: Agent Testing — test suites, assertions, template generation, automated runs
- R75: Playbook Version Control — version history, rollback, diff, branching
- R76: Analytics Pro — funnel analysis, retention cohorts, cost forecasting, model comparison
- R77: Embeddable Agent Widget — JS snippet generator, iframe config, embed demo page
- R78: CLI Power Mode — smart terminal execution, error explanation, NL-to-command translation
- R79: Extension API V2 — plugin UI pages and widgets, method invocation, scoped storage (SQLite)
- R80: Plugin certification 5-step process, partner program (Bronze/Silver/Gold/Platinum tiers)

### Changed
- Version bump to 1.3.0 across Cargo.toml, frontend, and mobile packages

## [1.2.0] - 2026-03-29

### Added
- R69: Team Collaboration — teams, members, roles (owner/admin/member/viewer), resource sharing
- R70: v1.2 Enterprise — department quotas (budget, daily task limits, model allowlists), SCIM 2.0 user provisioning stubs

### Changed
- Version bump to 1.2.0 across Cargo.toml, frontend, and mobile packages

## [1.1.0] - 2026-03-29

### Added
- R51: Multi-Agent Conversations — conversation chains, agent messaging, review patterns
- R52: Screen Recording & Replay — frame capture, recording manager, replay metadata
- R53: Natural Language Triggers — NL parser, file watch, condition triggers
- R54: Agent Memory (RAG Local) — local memory store, keyword search, context injection
- R55: File Understanding — read/process files, temp file management
- R56: Smart Notifications — notification center, monitor checks, read/unread tracking
- R57: Collaborative Chains — user intervention, subtask actions, context injection
- R58: Template Engine — template CRUD, variable rendering, reusable templates
- R59: Agent Personas — persona profiles, default personas, custom persona management
- R60: v1.1 Growth Release — adoption metrics, sharing/referral links, version bump

### Changed
- Version bump to 1.1.0 across Cargo.toml, frontend, and mobile packages

## [1.0.0] - 2026-03-29

### Added
- Chat interface with AI-powered task execution
- Vision control (screen capture + mouse/keyboard)
- PowerShell command execution with sandboxing
- Smart Playbooks with variables, conditionals, loops
- Agent Chains for complex multi-step tasks
- Cron-based trigger automation
- Multi-PC Mesh networking with smart orchestration
- Cloud Mesh via relay server
- WhatsApp and Telegram integration
- Marketplace for playbook packages
- Plugin system with script-based execution
- Public REST API on port 8080
- Agent-to-Agent Protocol (AAP) on port 9100
- Local LLM support via Ollama
- AES-256-GCM encrypted vault
- Free/Pro/Team billing plans
- Enterprise features (audit log, org management, SSO stub)
- GDPR compliance (data export, erasure, retention)
- ROI calculator and advanced analytics
- Desktop widgets (quick task, status, notifications)
- Internationalization (English, Spanish, Portuguese)
- Structured logging and alert management
- White-label/OEM branding support
- AI training pipeline with anonymized feedback
- Python SDK
- Developer documentation and API playground
- Mobile companion app (React Native)
