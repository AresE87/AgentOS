# FASE R64 — EMAIL INTEGRATION: El agente maneja tu correo

**Objetivo:** El agente lee emails, los clasifica por prioridad, genera drafts de respuesta, y puede enviar con aprobación. "Respondele a Juan que acepto la reunión" → draft generado → approve → enviado.

---

## Tareas

### 1. Email provider abstraction

```rust
pub trait EmailProvider: Send + Sync {
    async fn list_messages(&self, folder: &str, limit: usize) -> Result<Vec<EmailSummary>>;
    async fn get_message(&self, id: &str) -> Result<EmailMessage>;
    async fn send(&self, message: NewEmail) -> Result<()>;
    async fn create_draft(&self, message: NewEmail) -> Result<DraftEmail>;
    async fn search(&self, query: &str) -> Result<Vec<EmailSummary>>;
    async fn move_to(&self, id: &str, folder: &str) -> Result<()>;
    async fn mark_read(&self, id: &str) -> Result<()>;
}

pub struct GmailProvider { /* Google OAuth tokens */ }
pub struct OutlookProvider { /* Microsoft Graph tokens */ }
```

### 2. OAuth flow (similar a R63)

```
// Gmail: scopes gmail.readonly, gmail.send, gmail.modify
// Outlook: scopes Mail.ReadWrite, Mail.Send
```

### 3. Email AI triage

```rust
// El agente clasifica cada email no leído:
pub struct EmailTriage {
    pub priority: Priority,        // Urgent, Important, Normal, Low
    pub category: String,          // "meeting", "invoice", "newsletter", "spam"
    pub suggested_action: String,  // "Reply", "Archive", "Forward to Juan", "Unsubscribe"
    pub draft_reply: Option<String>, // Draft si se puede auto-responder
}

// Ejecutar triage en background cada 5 minutos (si habilitado)
```

### 4. Natural language email actions

```
"Qué emails nuevos tengo" → lista de inbox no leídos con triage
"Leé el email de Juan" → muestra contenido
"Respondele que acepto" → genera draft → approval → envía
"Mandále un email a María con el reporte adjunto" → compose + attach + approval
"Buscá emails sobre facturación del mes pasado" → search → resultados
"Archivá todos los newsletters" → bulk move
```

### 5. Frontend: Email panel (sub-sección o integrado en Chat)

```
📧 INBOX (3 unread)                    [Refresh] [Settings]
┌──────────────────────────────────────────────────────┐
│ 🔴 Juan García — "Urgente: revisión del contrato"    │
│    Suggested: Reply — Draft ready                     │
│    [Reply] [Archive] [View]                           │
│                                                       │
│ 🟡 AWS — "Your monthly invoice"                       │
│    Suggested: Archive — Amount: $45.67                │
│    [Archive] [View]                                   │
│                                                       │
│ ⚪ Newsletter Tech — "Top 10 AI tools"                │
│    Suggested: Archive or Unsubscribe                  │
│    [Archive] [Unsubscribe] [View]                     │
└──────────────────────────────────────────────────────┘
```

### 6. Integration con approval workflow (R62)

Enviar email = action de riesgo HIGH → pasa por approval workflow automáticamente.

---

## Demo

1. Conectar Gmail → "Connected ✅"
2. "Qué emails nuevos tengo" → lista con triage (priority + suggested action)
3. "Respondele a Juan que acepto" → draft generado → approval dialog → enviar
4. "Buscá facturas del mes pasado" → resultados reales de Gmail
5. Email panel en Home muestra inbox con colores por prioridad
