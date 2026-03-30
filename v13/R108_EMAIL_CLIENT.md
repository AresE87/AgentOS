# FASE R108 — EMAIL CLIENT EMBEBIDO: Gmail/Outlook dentro de AgentOS

**Objetivo:** Un cliente de email completo DENTRO de AgentOS. No solo leer y enviar (R64) — un inbox real con AI triage automático, smart compose, follow-up reminders, y email analytics. El usuario puede abandonar Gmail/Outlook.

---

## Tareas

### 1. Full email client UI
```
EMAIL                              [Compose] [Refresh]
─────────────────────────────────────────────────────
INBOX (3 unread)                         [AI Triage ▾]

┌─ 🔴 URGENT ────────────────────────────────────────┐
│ Juan García — "Contract review needed"    10:30 AM  │
│ Preview: Please review the attached contract...     │
│ 🤖 Suggested: Reply with review notes              │
│ [Reply] [Forward] [Archive] [AI Draft]              │
└─────────────────────────────────────────────────────┘

┌─ 🟡 IMPORTANT ─────────────────────────────────────┐
│ AWS — "Your March invoice: $45.67"         9:15 AM  │
│ 🤖 Suggested: Archive (routine billing)             │
│ [Archive] [View] [Process invoice]                  │
└─────────────────────────────────────────────────────┘

┌─ ⚪ NORMAL ─────────────────────────────────────────┐
│ Tech Newsletter — "Top 10 AI tools"        8:00 AM  │
│ 🤖 Suggested: Read later or unsubscribe             │
│ [Read later] [Unsubscribe] [Archive]                │
└─────────────────────────────────────────────────────┘

FOLDERS: Inbox | Sent | Drafts | Archive | Spam
LABELS: 🔴 Urgent | 🟡 Important | 📎 Has attachment | 💰 Invoice
```

### 2. AI features en email
- **Auto-triage:** Cada email se clasifica: Urgent/Important/Normal/Spam
- **Smart compose:** "Reply that I accept" → full professional reply drafted
- **Follow-up reminders:** "I haven't heard back from Juan in 3 days" → reminder
- **Email analytics:** Response time avg, busiest hours, top senders
- **Unsubscribe manager:** Detecta newsletters → "Unsubscribe from 12 newsletters?"
- **Attachment processor:** PDF invoices → auto-extract data → offer to process

### 3. Email-to-task bridge
```
Un email puede convertirse en tarea:
"Process this invoice" → agente extrae datos del PDF adjunto → guarda en DB
"Schedule a meeting about this" → agente lee el email → crea evento en calendar
"Research this topic" → agente investiga lo que se menciona en el email
```

### 4. IMAP/SMTP support (para email providers no-Google/MS)
```rust
// Además de Gmail API y Outlook Graph:
// IMAP para cualquier proveedor
// SMTP para enviar
// Esto cubre: ProtonMail, Fastmail, Yahoo, corporate email

pub struct IMAPProvider {
    host: String,      // "imap.gmail.com"
    port: u16,         // 993
    username: String,
    password: String,   // App-specific password (del vault)
}
```

---

## Demo
1. Abrir Email section → inbox real con emails triageados por AI
2. Email urgente → "AI Draft" → reply profesional generado → editar → enviar
3. Invoice email → "Process invoice" → datos extraídos automáticamente
4. "Unsubscribe manager" → 8 newsletters detectadas → unsubscribe batch
5. Email analytics: "You respond to Juan in avg 2.3 hours"
