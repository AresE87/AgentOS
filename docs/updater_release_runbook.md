# AgentOS Updater Release Runbook

This runbook documents the exact steps required to move AgentOS from `check_only` to `install_ready`.

## Current repo state

AgentOS already has the local wiring required for the Tauri 2 updater:

- `src-tauri/tauri.conf.json`
  - `"bundle.createUpdaterArtifacts": true`
- `src-tauri/src/lib.rs`
  - initializes `tauri_plugin_updater`
  - exposes `cmd_check_for_update`, `cmd_install_update`, `cmd_get_current_version`
- `src-tauri/src/updater/checker.rs`
  - checks GitHub Releases metadata as an honest fallback
  - attempts signed install only when `updater_pubkey` is configured
  - exposes `status_mode`:
    - `check_only`
    - `manifest_pending`
    - `install_ready`
- `src-tauri/src/config/settings.rs`
  - persists `github_repo`
  - persists `updater_pubkey`
- `frontend/src/pages/dashboard/Settings.tsx`
  - shows updater mode, manifest URL, repo, and whether install is enabled

As of 2026-03-31, the upstream repo still has no public releases and no `latest.json`, so C2 remains partial until the steps below are completed.

## What changes AgentOS from check-only to install-ready

AgentOS becomes `install_ready` only when all of the following are true:

1. `updater_pubkey` is configured in app settings.
2. A signed updater artifact has been built from this repo version.
3. A valid `latest.json` is published at:
   `https://github.com/<owner>/<repo>/releases/latest/download/latest.json`
4. The asset URLs referenced by `latest.json` are reachable from GitHub Releases.
5. `cmd_check_for_update` can validate the manifest through `tauri_plugin_updater`.

If any of those is missing, AgentOS must remain in `check_only` or `manifest_pending`.

## One-time key generation

Tauri's official updater docs require a public/private signing key pair.

PowerShell:

```powershell
cargo tauri signer generate -w $HOME\.tauri\agentos.key
```

Expected output:

- private key file at something like `C:\Users\<you>\.tauri\agentos.key`
- matching public key text

Rules:

- never commit the private key
- never publish the private key
- store the public key in AgentOS settings

## Configure AgentOS with the public key

Choose one of these two paths:

1. App UI
   - Open `Settings -> Updates`
   - Set `GitHub repo`
   - Paste `Updater public key`
   - Save both values

2. Config file
   - Open the AgentOS `config.json` in the app data directory
   - Set:

```json
{
  "github_repo": "AresE87/AgentOS",
  "updater_pubkey": "PASTE_THE_PUBLIC_KEY_CONTENT_HERE"
}
```

Notes:

- `updater_pubkey` must be the public key content, not a file path
- changing only the key is not enough; the release artifacts still need to exist upstream

## Build signed updater artifacts

Before building, export the signing key into the shell environment.

PowerShell:

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = "C:\Users\<you>\.tauri\agentos.key"
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
```

Then build AgentOS from `src-tauri`:

```powershell
cargo tauri build
```

Expected Windows outputs with `"createUpdaterArtifacts": true`:

- `target\release\bundle\nsis\*.exe`
- `target\release\bundle\nsis\*.exe.sig`
- `target\release\bundle\msi\*.msi`
- `target\release\bundle\msi\*.msi.sig`

## Publish a GitHub Release that the app can consume

1. Tag the repo with the version that matches `src-tauri/tauri.conf.json` and `Cargo.toml`.
2. Create a GitHub Release from that tag.
3. Upload the updater assets and signatures produced by the build.
4. Upload `latest.json` as a release asset.

For AgentOS, `latest.json` must end up reachable at:

```text
https://github.com/AresE87/AgentOS/releases/latest/download/latest.json
```

## Minimum required latest.json shape

Tauri validates the whole file before comparing versions.

Example shape:

```json
{
  "version": "4.2.1",
  "notes": "Release notes here",
  "pub_date": "2026-03-31T12:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "CONTENT_OF_SIG_FILE",
      "url": "https://github.com/AresE87/AgentOS/releases/download/v4.2.1/AgentOS_4.2.1_x64-setup.exe"
    }
  }
}
```

Rules:

- `signature` must be the content of the `.sig` file, not a path
- `url` must point to the downloadable updater artifact
- `version` must be valid semver

## Validate from the app

Open `Settings -> Updates` and verify:

1. `GitHub repo` points to the release repo.
2. `Updater public key` has been saved.
3. `Manifest URL` matches the GitHub latest download URL.
4. `Mode` becomes `Install ready` after a successful check.
5. `Install update` is enabled only when:
   - a newer signed release exists
   - the manifest validates successfully

Expected modes:

- `Check only`
  - no public key saved
- `Manifest pending`
  - public key saved but `latest.json` invalid, missing, or not published
- `Install ready`
  - public key saved and manifest validated by Tauri updater

## Repo validation commands

Run from `src-tauri`:

```powershell
cargo test latest_manifest_url_points_to_expected_location -- --nocapture
cargo test updater_requires_pubkey_for_install_support -- --nocapture
cargo test operational_mode_distinguishes_check_only_manifest_pending_and_install_ready -- --nocapture
```

## What is still outside the codebase

The final step for C2 is not another code change.

It is the publication of a real GitHub Release containing:

- signed updater artifacts
- matching `.sig` files
- a valid `latest.json`

Until that exists upstream, AgentOS must continue reporting C2 as partial.
