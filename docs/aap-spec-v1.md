# Agent-to-Agent Protocol (AAP) v1.0 Specification

## Overview

The Agent-to-Agent Protocol (AAP) is an open HTTP-based protocol enabling communication between autonomous AI agents. It provides a standardized way for agents to discover each other's capabilities, delegate tasks, and exchange results.

## Transport

- **Protocol**: HTTP/1.1 or HTTP/2
- **Default Port**: 9100
- **Content-Type**: `application/json`
- **Encoding**: UTF-8

## Endpoints

### Health Check

```
GET /aap/health
```

Returns the protocol version and server status.

**Response:**
```json
{
  "protocol": "AAP",
  "version": "1.0",
  "status": "ok"
}
```

### Send Message

```
POST /aap/v1/message
```

Send any AAP message to the agent. The agent processes the message based on its `msg_type`.

**Request Body:** An `AAPMessage` object (see Message Format below).

**Response (TaskRequest):**
```json
{
  "status": "accepted",
  "trace_id": "<uuid>"
}
```

### Query Capabilities

```
GET /aap/v1/capabilities
```

Returns the agent's identity and declared capabilities.

**Response:**
```json
{
  "node_id": "<uuid>",
  "node_name": "AgentOS-Desktop",
  "protocol_version": "1.0",
  "capabilities": { ... }
}
```

## Message Format

Every AAP message follows this envelope:

| Field         | Type     | Required | Description                              |
|---------------|----------|----------|------------------------------------------|
| `version`     | string   | yes      | Protocol version, e.g. `"1.0"`          |
| `msg_type`    | string   | yes      | One of the defined message types         |
| `sender_id`   | string   | yes      | Unique identifier of the sending agent   |
| `sender_name` | string   | yes      | Human-readable name of the sender        |
| `timestamp`   | string   | yes      | ISO 8601 / RFC 3339 timestamp            |
| `payload`     | object   | yes      | Type-specific payload data               |
| `trace_id`    | string   | no       | UUID for distributed tracing             |

## Message Types

### `task_request`

Request another agent to perform a task.

**Payload:**
```json
{
  "task": "Summarize this document",
  "priority": "normal"
}
```

Priority values: `"low"`, `"normal"`, `"high"`, `"critical"`.

### `task_response`

Response to a previously received task request.

**Payload:**
```json
{
  "result": "The document discusses...",
  "success": true
}
```

### `capability_query`

Ask an agent what it can do. Payload is empty `{}`.

### `capability_response`

Response listing the agent's capabilities. Payload is agent-defined.

### `heartbeat`

Periodic liveness signal.

**Payload:**
```json
{
  "uptime_secs": 3600,
  "load": 0.42
}
```

### `error`

Report an error in processing a previous message.

**Payload:**
```json
{
  "code": "TASK_FAILED",
  "message": "Unable to process task",
  "original_trace_id": "<uuid>"
}
```

## Discovery

Agents can be discovered via:
1. **Direct connection** using a known host and port.
2. **Local network broadcast** (UDP on port 9101, outside this spec).
3. **Registry** (future extension).

## Security Considerations

- AAP v1.0 operates over plain HTTP for local/trusted networks.
- Production deployments should use TLS (HTTPS).
- Authentication can be added via bearer tokens in the `Authorization` header.
- Rate limiting is recommended on the server side.

## Versioning

The `version` field in every message enables forward compatibility. Agents should reject messages with unsupported major versions and gracefully handle unknown fields.
