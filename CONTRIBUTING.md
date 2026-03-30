# Contributing to AgentOS

## Development Setup

1. Fork and clone the repository
2. Install Rust (rustup.rs) and Node.js 18+
3. Install pnpm: `npm install -g pnpm`
4. Install dependencies: `pnpm install`
5. Run dev server: `cargo tauri dev`

## Code Style

- Rust: `cargo fmt` and `cargo clippy`
- TypeScript: ESLint + Prettier
- Commits: Conventional Commits (feat:, fix:, docs:, etc.)

## Pull Requests

1. Create a feature branch from `main`
2. Make your changes
3. Run tests: `cargo test`
4. Submit a PR with a clear description

## Plugin Development

See [Building Plugins Guide](docs/guides/building-plugins.md)

## Reporting Issues

Use GitHub Issues with the bug/feature template.
