# AgentOS Platform Standardization

## Scope

This document defines the internal contract conventions that AgentOS uses across IPC, API, relay, handoff, swarm, and federated payloads. It is not marketing language. It reflects the real code in this branch.

## Naming

- Commands use verb-first snake_case: `get_status`, `swarm_execute`, `assign_escalation`.
- JSON keys use snake_case.
- Enums exposed through serde use `snake_case` when they represent runtime/API state.

## Lifecycle states

### Shared task lifecycle

Use these values when a unit of work moves through execution:

- `pending`
- `running`
- `completed`
- `failed`
- `cancelled`

### Human handoff lifecycle

Use the persisted `HandoffStatus` values from `src-tauri/src/escalation/detector.rs`:

- `pending_handoff`
- `assigned_to_human`
- `resumed`
- `completed_by_human`

### Swarm strategy vocabulary

Use only the normalized strategies from `src-tauri/src/swarm/coordinator.rs`:

- `parallel`
- `sequential`
- `vote`

## Errors

- Backend commands should return `Result<serde_json::Value, String>` at the Tauri boundary.
- Error strings must describe the real failing subsystem and avoid placeholder text.
- When an execution event is business-relevant, also log it to persisted audit data.

Examples already live in code:

- billing limit blocks -> `billing_limit_blocked`
- upgrade requests -> `upgrade_checkout_requested`
- plan changes -> `plan_changed`

## Versioning

- Protocol payloads carry explicit version strings where a network contract exists.
- `AAPMessage.version` is the source of truth for the Agent-to-Agent Protocol contract version.
- Desktop/app version stays aligned to the package version and updater metadata.

## Normalized areas already live

### Agent-to-Agent Protocol

- File: `src-tauri/src/protocol/spec.rs`
- Contract has explicit `version`, `msg_type`, `timestamp`, and `trace_id`.
- `AAPMessageType` is serialized as snake_case.

### Human Handoff

- File: `src-tauri/src/escalation/detector.rs`
- Handoff reasons and statuses are persisted as snake_case enums.
- Context package contains task, chain, evidence, notes, and audit trail.

### Swarm

- File: `src-tauri/src/swarm/coordinator.rs`
- Strategy input is normalized to `parallel` / `sequential` / `vote`.
- Runtime status fields use the shared lifecycle vocabulary.

### Federated sync

- File: `src-tauri/src/federated/client.rs`
- Shared payload is aggregate-only.
- Sensitive raw fields are explicitly excluded.
- Sync state is visible through `status`, `last_round`, and `last_payload`.

## Rule for new modules

Before adding a new IPC/API contract:

1. Pick an existing lifecycle vocabulary from this document.
2. Serialize enums as snake_case if they cross a boundary.
3. Include versioning when the payload leaves the local process.
4. Prefer persisted evidence over inferred dashboards.
