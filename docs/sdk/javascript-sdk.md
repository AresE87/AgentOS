# AgentOS JavaScript SDK Reference Client

This is the minimal reference client for the local AgentOS API. It matches the real HTTP contract implemented in `src-tauri/src/api/routes.rs`.

## Quick start

```javascript
class AgentOS {
  constructor({ host = "http://localhost:8080", apiKey }) {
    this.host = host.replace(/\/+$/, "");
    this.apiKey = apiKey;
  }

  async request(method, path, body) {
    const headers = { "Content-Type": "application/json" };
    if (this.apiKey) headers.Authorization = `Bearer ${this.apiKey}`;

    const response = await fetch(`${this.host}${path}`, {
      method,
      headers,
      body: body ? JSON.stringify(body) : undefined,
    });

    const payload = await response.json().catch(() => ({}));
    if (!response.ok) {
      const error = new Error(payload.message || `HTTP ${response.status}`);
      error.status = response.status;
      error.code = payload.error || "http_error";
      throw error;
    }
    return payload;
  }

  health() {
    return this.request("GET", "/health");
  }

  status() {
    return this.request("GET", "/v1/status");
  }

  sendTask(text) {
    return this.request("POST", "/v1/message", { text });
  }

  listTasks({ limit = 20, status } = {}) {
    const params = new URLSearchParams();
    params.set("limit", String(limit));
    if (status) params.set("status", status);
    return this.request("GET", `/v1/tasks?${params.toString()}`);
  }

  getTask(taskId) {
    return this.request("GET", `/v1/task/${encodeURIComponent(taskId)}`);
  }
}
```

## Expected usage

```javascript
const client = new AgentOS({
  host: "http://localhost:8080",
  apiKey: "aos_your_key_here",
});

const queued = await client.sendTask("summarize today's CPU usage");
console.log(queued);

const task = await client.getTask(queued.task_id);
console.log(task.status, task.result);
```

## Real response shapes

`sendTask(text)`:

```json
{
  "task_id": "8d8c7a20-34cf-4f75-9d64-9b1e2d7e16dd",
  "status": "queued"
}
```

`listTasks()`:

```json
{
  "tasks": [
    {
      "task_id": "8d8c7a20-34cf-4f75-9d64-9b1e2d7e16dd",
      "status": "running",
      "text": "summarize today's CPU usage",
      "created_at": "2026-03-31T12:00:00Z",
      "has_result": false
    }
  ],
  "total": 1
}
```

## Error handling

The reference client preserves the API error code from the JSON payload:

```javascript
try {
  await client.status();
} catch (error) {
  console.error(error.code, error.message);
}
```
