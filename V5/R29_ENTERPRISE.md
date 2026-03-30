# FASE R29 — ENTERPRISE: SSO, audit logs, multi-tenant

**Objetivo:** Empresas pueden deployar AgentOS con login SSO (Google/Microsoft), audit logs exportables para compliance, y separación de datos por organización.

---

## Tareas

### 1. SSO via OpenID Connect

```rust
// Nuevo: src-tauri/src/auth/oidc.rs

// Soportar:
// - Google Workspace (accounts.google.com)
// - Microsoft Entra ID (login.microsoftonline.com)
// - Okta
// - Cualquier proveedor OIDC

pub struct OIDCProvider {
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,  // Del vault
}

impl OIDCProvider {
    /// Inicia flujo de login (abre browser)
    pub async fn login(&self) -> Result<UserIdentity>;
    
    /// Verifica token existente
    pub async fn verify_token(&self, token: &str) -> Result<UserIdentity>;
}

pub struct UserIdentity {
    pub user_id: String,
    pub email: String,
    pub name: String,
    pub org_id: Option<String>,
    pub roles: Vec<String>,
}
```

### 2. Audit log inmutable

```sql
CREATE TABLE IF NOT EXISTS audit_log (
    id          TEXT PRIMARY KEY,
    timestamp   TEXT NOT NULL,
    user_id     TEXT,
    action      TEXT NOT NULL,    -- task_created, settings_changed, playbook_installed, login, logout
    resource    TEXT,             -- qué se afectó
    details     TEXT,             -- JSON metadata
    ip_address  TEXT
);

-- Append-only: NUNCA delete ni update
```

```rust
pub struct AuditLogger;

impl AuditLogger {
    pub fn log(action: &str, resource: &str, details: &serde_json::Value) -> Result<()>;
    pub fn export(start: &str, end: &str, format: &str) -> Result<Vec<u8>>; // JSON o CSV
}
```

### 3. Multi-tenant (org_id en tablas principales)

```sql
-- Agregar org_id a tasks, playbooks, triggers:
ALTER TABLE tasks ADD COLUMN org_id TEXT DEFAULT 'default';
```

### 4. Admin dashboard

```
ADMIN (solo visible para roles admin)
───────────────────────────────
USERS (3)
│ alice@company.com  Admin   47 tasks today
│ bob@company.com    User    12 tasks today
│ carol@company.com  User     3 tasks today

USAGE
│ Total tasks this month: 1,247
│ Total cost: $45.67
│ Top user: alice (580 tasks)

AUDIT LOG                               [Export CSV]
│ 2026-03-28 14:30  alice  task_created  "check disk"
│ 2026-03-28 14:29  bob    login         via Google SSO
│ 2026-03-28 14:25  alice  settings_changed  tier=premium
```

---

## Demo

1. Login con Google SSO → sesión autenticada
2. Ejecutar tarea → audit log registra el evento
3. Export audit log → CSV con todos los eventos
4. Admin ve dashboard con users + usage
