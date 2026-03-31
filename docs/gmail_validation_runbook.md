# AgentOS Gmail Validation Runbook

This runbook documents what Gmail behavior is validated locally and what still requires a real Google account.

## Current validation strategy

AgentOS now validates Gmail in two layers:

1. Positive local tests against a mock HTTP server
2. Honest fallback tests when Gmail auth is absent

This avoids treating Gmail as "real" based on compilation alone.

## What is covered by local positive tests

The positive tests live in:

- `src-tauri/src/integrations/email.rs`

They now verify:

- OAuth auth URL includes the combined Calendar + Gmail scopes
- OAuth code exchange succeeds against a local mock token endpoint
- access token refresh succeeds against a local mock token endpoint
- Gmail `list_messages` succeeds
- Gmail `get_message` succeeds
- Gmail `search` succeeds
- Gmail `send_email` succeeds
- Gmail `mark_read` succeeds
- Gmail `move_to` succeeds
- `EmailManager` uses Gmail mode when enabled and authenticated
- fallback local mode remains active and honest when auth is missing

## Local commands to reproduce

Run from `src-tauri`:

```powershell
cargo test email::tests -- --nocapture
```

The key positive tests are:

- `gmail_oauth_exchange_and_refresh_succeed_with_mock_server`
- `gmail_provider_positive_flow_supports_list_get_search_send_and_modify`
- `email_manager_uses_gmail_provider_when_enabled_and_authenticated`
- `email_manager_fallback_supports_send_list_search_move_and_mark_read`

## What the local mock server proves

The mock server is used only inside the test module. It proves that AgentOS:

- formats Gmail REST URLs correctly
- sends bearer auth headers
- exchanges and refreshes tokens using the expected token endpoint contract
- parses Gmail message payloads into `EmailMessage`
- routes product behavior through Gmail mode when auth is present
- falls back to in-memory mode when auth is absent

## Optional real-account smoke test

If you want a live Google validation after the local tests pass:

1. Configure:
   - `google_client_id`
   - `google_client_secret`
   - `google_gmail_enabled=true`
2. Open the Gmail auth URL from the app.
3. Complete consent and capture the callback code.
4. Run the code exchange through AgentOS.
5. Verify:
   - `cmd_gmail_auth_status` reports `authenticated=true`
   - `cmd_email_list("inbox")` returns messages
   - `cmd_email_search("subject")` returns results
   - `cmd_email_send(...)` sends a message to a test mailbox
   - `cmd_email_get(id)` returns the sent/read message

Evidence to collect:

- auth status JSON
- one successful list result
- one successful search result
- one successful send result

## Honest scope of the current evidence

What is real now:

- reproducible positive Gmail behavior is covered locally
- fallback behavior is covered locally
- Gmail no longer depends on compile-only evidence

What still depends on external credentials:

- validating against a real Google tenant
- OAuth consent screen behavior outside the app
- quotas, revoked tokens, and Google-side permission errors

Those are integration-environment concerns, not missing repo wiring.
