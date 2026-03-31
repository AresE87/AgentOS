# AgentOS Deployment Runbooks

This document covers the real deployment and support procedures already wired in the repository.

## 1. Desktop installation runbook

1. Install prerequisites for the current platform (`Rust`, `Node.js`, Tauri prerequisites).
2. From the repo root, install frontend dependencies and build the UI:
   - `cd frontend`
   - `npm install`
   - `npm run build`
3. Build the desktop app:
   - `cd ..\\src-tauri`
   - `cargo build --release`
4. On first run, AgentOS will create the local app directory, SQLite database, plugin directory and settings files under the platform app-data directory.

## 2. Updater runbook

The updater supports two honest modes:

- `check_only`: the app can detect releases but cannot install them yet
- `install_ready`: the app has a configured public key and the release publishes valid updater artifacts

To move to `install_ready`:

1. Set `updater_pubkey` in settings.
2. Build release artifacts with updater outputs enabled in `src-tauri/tauri.conf.json`.
3. Publish a GitHub Release that includes the generated updater artifacts and `latest.json`.
4. Confirm from the desktop UI or updater IPC that the status mode changed to `install_ready`.

## 3. Multi-node / relay runbook

1. Configure `relay_auth_token` and relay endpoint.
2. Start the relay server reachable by all participating nodes.
3. Connect each node with the relay commands or UI entry point.
4. Validate:
   - node registration
   - heartbeat visibility
   - task send
   - task poll

If one node fails, the relay should keep other nodes visible and returning status independently.

## 4. Plugin lifecycle runbook

AgentOS now supports:

- install
- update
- rollback
- uninstall
- enable / disable

Operational rules:

1. A plugin directory must contain `plugin.json` and a real entry point file.
2. Versions must follow `x.y.z`.
3. Updates must increase version number.
4. Before update, AgentOS creates a rollback backup under `plugins/.backups/<plugin>/<version>`.
5. Rollback restores the latest backup for that plugin.

## 5. Tenant / org runbook

1. Create one or more organizations.
2. Set the active tenant context with the current-org command.
3. Perform branding or marketplace operations inside that scope.
4. If a command tries to act on a different org than the active one, AgentOS rejects it as a tenant scope violation.

## 6. Recovery notes

- API failure: check `cmd_api_get_status`, restart the desktop app, then verify `/health` and `/v1/status`.
- Plugin update failure: run plugin rollback and inspect the plugin backup directory.
- Tenant mismatch: reselect the correct current org before retrying the operation.
- Updater not install-ready: verify `updater_pubkey`, signed release artifacts and `latest.json` publication.

## 7. Minimum verification checklist

- Backend compiles with `cargo test --lib --no-run`
- API route tests pass
- Plugin lifecycle tests pass
- Tenant isolation tests pass
- Frontend `tsc` passes if any UI wiring was touched
