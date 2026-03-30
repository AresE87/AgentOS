# AgentOS Plugin Certification Process

## Overview

All plugins submitted to the AgentOS Marketplace must pass a 5-step certification review before publication. This ensures quality, security, and compatibility across the ecosystem.

---

## Step 1: Manifest Validation

- Plugin manifest (`plugin.yaml`) must include all required fields: `name`, `version`, `author`, `description`, `entry_point`, `permissions`.
- Version must follow semantic versioning (semver).
- Permissions must be declared explicitly (no wildcard access).
- Entry point file must exist and be loadable.

## Step 2: Security Audit

- Static analysis of plugin code for unsafe patterns (file system access outside sandbox, network calls to undeclared hosts, credential harvesting).
- Plugins requesting elevated permissions (e.g., `filesystem_write`, `network`) undergo manual review.
- No obfuscated code allowed.
- Dependencies must be pinned to specific versions.

## Step 3: Functional Testing

- Plugin must load without errors in a clean AgentOS environment.
- All declared methods must be callable and return valid responses.
- Plugin must handle missing or malformed inputs gracefully (no panics/crashes).
- UI pages and widgets (if declared) must render without JavaScript errors.

## Step 4: Performance & Resource Review

- Plugin must not consume more than 50 MB of memory at rest.
- Startup time must be under 2 seconds.
- API method calls must respond within 5 seconds under normal conditions.
- Plugin must not spawn background threads or processes without declaration.

## Step 5: Documentation & Metadata

- README or description must explain what the plugin does and how to use it.
- At least one usage example must be provided.
- Screenshots or demo GIFs are recommended for UI plugins.
- Author contact information must be valid.

---

## Certification Outcome

| Result | Meaning |
|--------|---------|
| **Certified** | Plugin passes all 5 steps and is published to the Marketplace. |
| **Conditional** | Minor issues found; plugin is published with a note to fix within 30 days. |
| **Rejected** | Critical issues found; plugin must be resubmitted after fixes. |

## Re-certification

Plugins must be re-certified when:
- A new major version is released.
- The AgentOS Extension API version changes.
- A security vulnerability is reported.
