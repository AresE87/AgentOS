# FASE R62 — APPROVAL WORKFLOWS: El agente pide permiso antes de actuar

**Objetivo:** Para acciones de alto impacto (enviar email, hacer compras, borrar archivos, ejecutar en producción), el agente pausa y pide aprobación al usuario. Configurable por nivel de riesgo.

---

## Tareas

### 1. Risk classification de acciones

```rust
pub enum ActionRisk {
    Low,        // Leer archivos, consultar datos, generar texto
    Medium,     // Crear archivos, modificar config, instalar software
    High,       // Enviar emails, hacer compras, borrar archivos, ejecutar en prod
    Critical,   // Transferencias de dinero, acceso a sistemas externos, borrar DB
}

pub fn classify_action_risk(action: &AgentAction) -> ActionRisk {
    match action {
        AgentAction::Command { command, .. } => {
            if contains_destructive(command) { ActionRisk::High }
            else if contains_install(command) { ActionRisk::Medium }
            else { ActionRisk::Low }
        }
        AgentAction::SendEmail { .. } => ActionRisk::High,
        AgentAction::Purchase { .. } => ActionRisk::Critical,
        AgentAction::DeleteFile { .. } => ActionRisk::High,
        _ => ActionRisk::Low,
    }
}
```

### 2. Approval flow

```rust
// Cuando una acción es Medium+ y approval está habilitado:
async fn execute_with_approval(action: &AgentAction, state: &AppState) -> Result<()> {
    let risk = classify_action_risk(action);
    
    if risk >= state.settings.approval_threshold {
        // 1. Pausar ejecución
        // 2. Emitir evento "approval_required"
        // 3. Esperar respuesta del usuario (con timeout)
        let approval = request_approval(action, risk).await?;
        
        match approval {
            Approval::Approved => execute(action).await,
            Approval::Rejected => Ok(()),  // No ejecutar
            Approval::Modified(new_action) => execute(&new_action).await,
            Approval::Timeout => Err("Approval timeout".into()),
        }
    } else {
        execute(action).await
    }
}
```

### 3. Frontend: Approval dialog

```
┌──────────────────────────────────────────────────────┐
│ ⚠️ APPROVAL REQUIRED                                 │
│                                                       │
│ The agent wants to:                                   │
│ ┌───────────────────────────────────────────────────┐ │
│ │ 📧 Send email to juan@company.com                 │ │
│ │    Subject: "Monthly Report - March 2026"          │ │
│ │    Body: [preview truncado...]                     │ │
│ │    Attachments: report_march.pdf                   │ │
│ └───────────────────────────────────────────────────┘ │
│                                                       │
│ Risk level: 🟡 HIGH                                   │
│ Requested by: Agent "María" during chain "Monthly     │
│ Report Generation"                                    │
│                                                       │
│ [✅ Approve]  [✏️ Edit & Approve]  [❌ Reject]        │
│                                                       │
│ ⏱ Auto-reject in 5:00 minutes                        │
└──────────────────────────────────────────────────────┘
```

### 4. Approval via canales (Telegram/WhatsApp)

```
Si el usuario no está frente a la PC:

Telegram:
🤖 AgentOS — Approval Required
The agent wants to send an email to juan@company.com
Subject: "Monthly Report - March 2026"
Risk: 🟡 HIGH

Reply: /approve or /reject
⏱ Auto-reject in 5 minutes
```

### 5. Settings: configurar thresholds

```
APPROVAL SETTINGS
  Require approval for:
  [ ] Low risk actions (read, query)
  [x] Medium risk actions (create, modify, install)
  [x] High risk actions (email, delete, external)
  [x] Critical risk actions (money, production)
  
  Approval timeout: [5 minutes ▾]
  On timeout: [Auto-reject ▾]  (auto-reject | auto-approve | pause indefinitely)
  
  Notify via: [Desktop + Telegram ▾]
  
  Trusted actions (skip approval):
  + [Add exception: e.g. "send email to team@company.com"]
```

### 6. Audit trail de approvals

```sql
CREATE TABLE IF NOT EXISTS approvals (
    id          TEXT PRIMARY KEY,
    task_id     TEXT NOT NULL,
    action_type TEXT NOT NULL,
    action_detail TEXT NOT NULL,
    risk_level  TEXT NOT NULL,
    decision    TEXT NOT NULL,     -- approved, rejected, modified, timeout
    decided_by  TEXT,              -- user_id o "auto"
    decided_via TEXT,              -- "desktop", "telegram", "timeout"
    decided_at  TEXT NOT NULL,
    created_at  TEXT NOT NULL
);
```

---

## Demo

1. Configurar approval para High+ → agente intenta enviar email → dialog aparece → approve → se envía
2. Reject → el agente NO envía y reporta "Action rejected by user"
3. Edit & Approve → cambiar el destinatario → se envía al nuevo
4. Timeout → 5 min sin respuesta → auto-reject → notificación "Approval expired"
5. Via Telegram: /approve → acción se ejecuta remotamente
6. Audit log: ver historial de todas las approvals con decisión y quién decidió
