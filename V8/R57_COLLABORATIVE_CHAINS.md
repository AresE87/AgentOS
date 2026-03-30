# FASE R57 — COLLABORATIVE CHAINS: El usuario corrige el rumbo mid-execution

**Objetivo:** Mientras una cadena de agentes se ejecuta, el usuario puede intervenir: "enfocate en pricing, no en features", "saltá el paso 3", "usá Claude en vez de GPT para esto". Los agentes incorporan la corrección sin reiniciar.

---

## Tareas

### 1. Chain input durante ejecución

```rust
// El Board ya tiene un input field (R6). Ahora hacerlo funcional:

#[tauri::command]
async fn inject_chain_context(chain_id: String, message: String) -> Result<(), String> {
    // 1. Agregar el mensaje al ChainContext
    // 2. El próximo agente en la cadena lo ve como "User intervention"
    // 3. Log en chain_log: "User injected: {message}"
    // 4. Emit evento para actualizar el Board
}
```

### 2. Acciones del usuario sobre sub-tareas

```
En el Board, cada card tiene acciones:
- [⏭ Skip] — Saltear esta sub-tarea
- [🔄 Retry] — Reintentar con diferente approach
- [✏ Edit] — Cambiar la descripción de la sub-tarea
- [🔀 Reassign] — Cambiar agente/modelo/nodo
- [⏹ Cancel] — Cancelar solo esta sub-tarea
```

### 3. Prompt injection segura

```rust
// El mensaje del usuario se inyecta así:
let enhanced_prompt = format!(
    "{}\n\n⚠️ USER INTERVENTION:\n{}\n\nIncorporate this feedback into your work.",
    original_prompt, user_message
);

// Si el usuario dice "skip step 3":
// Marcar step 3 como skipped → el orchestrator pasa al 4
// Si dice "use Claude instead of GPT":
// Cambiar el modelo para las sub-tareas restantes
```

### 4. Frontend: intervención inline en Board

```
BOARD — "Research and report on AI trends"      
─────────────────────────────────────────────────

[cards del kanban...]

── USER INTERVENTION ──────────────────────────
┌──────────────────────────────────────────────┐
│ 💬 Add context for the agents:               │
│ ┌──────────────────────────── [Send] ──────┐ │
│ │ Focus on open source AI, not commercial  │ │
│ └──────────────────────────────────────────┘ │
│                                               │
│ Quick actions:                                │
│ [Skip current step] [Change model] [Pause]   │
└──────────────────────────────────────────────┘

── AGENT LOG ──────────────────────────────────
10:42:15  USER          "Focus on open source AI, not commercial"
10:42:15  Orchestrator   Injecting user context into remaining tasks
10:42:16  Researcher     Adjusting research focus to open source AI
```

---

## Demo

1. Enviar tarea compleja → cadena empieza → 3 sub-tareas en Board
2. Mientras Research está en progress: "enfocate en Llama y Mistral, no en GPT"
3. El Researcher ajusta su investigación → menciona Llama y Mistral en el resultado
4. Click "Skip" en sub-tarea 2 → se marca como skipped → sub-tarea 3 ejecuta sin ella
5. Click "Retry" en sub-tarea fallida → se re-ejecuta con diferente approach
