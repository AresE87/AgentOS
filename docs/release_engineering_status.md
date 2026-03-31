# Release Engineering Status

## Scope
- Version source: `src-tauri/src/lib.rs` via `cmd_get_current_version`
- Release polling: `src-tauri/src/updater/checker.rs`
- Frontend surface: `frontend/src/pages/dashboard/Operations.tsx`

## What is real now
- The desktop app exposes the current packaged version at runtime.
- The updater checks GitHub Releases for `AresE87/AgentOS`.
- The dashboard now shows current version, latest release, updater state, and a release note excerpt.

## What is still partial
- There is no one-click in-app installer flow yet.
- Release promotion gates are documented but not automated inside the dashboard.
- Desktop and mobile release evidence still depend on external CI output.

## Validation path
- Frontend: `npm run build` in `frontend/`
- Backend: `cargo check` in `src-tauri/`

## Status
- Partial but real.
