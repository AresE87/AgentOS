# Agent-to-Agent Protocol (AAP) v2.0 Specification

**Version:** 2.0.0
**Status:** Stable
**Date:** 2026-03-29

## 1. Overview

AAP v2.0 is the next-generation protocol for inter-agent communication in the AgentOS ecosystem. It builds on AAP v1.0 with support for streaming responses, multi-agent conversations, swarm coordination, and cross-platform interoperability.

## 2. Transport

- **Primary:** HTTP/2 over TLS 1.3
- **Secondary:** WebSocket for streaming
- **Discovery:** mDNS for local network, relay servers for WAN

## 3. Authentication

- Mutual TLS (mTLS) for trusted agent pairs
- Bearer tokens (JWT) for relay-mediated connections
- API key fallback for simple integrations

## 4. Message Format

All messages use JSON with the following envelope:

```json
{
  "aap_version": "2.0",
  "message_id": "uuid-v4",
  "timestamp": "ISO-8601",
  "sender": {
    "agent_id": "string",
    "capabilities": ["string"]
  },
  "recipient": {
    "agent_id": "string"
  },
  "payload": {}
}
```

## 5. Endpoints

### 5.1 Health Check
- `GET /aap/v2/health` -- Returns agent availability and version

### 5.2 Capabilities
- `GET /aap/v2/capabilities` -- Returns supported task types and resource limits

### 5.3 Task Submission
- `POST /aap/v2/tasks` -- Submit a task for execution
- `GET /aap/v2/tasks/{id}` -- Get task status and result
- `DELETE /aap/v2/tasks/{id}` -- Cancel a running task

### 5.4 Streaming
- `WS /aap/v2/stream` -- Bidirectional streaming for real-time collaboration

### 5.5 Conversations
- `POST /aap/v2/conversations` -- Start a multi-agent conversation
- `POST /aap/v2/conversations/{id}/messages` -- Add a message to conversation
- `GET /aap/v2/conversations/{id}` -- Get conversation state

### 5.6 Swarm
- `POST /aap/v2/swarm/join` -- Join a swarm coordination group
- `POST /aap/v2/swarm/vote` -- Submit a consensus vote
- `GET /aap/v2/swarm/status` -- Get swarm task status

## 6. Error Codes

| Code | Meaning |
|------|---------|
| 4001 | Invalid AAP version |
| 4002 | Authentication failed |
| 4003 | Capability mismatch |
| 4004 | Task not found |
| 4005 | Rate limited |
| 4006 | Agent busy |
| 5001 | Internal agent error |
| 5002 | Upstream LLM failure |
| 5003 | Timeout exceeded |

## 7. Rate Limiting

Agents MUST respect the `X-AAP-RateLimit-Remaining` header. Default limits:
- 100 tasks/minute for standard agents
- 1000 tasks/minute for enterprise agents

## 8. Security Considerations

- All inter-agent communication MUST be encrypted in transit
- Agents MUST validate message signatures before processing
- Task outputs MUST be sanitized before forwarding
- Agents SHOULD implement circuit breakers for failed peers

## 9. Backwards Compatibility

AAP v2.0 agents MUST support AAP v1.0 endpoints at `/aap/v1/*` for a minimum of 12 months after v2.0 adoption.

## 10. Reference Implementation

The reference implementation is included in AgentOS v3.0 under `src-tauri/src/protocol/`.
