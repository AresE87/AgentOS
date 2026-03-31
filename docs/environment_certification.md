# Environment Certification

## Certified baseline for this repo
- Desktop shell built with Tauri v2 and Rust IPC
- React/Vite dashboard build for operator surfaces
- SQLite-backed local runtime for workflows, metrics, and app state
- GitHub release polling wired through the updater checker

## Supported execution contexts
- Local desktop operator environment
- Local development browser mode with mocks
- GitHub-backed release lookup for updater status

## Not certified yet
- Fully automated multi-environment conformance suite
- Production cloud relay deployment matrix
- Formal OS-by-OS desktop certification report

## Evidence
- `frontend/src/pages/dashboard/Operations.tsx`
- `src-tauri/src/updater/checker.rs`
- `src-tauri/src/lib.rs`

## Status
- Partial but explicit.
