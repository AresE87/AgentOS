# FASE R4 — PLAYBOOKS VIVOS: Grabar y reproducir tareas

**Objetivo:** El usuario puede grabar una tarea (screenshots + acciones), guardarla como playbook, y reproducirla después. TODO con UI en el dashboard.

**Prerequisito:** R2 (vision funciona) + R3 (frontend conectado)

---

## Estado actual

- `playbooks/recorder.rs` — existe, graba acciones + screenshots como JSON
- `playbooks/player.rs` — existe, reproduce playbooks
- Frontend: NO tiene UI para recorder ni player
- El directorio de playbooks existe, puede tener archivos

## Problema

Los playbooks son el feature más diferenciador de AgentOS (vs la competencia). Pero hoy son código muerto — el backend existe, el frontend no.

---

## Tareas

### 1. IPC commands para playbooks (verificar/crear en lib.rs)

```rust
#[tauri::command] async fn start_recording(name: String) -> Result<(), String>
// Inicia el recorder: captura screenshots en cada acción significativa

#[tauri::command] async fn stop_recording() -> Result<PlaybookSummary, String>
// Para el recorder, guarda el playbook, retorna resumen

#[tauri::command] async fn play_playbook(name: String) -> Result<(), String>
// Reproduce un playbook paso a paso con vision mode

#[tauri::command] async fn get_playbook_detail(name: String) -> Result<PlaybookDetail, String>
// Retorna: nombre, descripción, steps con thumbnails, config

#[tauri::command] async fn delete_playbook(name: String) -> Result<(), String>
```

### 2. Frontend: Playbook Detail View

Cuando el usuario clickea un playbook en la lista:

```
┌─────────────────────────────────────────────────┐
│ ← Back to Playbooks                             │
│                                                  │
│ 📘 System Monitor                                │
│ Monitors PC health: CPU, memory, disk            │
│                                                  │
│ Config: Tier 1 · CLI · timeout 30s               │
│                                                  │
│ STEPS (3)                                        │
│ ┌────────────────────────────────────────────┐   │
│ │ [thumbnail] Step 1: Open PowerShell        │   │
│ │ [thumbnail] Step 2: Run systeminfo         │   │
│ │ [thumbnail] Step 3: Parse output           │   │
│ └────────────────────────────────────────────┘   │
│                                                  │
│ [▶ Play]  [✏ Edit]  [🗑 Delete]                  │
└─────────────────────────────────────────────────┘
```

### 3. Frontend: Recorder UI

Botón "Record New Playbook" en la página Playbooks:

```
┌─────────────────────────────────────────────────┐
│ RECORDING: "My Task"                    [⏹ Stop] │
│                                                  │
│ ● Recording... 3 steps captured                  │
│                                                  │
│ Steps so far:                                    │
│ 1. Opened PowerShell            [screenshot]     │
│ 2. Typed "ipconfig"             [screenshot]     │
│ 3. Waiting for next action...                    │
│                                                  │
│ Tip: Perform the task normally.                  │
│ AgentOS is watching and learning.                │
└─────────────────────────────────────────────────┘
```

Flujo:
1. User clicks "Record New Playbook"
2. Dialog pide nombre y descripción
3. invoke("start_recording", {name})
4. AgentOS se minimiza, usuario hace la tarea
5. Cada acción significativa (click, typing, window change) genera un step con screenshot
6. Usuario vuelve a AgentOS y clicks "Stop Recording"
7. invoke("stop_recording") → guarda el playbook
8. Playbook aparece en la lista de instalados

### 4. Frontend: Player UI

Cuando el usuario clickea "Play" en un playbook:

```
┌─────────────────────────────────────────────────┐
│ PLAYING: "System Monitor"              [⏹ Stop]  │
│                                                  │
│ Step 2 of 3                                      │
│ ██████████████░░░░░░ 66%                         │
│                                                  │
│ Current: Running systeminfo command              │
│ [live screenshot of what the agent sees]         │
│                                                  │
│ Log:                                             │
│ ✅ Step 1: Opened PowerShell                     │
│ ⏳ Step 2: Running systeminfo (in progress)      │
│ ○  Step 3: Parse output (waiting)                │
└─────────────────────────────────────────────────┘
```

### 5. Backend: Mejorar recorder para capturar screenshots reales

Verificar que `playbooks/recorder.rs` hace:
- Screenshot (via `eyes/capture.rs`) en cada evento:
  - Mouse click
  - Key press significativo (Enter, Tab, window switch)
  - Después de cada acción completada
- Guarda cada step como: `{action_type, screenshot_path, description, timestamp}`
- Los screenshots se guardan en `playbooks/{name}/steps/`

### 6. Backend: Mejorar player para usar vision

Verificar que `playbooks/player.rs` hace:
- Para cada step del playbook:
  1. Capturar pantalla actual
  2. Comparar con el screenshot del step (enviar ambos al LLM vision)
  3. LLM decide: "la pantalla actual se parece al step? Qué acción tomar?"
  4. Ejecutar la acción
  5. Verificar que la pantalla cambió
  6. Siguiente step

---

## Cómo verificar

1. Abrir Playbooks → "Record New Playbook" → grabar: abrir Notepad, escribir "hello", guardar
2. Volver a Playbooks → el nuevo playbook aparece con 3+ steps y thumbnails
3. Click "Play" → AgentOS reproduce: abre Notepad, escribe "hello", guarda
4. Los steps se muestran en la UI con progreso en tiempo real

---

## NO hacer

- No implementar marketplace (es otra cosa)
- No agregar CLIP embeddings (overkill para v1 de playbooks)
- No intentar playbooks cross-application complejos (empezar simple)
