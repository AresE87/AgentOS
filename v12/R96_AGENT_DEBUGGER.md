# FASE R96 — AGENT DEBUGGER: Step-through visual del pensamiento del agente

**Objetivo:** Un debugger visual donde el developer puede ver EXACTAMENTE qué pensó el agente en cada paso: qué prompt se envió, qué respondió el LLM, por qué eligió esa acción, qué datos tenía disponibles. Como un debugger de código pero para agentes.

---

## Tareas

### 1. Execution trace recording

```rust
pub struct ExecutionTrace {
    pub task_id: String,
    pub steps: Vec<TraceStep>,
}

pub struct TraceStep {
    pub step_number: usize,
    pub timestamp: DateTime<Utc>,
    pub phase: String,              // "classify", "route", "select_agent", "generate_prompt", "llm_call", "parse_response", "execute_action", "verify"
    pub input: serde_json::Value,   // Qué entró a este paso
    pub output: serde_json::Value,  // Qué salió
    pub decision: Option<String>,   // "Selected claude-sonnet because tier=2 and type=code"
    pub prompt_sent: Option<String>,// El prompt completo que se envió al LLM
    pub llm_response: Option<String>,// La respuesta raw del LLM
    pub duration_ms: u64,
    pub cost: f64,
    pub tokens: Option<(u32, u32)>, // (input, output)
}
```

### 2. Trace capture en el pipeline

```rust
// En CADA punto de decisión del pipeline, registrar:
// 1. Classifier: "Input: 'check disk'. Decision: type=command, complexity=1, tier=1"
// 2. Router: "Tier 1 → trying gpt-4o-mini. Available: [mini, flash, haiku]"
// 3. Agent selector: "Selected 'System Admin' because keywords match: [disk, system]"
// 4. Prompt builder: [el prompt completo, con system prompt + context + user message]
// 5. LLM call: [la respuesta raw]
// 6. Response parser: "Parsed as: command mode, command='Get-PSDrive C'"
// 7. Action executor: "Running PowerShell: Get-PSDrive C → exit code 0"
// 8. Verifier: "Output contains disk data → success"
```

### 3. Frontend: Debug viewer

```
DEBUGGER: Task "check disk space"            [Step 1 of 8]
──────────────────────────────────────────────────────────

TIMELINE
[1 Classify] → [2 Route] → [3 Agent] → [4 Prompt] → [5 LLM] → [6 Parse] → [7 Execute] → [8 Verify]
     ✅           ✅          ✅          ✅          ✅         ✅          ✅           ✅

STEP 5: LLM Call                              2.1s · $0.001
────────────────────────────────────────────────────────────

Provider: OpenAI
Model: gpt-4o-mini
Tokens: 234 in / 89 out

PROMPT SENT:
┌─────────────────────────────────────────────────────┐
│ System: You are a System Administrator...           │
│                                                      │
│ User: check disk space                               │
│                                                      │
│ Respond with JSON: {"mode": "...", "command": "..."}│
└─────────────────────────────────────────────────────┘

LLM RESPONSE:
┌─────────────────────────────────────────────────────┐
│ {"mode": "command", "command": "Get-PSDrive C |     │
│ Select-Object Used, Free, @{N='Total';E={...}}"}   │
└─────────────────────────────────────────────────────┘

DECISION: "LLM chose command mode with PowerShell command"

[◀ Prev] [Next ▶] [Re-run this step] [Edit prompt & retry]
```

### 4. "Edit & Retry" (iterative debugging)

```
El developer puede:
1. Pausar en cualquier step
2. Editar el prompt
3. Re-ejecutar desde ese punto
4. Ver si el resultado mejora

Esto es invaluable para:
- Mejorar system prompts de specialists
- Debuggear playbooks que fallan
- Entender por qué el router eligió el modelo incorrecto
- Optimizar prompts para reducir tokens/costo
```

### 5. Comparison mode

```
Comparar dos ejecuciones de la misma tarea side by side:
- ¿Qué prompt cambió?
- ¿El LLM respondió diferente?
- ¿Cuánto más/menos costó?
- ¿Cuál fue más preciso?

Útil para A/B testing de prompts y specialists.
```

### 6. IPC commands

```rust
#[tauri::command] async fn debug_get_trace(task_id: String) -> Result<ExecutionTrace, String>
#[tauri::command] async fn debug_replay_step(task_id: String, step: usize, modified_prompt: Option<String>) -> Result<TraceStep, String>
#[tauri::command] async fn debug_compare(task_id_a: String, task_id_b: String) -> Result<TraceComparison, String>
#[tauri::command] async fn debug_enable_tracing(enabled: bool) -> Result<(), String>
```

---

## Demo

1. Ejecutar tarea → ir a Debugger → ver los 8 steps con timeline
2. Step 5 (LLM): ver el prompt EXACTO que se envió y la respuesta EXACTA
3. Step 3 (Agent): ver POR QUÉ eligió "System Admin" y no "Programmer"
4. Edit prompt en Step 5 → re-run → resultado diferente → comparar
5. Comparison: misma tarea con 2 modelos diferentes → ver cuál fue mejor y por qué
