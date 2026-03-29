# AgentOS -- Intellectual Property Inventory

## Proprietary Software Assets

### Core Engine (Rust)
| Asset | Description | Status |
|-------|-------------|--------|
| LLM Gateway | Multi-provider routing (Anthropic, OpenAI, Ollama) | Production |
| Task Pipeline | Decomposition, chaining, parallel execution | Production |
| Vision System | Screen capture, UI element detection, coordinate scaling | Production |
| Input System | Mouse/keyboard automation via Windows SendInput | Production |
| Mesh Networking | UDP discovery + TCP task transport across PCs | Production |
| Vault | AES-256-GCM encrypted credential storage | Production |
| Plugin System | Manifest-based extensible plugin architecture | Production |
| Marketplace | Package catalog, install/uninstall, reviews | Production |
| Automation Engine | Cron scheduler, event triggers | Production |
| Web Browsing | Page fetch, content extraction, DuckDuckGo search | Production |
| Security Layer | Sandbox, rate limiter, input sanitizer | Production |
| Compliance Module | GDPR data export/erasure, retention policies | Production |
| Enterprise Module | Audit log, org management, SSO stub | Production |
| Billing System | Plan management, Stripe integration | Production |
| Analytics Engine | ROI calculator, heatmaps, period comparison, export | Production |
| Feedback System | Thumbs up/down, weekly AI-generated insights | Production |
| Business Metrics | Acquisition-ready dashboard data | Production |

### Frontend (TypeScript/React)
| Asset | Description | Status |
|-------|-------------|--------|
| Dashboard UI | Main agent interface, chat, task management | Production |
| Analytics Views | ROI charts, heatmaps, period comparison | Production |
| Settings Panel | Provider config, plan management, security | Production |
| Onboarding Flow | Welcome wizard, API key setup | Production |
| i18n System | Multi-language support framework | Production |
| Design System | Custom Tailwind theme, component library | Production |

### Mobile (React Native)
| Asset | Description | Status |
|-------|-------------|--------|
| Mobile App Shell | Expo-based companion app | In Progress |

## Trade Secrets
- LLM task classification algorithm (brain/classifier)
- Agent specialist selection and prompt engineering (agents/)
- Cost optimization routing across providers
- Mesh task distribution heuristics

## Data Assets
- Plugin/playbook marketplace catalog schema
- User feedback corpus (for insight generation)
- Task execution patterns and analytics

## Open Source Dependencies
All production dependencies use permissive licenses:
- **MIT:** tauri, serde, tokio, reqwest, axum, uuid, base64, image
- **Apache-2.0:** rusqlite, chrono
- **MIT/Apache-2.0 dual:** aes-gcm, pbkdf2, sha2, zip
- **No GPL dependencies** in production build

## Domain Assets
| Asset | Type | Status |
|-------|------|--------|
| AgentOS name | Product name | Active |
| Logo and brand assets | Visual identity | Active |

## Registered Trademarks
- None currently filed (recommended pre-acquisition)

## Patent Opportunities
- Multi-PC mesh task distribution protocol
- LLM cost optimization routing algorithm
- Desktop automation with AI vision pipeline
