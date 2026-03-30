# FASE R36 — SECURITY HARDENING: Auditoría y endurecimiento

**Objetivo:** El producto es seguro para que una empresa lo use sin riesgo. Sandbox mejorado, CSP estricto, dependency audit, y documentación de security model.

---

## Tareas

### 1. Tauri CSP (Content Security Policy) estricto

```json
// tauri.conf.json:
{
  "app": {
    "security": {
      "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; connect-src 'self' https://api.anthropic.com https://api.openai.com https://generativelanguage.googleapis.com"
    }
  }
}
```

### 2. Sandbox del CLI executor mejorado

```rust
// Además de la blacklist de comandos:
// 1. PowerShell Constrained Language Mode para commands del agente
// 2. Timeout estricto (30s default, configurable)
// 3. Working directory restringido (no puede cd a system dirs)
// 4. Environment sanitizado (no hereda todas las env vars del proceso)
// 5. Output truncado a 50KB (prevenir memory exhaustion)
```

### 3. Dependency audit

```bash
# Rust:
cargo audit
cargo deny check

# Frontend:
npm audit
npx better-npm-audit audit

# Documentar: cada dependencia con versión, licencia, y última vulnerabilidad conocida
```

### 4. Input sanitization en todos los IPC commands

```rust
// Cada IPC command debe validar sus inputs:
// - String lengths (max 10KB para mensajes, 100 chars para nombres)
// - No path traversal en playbook names (no ../, no absolute paths)
// - No SQL injection en queries (ya usamos parametrized queries, verificar)
// - No XSS en contenido que se renderiza en el frontend
```

### 5. Rate limiting en la API pública

```rust
// Agregar rate limiter a axum:
// - Por API key: 100/min free, 1000/min pro
// - Por IP: 50/min sin auth
// - Headers: X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Reset
```

### 6. Security documentation

```markdown
# AgentOS Security Model

## Data at rest
- API keys: AES-256-GCM encrypted vault (R21)
- Database: SQLite (local, not exposed to network)
- Playbooks: local filesystem, sandboxed

## Data in transit
- LLM API calls: HTTPS/TLS 1.3
- Mesh: WebSocket with E2E encryption (X25519 + AES-256-GCM)
- Webhooks: HMAC-SHA256 signed

## Execution sandbox
- CLI: PowerShell Constrained Language, timeout, blacklist
- Vision: max_steps limit, action dedup, coordinate bounds check
- Plugins: WebAssembly sandbox (wasmtime)

## Authentication
- Local: vault master password + OS keychain
- API: bearer token (bcrypt hashed)
- Enterprise: OIDC/SAML SSO
```

---

## Demo

1. Intentar inyectar comando peligroso → safety guard lo bloquea
2. cargo audit → 0 vulnerabilities
3. npm audit → 0 high/critical
4. API sin auth → 401. API rate exceeded → 429 con headers correctos
5. Security doc publicado y revisable
