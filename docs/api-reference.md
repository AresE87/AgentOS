# AgentOS Public API Reference

Base URL: `http://localhost:8080`

Enable the API in **Settings → API → Enable Local REST API**.

---

## Authentication

All requests require a Bearer token. Generate one in **Settings → API → API Keys**.

```
Authorization: Bearer aos_your_key_here
```

---

## Endpoints

### POST /v1/message

Send a natural language task to the agent.

**Request**
```json
{
  "text": "Check my disk space",
  "context": {}
}
```

**Response**
```json
{
  "id": "msg_01abc",
  "status": "completed",
  "output": "C: drive has 42.3 GB free of 512 GB",
  "steps": [
    {
      "type": "shell",
      "command": "Get-PSDrive C | ...",
      "output": "..."
    }
  ],
  "created_at": "2025-01-01T12:00:00Z",
  "duration_ms": 1240
}
```

---

### GET /v1/tasks

List recent tasks.

**Query Parameters**
| Parameter | Type   | Description                  |
|-----------|--------|------------------------------|
| limit     | int    | Max results (default: 20)    |
| offset    | int    | Pagination offset             |
| status    | string | Filter: pending/completed/failed |

**Response**
```json
{
  "tasks": [
    {
      "id": "msg_01abc",
      "text": "Check my disk space",
      "status": "completed",
      "created_at": "2025-01-01T12:00:00Z"
    }
  ],
  "total": 142
}
```

---

### GET /v1/tasks/:id

Get a specific task by ID.

**Response**
```json
{
  "id": "msg_01abc",
  "text": "Check my disk space",
  "status": "completed",
  "output": "C: drive has 42.3 GB free",
  "steps": [],
  "created_at": "2025-01-01T12:00:00Z",
  "duration_ms": 1240
}
```

---

### POST /v1/playbooks/:id/run

Run a playbook by ID.

**Request**
```json
{
  "params": {
    "city": "London"
  }
}
```

**Response**
```json
{
  "run_id": "run_02xyz",
  "playbook_id": "weather-check",
  "status": "running"
}
```

---

### GET /v1/playbooks

List all available playbooks.

**Response**
```json
{
  "playbooks": [
    {
      "id": "disk-cleanup",
      "name": "Disk Cleanup",
      "category": "productivity",
      "version": "1.0.0"
    }
  ]
}
```

---

### GET /v1/health

Check API server health and connected providers.

**Response**
```json
{
  "status": "ok",
  "version": "1.0.0",
  "providers": {
    "anthropic": true,
    "openai": false,
    "ollama": false
  },
  "uptime_seconds": 3600
}
```

---

### POST /v1/webhooks

Register a webhook URL to receive task completion events.

**Request**
```json
{
  "url": "https://your-server.com/agentos-webhook",
  "events": ["task.completed", "task.failed"],
  "secret": "your_webhook_secret"
}
```

**Response**
```json
{
  "id": "wh_03def",
  "url": "https://your-server.com/agentos-webhook",
  "events": ["task.completed", "task.failed"],
  "created_at": "2025-01-01T12:00:00Z"
}
```

---

### Webhook Payload

When a task completes, AgentOS POSTs to your registered URL:

```json
{
  "event": "task.completed",
  "task_id": "msg_01abc",
  "text": "Check disk space",
  "output": "C: 42.3 GB free",
  "timestamp": "2025-01-01T12:01:00Z"
}
```

Verify authenticity using the `X-AgentOS-Signature` header (HMAC-SHA256 of the body using your secret).

---

## Error Responses

| Code | Meaning                    |
|------|----------------------------|
| 400  | Bad request / invalid JSON |
| 401  | Missing or invalid API key |
| 404  | Resource not found         |
| 429  | Rate limit exceeded        |
| 500  | Internal server error      |

```json
{
  "error": "rate_limit_exceeded",
  "message": "You have exceeded 500 tasks/day on the Pro plan",
  "retry_after": 3600
}
```

---

## Rate Limits

| Plan | Tasks/day | Requests/min |
|------|-----------|--------------|
| Free | 20        | 10           |
| Pro  | 500       | 60           |
| Team | Unlimited | 300          |
