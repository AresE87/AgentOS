# AgentOS

The universal AI agent for your PC.

## Quick Start

```bash
# 1. Clone and setup
pip install -e ".[dev]"

# 2. Configure environment
cp .env.example .env
# Edit .env with your API keys

# 3. Run tests
make test

# 4. Start the agent
make dev
```

## Development Commands

| Command | Description |
|---------|-------------|
| `make setup` | Install dependencies (dev mode) |
| `make dev` | Run the agent |
| `make test` | Run test suite |
| `make test-cov` | Run tests with coverage |
| `make lint` | Check code with ruff |
| `make format` | Format code with ruff |
| `make check` | Lint + type check |
| `make clean` | Remove cache files |

## Project Structure

```
agentos/           # Python package
  gateway/         # LLM Gateway (provider abstraction, routing, cost tracking)
  executor/        # CLI Executor (PTY, sandbox)
  context/         # Context Folder Protocol parser
  store/           # SQLite persistence
  messaging/       # Telegram bot adapter
  core/            # Agent pipeline
  utils/           # Logging, helpers
config/            # YAML configuration files
examples/          # Example playbooks
tests/             # Test suite
```
