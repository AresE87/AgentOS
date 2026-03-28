# Architecture: AOS-071 a AOS-079 — API Pública, SDK, Developer Ecosystem

**Fecha:** Marzo 2026

---

## API Design Principles

1. **RESTful**: recursos como sustantivos, HTTP methods como verbos
2. **Versionado**: `/api/v1/` — breaking changes solo en v2
3. **Consistencia**: todas las respuestas `{data, error, meta}`. Errores con `{code, message, details}`
4. **Async tasks**: POST /tasks retorna `{task_id, status: "pending"}`. Resultado por polling o webhook.
5. **Pagination**: `?page=1&per_page=20` con `meta: {total, page, per_page, total_pages}`

## API Response Format

```json
// Success
{"data": {...}, "meta": {"request_id": "...", "latency_ms": 42}}

// Error
{"error": {"code": "rate_limit_exceeded", "message": "...", "details": {}}, "meta": {...}}

// List
{"data": [...], "meta": {"total": 100, "page": 1, "per_page": 20, "total_pages": 5}}
```

## Authentication

```
Authorization: Bearer aos_key_xxxxxxxxxxxx

Key format: aos_key_ + 32 chars random (base62)
Stored as: bcrypt hash in SQLite
Scopes: tasks:read, tasks:write, playbooks:read, playbooks:write, mesh:read, admin
```

## Rate Limiting

```
Free:       100 req/min,   1,000 req/day
Pro:      1,000 req/min,  50,000 req/day
Enterprise: custom

Headers: X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Reset (epoch)
Algorithm: sliding window (Redis-like, but SQLite for local)
```

## Webhook Signing

```
POST to user's URL with:
  X-AgentOS-Signature: sha256=HMAC(webhook_secret, raw_body)
  X-AgentOS-Timestamp: epoch seconds
  Content-Type: application/json

User verifies: recompute HMAC with their secret, compare.
Reject if timestamp > 5 min old (replay prevention).
```

## SDK Architecture

```python
# agentos_sdk/client.py
class AgentOS:
    def __init__(self, api_key: str, base_url: str = "http://localhost:8080"): ...
    def run_task(self, text: str, **kwargs) -> TaskResult: ...
    def run_task_async(self, text: str, **kwargs) -> str:  # returns task_id
    def get_task(self, task_id: str) -> TaskResult: ...
    def wait_for_task(self, task_id: str, timeout: int = 300) -> TaskResult: ...

# agentos_sdk/async_client.py
class AsyncAgentOS:
    async def run_task(self, text: str, **kwargs) -> TaskResult: ...
    async def run_task_stream(self, text: str, **kwargs) -> AsyncIterator[str]: ...
```

## CLI Architecture

```
agentos — built on click + rich
├── run <message>         — execute task
├── status                — agent status
├── tasks [--limit N]     — list tasks
├── playbooks             — list playbooks
├── pack <dir>            — package playbook
├── install <file.aosp>   — install playbook
├── config [key] [value]  — get/set config
├── mesh                  — mesh status
└── docs                  — open API docs in browser
```
