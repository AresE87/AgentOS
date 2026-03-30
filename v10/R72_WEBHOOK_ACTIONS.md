# FASE R72 — WEBHOOK ACTIONS: El agente reacciona a eventos externos

**Objetivo:** GitHub hace push → AgentOS corre tests. Stripe cobra → AgentOS envía factura. Jira cambia status → AgentOS actualiza el board. El agente reacciona a webhooks de cualquier servicio.

---

## Tareas

### 1. Webhook receiver server

```rust
// Extender el HTTP server de la API (R24) con rutas de webhook:
// POST /webhooks/{trigger_id}  — endpoint por trigger
// POST /webhooks/generic       — endpoint genérico (rutea por headers/body)

// Cada webhook trigger tiene su propia URL única:
// https://localhost:8080/webhooks/wh_abc123
// (o via ngrok/relay para URLs públicas)
```

### 2. Webhook trigger types

```rust
pub struct WebhookTrigger {
    pub id: String,
    pub name: String,
    pub source: String,         // "github", "stripe", "jira", "custom"
    pub secret: Option<String>, // Para verificar firma
    pub filter: Option<String>, // JSON path filter: "$.action == 'opened'"
    pub task_template: String,  // "Review PR #{payload.pull_request.number}: {payload.pull_request.title}"
    pub enabled: bool,
}
```

### 3. Pre-built webhook templates

```
GitHub:
- Push → "Run tests for commit {sha}"
- PR opened → "Review PR #{number}: {title}"
- Issue created → "Triage issue #{number}: {title}"

Stripe:
- payment_succeeded → "Send invoice to {customer_email}"
- subscription_cancelled → "Alert: {customer} cancelled"

Jira:
- Issue transitioned → "Update board: {key} moved to {status}"
- Comment added → "Check comment on {key} by {author}"

Generic:
- Any POST → "Process webhook: {body preview}"
```

### 4. Webhook verification (security)

```rust
// GitHub: HMAC-SHA256 en X-Hub-Signature-256
// Stripe: HMAC-SHA256 en Stripe-Signature
// Jira: shared secret
// Custom: configurable

fn verify_webhook(headers: &HeaderMap, body: &[u8], trigger: &WebhookTrigger) -> bool {
    match trigger.source.as_str() {
        "github" => verify_github_signature(headers, body, &trigger.secret),
        "stripe" => verify_stripe_signature(headers, body, &trigger.secret),
        _ => trigger.secret.is_none() || verify_generic_hmac(headers, body, &trigger.secret),
    }
}
```

### 5. Frontend: Webhook management

```
WEBHOOK TRIGGERS                         [+ New Webhook]
──────────────────────────────────────────────────────
┌──────────────────────────────────────────────────────┐
│ 🐙 GitHub PR Review                    [ON] [Edit]   │
│    URL: .../webhooks/wh_abc123                        │
│    On: pull_request.opened                            │
│    Action: "Review PR #{number}: {title}"             │
│    Last triggered: 2h ago — ✅                        │
│                                                       │
│ 💳 Stripe Invoice                      [ON] [Edit]   │
│    URL: .../webhooks/wh_def456                        │
│    On: payment_succeeded                              │
│    Action: "Send invoice to {customer_email}"         │
│    Last triggered: 1 day ago — ✅                     │
└──────────────────────────────────────────────────────┘

Webhook logs:
│ 14:30  GitHub  PR #142 opened → "Review PR #142: Fix login" → ✅ completed
│ 12:15  Stripe  payment $49.99 → "Send invoice to john@..." → ✅ completed
```

---

## Demo

1. Configurar GitHub webhook → abrir PR en repo → AgentOS hace code review automáticamente
2. Configurar Stripe webhook → test payment → AgentOS genera invoice
3. Webhook log muestra cada evento recibido con resultado
4. Webhook con firma inválida → rechazado con log "Invalid signature"
5. Disable webhook → eventos llegan pero no se procesan
