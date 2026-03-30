# FASE R24 — API PÚBLICA: REST + SDK + CLI para developers

**Objetivo:** Developers externos pueden enviar tareas a AgentOS vía REST API, recibir resultados por webhook, y usar un SDK de Python o CLI tool.

---

## Tareas

### 1. HTTP server embebido (no FastAPI — Rust nativo)

```rust
// Usar axum o actix-web embebido en el binario
// Puerto configurable (default 8080, solo localhost)

// Endpoints:
// POST /api/v1/tasks              — crear tarea
// GET  /api/v1/tasks              — listar recientes
// GET  /api/v1/tasks/{id}         — detalle
// GET  /api/v1/status             — estado del agente
// GET  /api/v1/health             — health check
// POST /api/v1/webhooks           — registrar webhook
// GET  /api/v1/playbooks          — listar playbooks

// Auth: header Authorization: Bearer aos_key_xxx
// Rate limit: 100 req/min free, 1000 pro
```

```toml
[dependencies]
axum = "0.7"
tower-http = { version = "0.5", features = ["cors"] }
```

### 2. API key management

```rust
// Generar: aos_key_ + 32 chars random base62
// Almacenar: hash bcrypt en SQLite
// Scopes: tasks:read, tasks:write, playbooks:read, admin

CREATE TABLE IF NOT EXISTS api_keys (
    id          TEXT PRIMARY KEY,
    key_hash    TEXT NOT NULL,      -- bcrypt
    name        TEXT NOT NULL,
    scopes      TEXT NOT NULL,      -- JSON array
    last_used   TEXT,
    created_at  TEXT NOT NULL
);
```

### 3. Webhooks

```rust
// Cuando tarea completa → POST al webhook URL del usuario
// Payload: {task_id, status, output, model, cost}
// Firma: X-AgentOS-Signature: sha256=HMAC(secret, body)
// Retry: 3 intentos con backoff exponencial
```

### 4. Frontend: Developer section

```
DEVELOPER                               
API KEYS                          [+ Generate]
┌──────────────────────────────────────────┐
│ aos_key_***a3f7  tasks:rw  2d ago  [📋][🗑]│
└──────────────────────────────────────────┘

WEBHOOKS                          [+ Add]
┌──────────────────────────────────────────┐
│ https://myapp.com/hook  task.completed ●  │
└──────────────────────────────────────────┘

API USAGE
  [line chart: requests per day]
  142 / 1,000 this month

QUICK START
  curl -X POST http://localhost:8080/api/v1/tasks \
    -H "Authorization: Bearer aos_key_xxx" \
    -d '{"text": "check disk space"}'
```

### 5. Python SDK (archivo .py distribuible)

```python
# agentos_sdk.py — single file, pip installable later
import requests

class AgentOS:
    def __init__(self, api_key, base_url="http://localhost:8080"):
        self.api_key = api_key
        self.base_url = base_url
        self.headers = {"Authorization": f"Bearer {api_key}"}
    
    def run_task(self, text):
        r = requests.post(f"{self.base_url}/api/v1/tasks", 
                         json={"text": text}, headers=self.headers)
        return r.json()
    
    def get_status(self):
        return requests.get(f"{self.base_url}/api/v1/status", headers=self.headers).json()

# Usage:
# agent = AgentOS("aos_key_xxx")
# result = agent.run_task("check disk space")
```

---

## Demo

1. Generar API key en Developer section
2. curl POST /api/v1/tasks → tarea se ejecuta → resultado en JSON
3. Python SDK: `agent.run_task("hello")` → respuesta
4. Webhook: tarea completa → POST llega al endpoint configurado
