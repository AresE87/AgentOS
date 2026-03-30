# FASE R102 — WEARABLE INTEGRATION: AgentOS en tu muñeca

**Objetivo:** Apple Watch / Wear OS: voice commands rápidos, notifications con acciones, approval de un tap, y glances de status. "Hey Agent, ¿cuántas tareas hoy?" → vibra → respuesta en la muñeca.

---

## Tareas

### 1. Watch app (WatchOS + Wear OS)
- Complication: status del agente (idle/working/tasks count)
- Main screen: voice input button + last 3 notifications
- Quick actions: approve/reject (para R62 approval workflows)
- Glance: stats del día (tasks, cost, next scheduled)

### 2. Comunicación watch ↔ phone ↔ desktop
```
Watch → Phone (companion app R27) → API → Desktop AgentOS
Watch solo necesita: voice input, notifications, simple actions
El procesamiento pesado lo hace el desktop
```

### 3. Haptic notifications
- Tarea completada: tap suave
- Approval needed: tap fuerte + vibración patrón
- Error: triple tap
- Scheduled task ejecutada: tap + chime

### 4. Voice shortcuts
- Raise wrist → "Check my tasks" → respuesta
- "Approve" → aprueba la última acción pendiente
- "Skip" → saltea el trigger actual
- "Status" → estado del agente + stats

---

## Demo
1. Levantar muñeca → "How many tasks today?" → "42 tasks, $0.34 cost" en 2 segundos
2. Approval notification → tap "Approve" → acción ejecutada en el desktop
3. Complication muestra ícono con status (cyan = working, gris = idle)
