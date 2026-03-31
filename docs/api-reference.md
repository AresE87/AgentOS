# AgentOS Public API Reference

Base URL: `http://localhost:8080`

The public API is local-first and must be enabled from the desktop app. Generate an API key from the AgentOS settings UI and send it as a bearer token.

## Authentication

```http
Authorization: Bearer aos_your_key_here
```

Every `/v1/*` endpoint requires a valid, enabled API key. The server updates `last_used` on successful authentication.

## Endpoints

### `GET /health`

No auth required. Returns the package version that the desktop app is running.

```json
{
  "status": "ok",
  "version": "4.2.0",
  "name": "AgentOS Public API",
  "api_version": "v1"
}
```

### `GET /v1/status`

Returns API liveness and queued-task count.

```json
{
  "status": "running",
  "api_version": "v1",
  "version": "4.2.0",
  "tasks_queued": 1
}
```

### `POST /v1/message`

Queues a task for asynchronous processing.

Request:

```json
{
  "text": "check disk space"
}
```

Response:

```json
{
  "task_id": "8d8c7a20-34cf-4f75-9d64-9b1e2d7e16dd",
  "status": "queued"
}
```

### `GET /v1/tasks`

Lists queued or completed API tasks from the in-memory task store.

Query params:

- `limit` (optional, default `20`)
- `status` (optional: `queued`, `running`, `completed`, `error`)

Response:

```json
{
  "tasks": [
    {
      "task_id": "8d8c7a20-34cf-4f75-9d64-9b1e2d7e16dd",
      "status": "completed",
      "text": "check disk space",
      "created_at": "2026-03-31T12:00:00Z",
      "has_result": true
    }
  ],
  "total": 1
}
```

### `GET /v1/task/:id`

Returns a single task payload and final result if available.

```json
{
  "task_id": "8d8c7a20-34cf-4f75-9d64-9b1e2d7e16dd",
  "status": "completed",
  "text": "check disk space",
  "created_at": "2026-03-31T12:00:00Z",
  "result": "C: has 42.3 GB free"
}
```

### `POST /webhooks/stripe`

Consumes Stripe webhook events and updates persisted billing state. If `stripe_webhook_secret` is configured, signature verification is enforced.

Response:

```json
{
  "received": true,
  "plan_updated": "pro"
}
```

## Error contract

Every API error returns JSON with the same shape:

```json
{
  "error": "invalid_api_key",
  "message": "Invalid or revoked API key"
}
```

Common error codes:

- `missing_authorization`
- `invalid_api_key`
- `invalid_request`
- `task_not_found`
- `invalid_webhook_signature`
- `invalid_webhook_payload`

## Real operational notes

- `/v1/message` is asynchronous by design. Queue the task, then poll `/v1/task/:id`.
- `/v1/tasks` reflects the live task store inside the running desktop app, not a historical database export.
- The API currently exposes the stable local integration surface only: health, status, message, task list/detail, and Stripe webhook intake.
