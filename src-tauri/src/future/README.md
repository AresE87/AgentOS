# Future Modules

These modules were moved here during the F1 cleanup (v5.0.0) because they contain useful architectural patterns but are not yet production-ready.

## Modules

- **economy/** -- Agent economy: hiring, reputation, collaboration, microtasks, escrow, insurance, creator studio, analytics, affiliate. Has real SQLite operations but needs real payment integration.
- **partnerships/** -- Hardware partner registry. Has SQLite-backed partner tracking but no real hardware integrations.
- **swarm/** -- Multi-agent swarm coordinator. Has real tokio-based concurrent execution but needs production hardening.

## How to Reactivate

1. Move the module directory back to `src-tauri/src/`
2. Add `pub mod <name>;` to `lib.rs`
3. Add the AppState fields back
4. Register IPC commands in the invoke_handler
5. Run `cargo check` and fix any API drift
