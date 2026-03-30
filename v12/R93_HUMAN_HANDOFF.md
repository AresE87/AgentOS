# FASE R93 — HUMAN HANDOFF: Si el agente no puede, escala a un humano

**Objetivo:** Cuando el agente no puede resolver algo (confianza baja, tarea ambigua, error repetido), escala automáticamente a un humano — ya sea el propio usuario, un compañero de equipo, o un especialista externo — con TODO el contexto de lo que ya intentó.

---

## Tareas

### 1. Escalation detection

```rust
pub struct EscalationDetector;

impl EscalationDetector {
    pub fn should_escalate(task: &TaskContext) -> Option<EscalationReason> {
        // Escalar cuando:
        if task.retry_count >= 3 { return Some(TooManyRetries); }
        if task.llm_confidence < 0.4 { return Some(LowConfidence); }
        if task.involves_money() { return Some(FinancialAction); }
        if task.user_said_wrong() { return Some(UserFeedback); }
        if task.requires_credentials_not_in_vault() { return Some(MissingCredentials); }
        if task.external_system_down() { return Some(SystemUnavailable); }
        None
    }
}
```

### 2. Escalation targets

```rust
pub enum EscalationTarget {
    Self_,           // Notificar al mismo usuario (tarea compleja, necesita input)
    TeamMember(String), // Escalar a un compañero específico
    TeamPool,        // Escalar al equipo (cualquiera puede tomar)
    External(String), // Escalar a un especialista externo (ej: soporte AgentOS)
}
```

### 3. Handoff con contexto completo

```rust
pub struct HandoffPackage {
    pub task_id: String,
    pub original_request: String,
    pub what_was_tried: Vec<AttemptSummary>,
    pub agent_analysis: String,      // "I couldn't complete this because..."
    pub relevant_data: Vec<String>,  // Archivos, screenshots, outputs parciales
    pub suggested_action: String,    // "A human should verify the bank account number"
    pub urgency: Urgency,            // Low, Normal, High, Critical
    pub escalated_at: DateTime<Utc>,
}

// El humano recibe TODO esto y puede:
// 1. Resolver manualmente
// 2. Dar instrucciones al agente para que reintente
// 3. Re-asignar a otro humano
// 4. Cerrar como "no se puede"
```

### 4. Notification al humano

```
Vía desktop:
┌──────────────────────────────────────────────────┐
│ 🆘 Agent needs human help                        │
│                                                   │
│ Task: "Transfer $5,000 to supplier account"       │
│ Agent tried: Verified amount, found account #     │
│ Blocked: "I can't verify the bank account is     │
│ correct. A human should confirm before transfer." │
│                                                   │
│ [Take over] [Instruct agent] [Reassign] [Close]  │
└──────────────────────────────────────────────────┘

Vía Telegram/WhatsApp:
🆘 AgentOS needs your help
Task: "Transfer $5,000 to supplier"
I couldn't verify the bank account.
Reply /takeover to handle manually
Reply /instruct to give me more info
```

### 5. Human → Agent feedback loop

```
Cuando el humano resuelve:
1. El resultado se registra como training data
2. El agente aprende: "para tareas de transferencia bancaria, siempre pedir confirmación humana"
3. Próxima vez: el agente pide confirmación ANTES de intentar (no después de fallar)
```

### 6. Escalation dashboard

```
ESCALATIONS                              [Settings]
──────────────────────────────────────────
PENDING (2)
┌──────────────────────────────────────────────┐
│ 🔴 "Transfer $5,000"           HIGH          │
│    Agent tried 3 times, can't verify account │
│    Escalated: 10 min ago                      │
│    [Take over] [Instruct] [Reassign]          │
│                                               │
│ 🟡 "Create contract for Acme"   NORMAL       │
│    Agent needs legal review before sending    │
│    Escalated: 1 hour ago                      │
│    [Take over] [Instruct] [Reassign]          │
└──────────────────────────────────────────────┘

RESOLVED (last 7 days): 12
  Avg resolution time: 23 minutes
  Most common reason: Missing credentials (5)
```

---

## Demo

1. Tarea que falla 3 veces → "Agent needs help" notification → human takes over → resolved
2. Tarea financiera → agente pausa ANTES de ejecutar → "Confirm bank account" → humano confirma → ejecuta
3. Via Telegram: /takeover → humano resuelve desde el teléfono
4. Feedback loop: después de resolver → próxima vez el agente pide confirmación directamente
5. Escalation dashboard: 2 pending, 12 resolved this week, avg 23min resolution
