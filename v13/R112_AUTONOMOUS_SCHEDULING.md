# FASE R112 — AUTONOMOUS SCHEDULING: Coordinar reuniones solo

**Objetivo:** "Agendame una reunión con Juan, María, y Pedro esta semana" → el agente chequea disponibilidad de TODOS, propone horarios, negocia por email, y confirma — sin que el usuario haga nada.

---

## Tareas

### 1. Multi-party availability check
- Chequear calendar propio (R63)
- Enviar email a los otros: "When are you available this week?"
- Parsear respuestas
- O: si tienen calendar compartido → chequear directamente

### 2. Negotiation loop
```
Agente → Juan: "Hi Juan, Edgardo wants to meet this week. Are you available Thu 3pm or Fri 10am?"
Juan → Reply: "Thu 3pm works"
Agente → María: "Thu 3pm — Edgardo and Juan confirmed. Works for you?"
María → Reply: "Can we do 4pm instead?"
Agente → Juan: "María prefers 4pm. Can you adjust?"
Juan → Reply: "Sure"
Agente → All: "Meeting confirmed: Thursday 4pm. Calendar invites sent."
```

### 3. Smart slot finding
```rust
pub fn find_optimal_slot(
    calendars: &[Calendar],
    duration: Duration,
    constraints: &SchedulingConstraints,
) -> Vec<TimeSlot> {
    // 1. Find overlapping free slots across all calendars
    // 2. Prefer: morning over afternoon, mid-week over Monday/Friday
    // 3. Avoid: lunch hours, before 9am, after 6pm
    // 4. Consider: timezone differences
    // 5. Return top 3 options ranked by convenience score
}
```

### 4. Rescheduling
```
"La reunión del jueves hay que moverla"
→ Agente: cancela la actual → propone nuevos horarios → negocia → confirma
Todo automático si confidence > threshold.
```

---

## Demo
1. "Agendame reunión con Juan y María esta semana" → 3 emails enviados → slots negociados → confirmado
2. Calendar shows: "Team Meeting — Thu 4pm" con todos los asistentes
3. Todos recibieron invite → aceptaron
4. "Mové la reunión al viernes" → re-negociación automática → moved
