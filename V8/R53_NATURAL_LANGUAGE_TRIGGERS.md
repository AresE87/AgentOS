# FASE R53 — NATURAL LANGUAGE TRIGGERS: Automatizar en español, no en cron

**Objetivo:** El usuario escribe "avisame todos los lunes a las 9" o "cuando aparezca un archivo nuevo en Downloads, organizalo" en lenguaje natural. El agente traduce a trigger internamente.

---

## Tareas

### 1. Natural language → trigger parser

```rust
// El LLM convierte lenguaje natural a trigger config:

pub async fn parse_trigger_from_natural_language(text: &str) -> Result<TriggerConfig> {
    let prompt = format!(r#"
Convert this natural language automation request into a trigger configuration.
Respond ONLY with JSON.

Input: "{}"

Output format:
{{
  "type": "cron" | "file_watch" | "condition",
  "name": "short descriptive name",
  "schedule": "cron expression (if type=cron)",
  "path": "directory path (if type=file_watch)",
  "event": "created | modified | deleted (if type=file_watch)",
  "condition": "natural language condition to check (if type=condition)",
  "check_interval_minutes": number (if type=condition),
  "task": "what to do when triggered"
}}

Examples:
"avisame todos los lunes a las 9" → {{"type":"cron","schedule":"0 9 * * MON","task":"send morning briefing"}}
"cuando haya un archivo nuevo en Downloads, organizalo" → {{"type":"file_watch","path":"~/Downloads","event":"created","task":"organize this file: {{filename}}"}}
"si el disco pasa del 90%, avisame" → {{"type":"condition","condition":"disk usage > 90%","check_interval_minutes":60,"task":"alert: disk usage critical"}}
"#, text);

    let response = gateway.cheap_call(&prompt).await?;
    serde_json::from_str(&response)
}
```

### 2. Condition triggers (NUEVO tipo)

```rust
// Además de cron y file_watch, agregar "condition":
// El agente chequea periódicamente una condición

pub struct ConditionTrigger {
    pub condition: String,           // "disk usage > 90%"
    pub check_command: String,       // PowerShell que evalúa la condición
    pub check_interval: Duration,    // Cada cuánto chequear
    pub task_on_true: String,        // Qué hacer si la condición es true
    pub last_state: bool,            // Para detectar cambio (no repetir)
}

// El LLM genera el check_command a partir de la condición en lenguaje natural:
// "disk usage > 90%" → "if ((Get-WmiObject Win32_LogicalDisk -Filter 'DeviceID=\"C:\"').FreeSpace / (Get-WmiObject Win32_LogicalDisk -Filter 'DeviceID=\"C:\"').Size * 100) -lt 10) { 'true' } else { 'false' }"
```

### 3. Frontend: Crear trigger en lenguaje natural

```
AUTOMATION                                    [+ New]

Click [+ New]:
┌──────────────────────────────────────────────────┐
│ Describe your automation in plain language:        │
│                                                    │
│ ┌────────────────────────────────────────────────┐│
│ │ Every Monday at 9am, check my disk space and   ││
│ │ send me a summary on Telegram                  ││
│ └────────────────────────────────────────────────┘│
│                                                    │
│ [Create Automation]                                │
│                                                    │
│ Preview:                                           │
│ ⏰ Type: Scheduled (cron)                          │
│ 📅 Schedule: Every Monday at 9:00 AM               │
│ 📋 Action: "Check disk space and send summary      │
│            on Telegram"                            │
│                                                    │
│ [Looks good, save it] [Edit manually]              │
└──────────────────────────────────────────────────┘
```

### 4. Ejemplos que deben funcionar

```
"Todos los viernes a las 6pm, generame un resumen de la semana"
→ cron: 0 18 * * FRI, task: "generate weekly summary"

"Cuando alguien me mande un PDF por Telegram, extraé los datos y ponelos en un Excel"
→ condition on telegram message containing PDF

"Si mi sitio web no responde, avisame por WhatsApp"
→ condition: check http://mysite.com every 5 min, task: alert on WhatsApp

"Cada vez que guarde un archivo .py, corré los tests"
→ file_watch: *.py modified, task: "run python tests"

"El primer día de cada mes, generá el reporte de facturación"
→ cron: 0 9 1 * *, task: "generate billing report"
```

---

## Demo

1. Escribir "avisame todos los lunes a las 9 cómo está mi disco" → trigger creado automáticamente
2. Preview muestra: cron schedule + acción en español
3. El trigger se ejecuta (testear con "cada 1 minuto" primero)
4. "Cuando aparezca un archivo en Downloads, decime qué es" → file watcher funciona
5. "Si el disco pasa del 80%, avisame" → condition checker funciona
