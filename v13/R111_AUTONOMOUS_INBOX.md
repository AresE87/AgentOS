# FASE R111 — AUTONOMOUS INBOX: El agente maneja tu email solo

**Objetivo:** Para emails rutinarios (confirmaciones, newsletters, facturas, notificaciones), el agente responde, archiva, o procesa SIN que el usuario toque nada. Solo escala al humano los emails que realmente necesitan atención.

---

## Tareas

### 1. Email classification engine
```rust
pub enum EmailAction {
    AutoReply(String),          // "Thanks, confirmed!" — auto-send
    AutoArchive,                // Newsletter, notification — archive silently
    AutoProcess(String),        // Invoice → extract data → save to DB
    AutoForward(String),        // Forward to the right person
    NeedsHuman(HumanReason),    // Escalate with context
}

// Clasificar cada email:
// - Confirmación de reunión → AutoReply("Confirmed, see you then!")
// - Newsletter → AutoArchive
// - Factura PDF → AutoProcess("extract invoice data")
// - Email de cliente nuevo → NeedsHuman("New client, needs personal response")
// - Email del jefe → NeedsHuman("From manager, likely important")
```

### 2. Learning from corrections
```
Si el agente auto-responde y el usuario dice "eso estuvo mal":
→ Registrar la corrección
→ Ajustar el clasificador
→ Próxima vez: escalar ese tipo de email al humano

Si el agente escala al humano y el humano dice "esto lo podrías haber manejado":
→ Registrar como auto-handleable
→ Próxima vez: auto-responder emails similares
```

### 3. Confidence-based autonomy
```
Confidence > 95%: auto-act (no preguntar)
Confidence 80-95%: auto-act + notify user (puede revertir)
Confidence 60-80%: draft action + ask user to confirm
Confidence < 60%: escalate to human

El threshold es configurable por usuario.
Default: conservative (solo auto-act > 95%)
```

### 4. Daily inbox summary
```
📧 INBOX SUMMARY — March 29, 2026
─────────────────────────────────
Emails received today: 34
  Auto-replied: 8 (confirmations, thank yous)
  Auto-archived: 15 (newsletters, notifications)
  Auto-processed: 3 (invoices extracted)
  Forwarded: 2 (to María for accounting)
  Needs your attention: 6 ← only these need you

🕐 Time saved today: ~45 minutes
```

### 5. Settings: autonomy levels
```
AUTONOMOUS INBOX
  Mode: [Conservative ▾]
    Conservative: only auto-archive newsletters
    Standard: auto-reply confirmations + archive + process invoices
    Aggressive: handle everything except flagged senders
    
  Always escalate from: [boss@company.com, ceo@company.com]
  Never auto-reply to: [clients, new contacts]
  Auto-process invoices: [ON] → save to [Accounting DB ▾]
  
  Review auto-actions: [Daily summary ▾] / Per action / Never
```

---

## Demo
1. 34 emails llegan → agente procesa 28 automáticamente → solo 6 necesitan humano
2. Invoice → auto-extracted → datos guardados en DB → notification: "Invoice processed"
3. Newsletter → auto-archived (invisible para el usuario)
4. Confirmación de reunión → auto-reply "Confirmed!" → calendar updated
5. Daily summary: "Saved 45 minutes today. 6 emails need your attention."
