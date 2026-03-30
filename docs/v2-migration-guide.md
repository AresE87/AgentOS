# AgentOS v2.0 Migration Guide

## Overview

AgentOS v2.0 is a major release that introduces real-time translation, accessibility features, industry verticals, and offline-first capabilities. This guide covers what changed and how to migrate from v1.x.

## Breaking Changes

There are no breaking changes to existing IPC commands or frontend hooks. All v1.x commands continue to work as before. v2.0 is additive only.

## New Features

### R86: Real-time Translation

New IPC commands:
- `cmd_translate` — Translate text between 15+ supported languages
- `cmd_detect_language` — Auto-detect language from text content
- `cmd_supported_languages` — List all supported language codes and names

Frontend hooks (via `useAgent()`):
- `translate(text, sourceLang, targetLang)`
- `detectLanguage(text)`
- `supportedLanguages()`

### R87: Accessibility

New IPC commands:
- `cmd_get_accessibility` — Get current accessibility config
- `cmd_set_accessibility` — Update accessibility settings
- `cmd_get_accessibility_css` — Get generated CSS overrides

Frontend hooks:
- `getAccessibility()`
- `setAccessibility(config)`
- `getAccessibilityCss()`

Configuration options:
- `high_contrast` (bool) — Enable high contrast color scheme
- `font_scale` (f64) — Font size multiplier (0.5 to 3.0)
- `screen_reader_hints` (bool) — Add screen reader CSS utilities
- `reduce_motion` (bool) — Disable animations and transitions
- `keyboard_nav` (bool) — Enhanced focus indicators for keyboard navigation

### R88: Industry Verticals

New IPC commands:
- `cmd_list_verticals` — List all available industry verticals
- `cmd_get_vertical` — Get vertical details by ID
- `cmd_activate_vertical` — Activate a vertical for the current session
- `cmd_get_active_vertical` — Get the currently active vertical

Built-in verticals: `healthcare`, `legal`, `finance`, `education`, `ecommerce`

Frontend hooks:
- `listVerticals()`
- `getVertical(id)`
- `activateVertical(id)`
- `getActiveVertical()`

### R89: Offline First

New IPC commands:
- `cmd_check_connectivity` — Check network connectivity
- `cmd_get_offline_status` — Get offline status (cached count, pending sync, etc.)
- `cmd_sync_offline` — Flush pending sync queue
- `cmd_get_cached_response` — Retrieve a cached response by task query

Frontend hooks:
- `checkConnectivity()`
- `getOfflineStatus()`
- `syncOffline()`
- `getCachedResponse(task)`

New SQLite tables: `offline_cache`, `offline_pending`

## AppState Changes

Four new fields added to `AppState`:
- `translation_engine: Arc<TranslationEngine>`
- `accessibility_manager: Arc<Mutex<AccessibilityManager>>`
- `vertical_registry: Arc<Mutex<VerticalRegistry>>`
- `offline_manager: Arc<Mutex<OfflineManager>>`

## Version Bumps

- `src-tauri/Cargo.toml`: 1.3.0 -> 2.0.0
- `frontend/package.json`: 1.3.0 -> 2.0.0
- `mobile/package.json`: 1.3.0 -> 2.0.0

## Upgrade Steps

1. Pull the latest code from the `master` branch
2. Run `cargo build` in `src-tauri/` to compile new Rust modules
3. Run `npm install` in `frontend/` (no new dependencies)
4. The new features are available immediately via the existing `useAgent()` hook
5. No database migrations required (offline tables are auto-created on first use)
