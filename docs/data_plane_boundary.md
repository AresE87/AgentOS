# Data Plane Boundary

## Control plane
- Settings, approvals, workflows, updater decisions, and alert rules
- Main files:
  - `src-tauri/src/lib.rs`
  - `src-tauri/src/observability/alerts.rs`
  - `src-tauri/src/updater/checker.rs`

## Data plane
- Memory, files, logs, and outputs created while AgentOS executes work
- Main files:
  - `src-tauri/src/memory/database.rs`
  - `src-tauri/src/observability/logger.rs`
  - `src-tauri/src/files`

## Network plane
- Mesh discovery and relay transport for node-to-node operation
- Main files:
  - `src-tauri/src/mesh`
  - `src-tauri/src/lib.rs`

## Why this matters
- D9 requires explicit boundaries so the product can be reasoned about, audited, and operated.
- The dashboard now reflects these planes in `Operations`.
