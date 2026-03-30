# FASE R39 — COMPLIANCE: GDPR, SOC 2 prep, privacy by design

**Objetivo:** AgentOS cumple con los requisitos que una empresa compradora va a exigir: GDPR compliance, preparación para SOC 2, data residency configurable, y privacy by design documentado.

---

## Tareas

### 1. Right to erasure (GDPR Art. 17)

```rust
// El usuario puede borrar TODOS sus datos con un click
#[tauri::command]
async fn delete_all_user_data() -> Result<(), String> {
    // 1. Borrar todas las tablas de SQLite
    // 2. Borrar vault
    // 3. Borrar playbooks
    // 4. Borrar config
    // 5. Borrar logs
    // 6. Borrar cache
    // Resultado: la app queda como recién instalada
}
```

Frontend en Settings:
```
DATA & PRIVACY
  [Export My Data]  → JSON con todos los datos del usuario
  [Delete All Data] → confirmación doble → borra TODO
  
  ⚠️ This will permanently delete all your tasks, playbooks,
  settings, and analytics. This cannot be undone.
  [Type "DELETE" to confirm] [Cancel]
```

### 2. Data export (GDPR Art. 20 — portabilidad)

```rust
#[tauri::command]
async fn export_all_data() -> Result<String, String> {
    // Genera JSON con:
    // - tasks (all)
    // - playbooks (list + metadata, no binaries)
    // - settings (sin API keys)
    // - analytics summary
    // - triggers
    // - reviews
    // Retorna path al .json generado
}
```

### 3. Data residency documentation

```markdown
# Where AgentOS Stores Data

ALL data is stored LOCALLY on your machine. Nothing goes to our servers.

| Data type | Location | Encryption |
|-----------|----------|------------|
| Tasks & history | AppData/AgentOS/db.sqlite | No (local) |
| API keys | AppData/AgentOS/vault.enc | AES-256-GCM |
| Playbooks | AppData/AgentOS/playbooks/ | No (local) |
| Settings | AppData/AgentOS/config.json | No (local) |
| Analytics | Computed from SQLite | N/A |

Data that leaves your machine:
| Data | Destination | Why |
|------|-------------|-----|
| Task text | LLM provider (Anthropic/OpenAI/Google) | To get AI response |
| Screenshots | LLM provider (vision mode only) | To analyze screen |
| Telegram messages | Telegram servers | To send/receive |

Data that NEVER leaves your machine:
- API keys (encrypted in vault)
- File contents (unless you ask the agent to share them)
- Browsing history
- Playbook recordings
```

### 4. Privacy settings

```
PRIVACY
  [x] Send task text to AI providers (required for functionality)
  [ ] Send anonymous usage analytics to AgentOS team
  [ ] Allow crash reports to be sent automatically
  
  Data retention:
  Keep task history for: [30 days ▾] (7/30/90/forever)
  Auto-delete old tasks: [ON]
```

### 5. SOC 2 preparation checklist

```markdown
# SOC 2 Readiness (Type I)

## CC1 — Control Environment
[x] Security policy documented
[x] Roles defined (admin, user)
[x] Code of conduct (contributor guidelines)

## CC2 — Communication
[x] Security model documented
[x] Privacy policy published
[x] Data handling transparency

## CC3 — Risk Assessment
[x] Threat model documented
[x] Risk registry maintained
[x] Penetration testing (R36)

## CC6 — Logical Access
[x] Authentication (API keys, SSO)
[x] Authorization (scopes, plan limits)
[x] Encryption at rest (vault)
[x] Encryption in transit (TLS, mesh E2E)

## CC7 — System Operations
[x] Monitoring (analytics, audit logs)
[x] Incident response plan
[x] Change management (git, releases)

## CC8 — Change Management
[x] Version control (git)
[x] Code review process
[x] Automated testing (132+ tests)
[x] Release process documented
```

---

## Demo

1. Settings → "Export My Data" → JSON descargado con todos los datos
2. Settings → "Delete All Data" → confirmar → app queda vacía como nueva
3. Privacy settings: toggle analytics/crash reports
4. Data retention: configurar "30 days" → datos viejos se borran automáticamente
5. SOC 2 checklist publicado y revisable
