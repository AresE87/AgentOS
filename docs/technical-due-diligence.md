# AgentOS -- Technical Due Diligence

## Architecture Overview
- **Frontend:** React 18 + TypeScript + Tailwind CSS
- **Backend:** Rust (Tauri v2 framework)
- **Database:** SQLite (embedded, no external deps)
- **Desktop:** Tauri v2 (WebView2 on Windows)
- **Mobile:** React Native + Expo

## Code Quality
- Language: Rust (memory-safe, no GC)
- Type system: Strong typing in both Rust and TypeScript
- No runtime dependencies (fully self-contained binary)
- All API keys encrypted with AES-256-GCM (PBKDF2 key derivation)

## Module Architecture
| Module | Purpose | Lines (approx) |
|--------|---------|----------------|
| brain/ | LLM gateway, classifier, local LLM | ~800 |
| pipeline/ | Task execution engine, vision, chains | ~1200 |
| eyes/ | Screen capture, coordinate scaling | ~300 |
| hands/ | Mouse/keyboard input via SendInput | ~400 |
| channels/ | Telegram, WhatsApp, webhook server | ~600 |
| mesh/ | Multi-PC networking, orchestration | ~500 |
| vault/ | AES-256-GCM encrypted credential storage | ~200 |
| marketplace/ | Package catalog, install/uninstall | ~300 |
| plugins/ | Plugin system, manifest, execution | ~400 |
| security/ | Sandbox, rate limiter, input sanitizer | ~400 |
| compliance/ | GDPR, retention, privacy controls | ~300 |
| analytics/ | ROI, heatmap, export | ~300 |
| billing/ | Plans, limits, Stripe integration | ~200 |
| enterprise/ | Audit log, org management, SSO stub | ~400 |
| api/ | Embedded axum HTTP server | ~300 |
| feedback/ | Thumbs up/down, weekly insights | ~200 |
| cache/ | In-memory TTL cache, benchmarks | ~200 |
| automation/ | Cron scheduler, triggers | ~200 |
| metrics/ | Business metrics dashboard | ~100 |
| web/ | Web browsing, page fetch, search | ~300 |

## Dependencies
### Rust (key crates)
- tauri v2, serde, tokio, reqwest, rusqlite
- aes-gcm, pbkdf2, sha2 (encryption)
- axum (embedded API server)
- image (screenshot processing)
- chrono, uuid, base64
- zip (marketplace packages)

### Frontend (key packages)
- React 18, TypeScript, Tailwind CSS
- Recharts (analytics charts)
- Lucide React (icons)
- react-i18next pattern (i18n)

## Security Posture
- AES-256-GCM vault for API keys
- Command execution sandboxing (blocked patterns, timeout, output truncation)
- Input sanitization (XSS, SQL injection, path traversal)
- API rate limiting (per-plan tiers)
- Immutable audit log
- GDPR compliance (data export/erasure)

## Scalability
- Local-first: no server required
- Mesh networking: up to 50 nodes (Team plan)
- Plugin system: extensible without core changes
- Multi-provider: Anthropic, OpenAI, Ollama (local)

## Intellectual Property
- Core engine: 100% proprietary
- UI/UX: 100% proprietary
- Open source dependencies: all MIT/Apache-2.0 licensed
- No GPL dependencies in production build
