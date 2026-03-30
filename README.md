# AgentOS

**Your AI team, running on your PC.**

AgentOS is a desktop AI agent that controls your screen, runs commands, automates workflows, and connects to any service — all through simple chat.

## Features

- **Vision Control** — Sees your screen and controls mouse + keyboard
- **PowerShell Integration** — Executes commands natively
- **Smart Playbooks** — Record and replay with variables, conditionals, loops
- **Agent Chains** — Complex tasks auto-decomposed into subtasks
- **Multi-PC Mesh** — Connect multiple PCs in a network
- **Plugin System** — Extend with custom plugins
- **WhatsApp & Telegram** — Control via messaging apps
- **Local LLMs** — Run with Ollama for free
- **Public API** — REST API on port 8080
- **Agent Protocol (AAP)** — Open agent-to-agent communication

## Quick Start

1. Download from [Releases](https://github.com/yourusername/agentos/releases)
2. Install and open AgentOS
3. Add your API key (Anthropic, OpenAI, or Ollama)
4. Type: `Check my disk space`

## Build from Source

Prerequisites: Rust 1.75+, Node.js 18+, pnpm

```bash
git clone https://github.com/yourusername/agentos
cd agentos
pnpm install
cargo tauri dev
```

## Architecture

| Layer | Technology |
|-------|-----------|
| Frontend | React 18 + TypeScript + Tailwind CSS |
| Backend | Rust (Tauri v2) |
| Database | SQLite (embedded) |
| Mobile | React Native + Expo |
| AI | Anthropic Claude, OpenAI GPT, Ollama |

## Documentation

- [Getting Started](docs/getting-started.md)
- [API Reference](docs/api-reference.md)
- [API Playground](docs/api-playground.html)
- [Python SDK](sdk/python/)

## Plans

| | Free | Pro ($19/mo) | Team ($49/mo) |
|---|---|---|---|
| Tasks/day | 20 | 500 | Unlimited |
| Triggers | - | Yes | Yes |
| Mesh nodes | 1 | 5 | 50 |
| Audit log | - | - | Yes |

## Security

- AES-256-GCM encrypted credential vault
- Command execution sandboxing
- Input sanitization
- API rate limiting
- GDPR compliance (data export/erasure)

## License

MIT

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.
