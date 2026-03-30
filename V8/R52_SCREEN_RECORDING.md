# FASE R52 — SCREEN RECORDING + REPLAY: Ver lo que el agente hizo

**Objetivo:** Cuando el agente ejecuta una tarea con vision mode, graba un video de la pantalla mostrando cada acción. El usuario puede ver el replay después: "¿qué hizo exactamente?". También útil para auditoría enterprise.

---

## Tareas

### 1. Screen recorder durante ejecución

```rust
// Nuevo: src-tauri/src/eyes/recorder.rs

pub struct ScreenRecorder {
    frames: Vec<Frame>,
    recording: bool,
    fps: u32,              // 2-5 fps es suficiente (no es video gaming)
}

pub struct Frame {
    pub screenshot: Vec<u8>,    // JPEG comprimido
    pub timestamp_ms: u64,
    pub action: Option<String>, // "Clicked at (500, 300)" | "Typed 'hello'"
    pub agent_thought: Option<String>,  // "Looking for the Save button"
}

impl ScreenRecorder {
    pub fn start(&mut self);
    pub fn capture_frame(&mut self, action: Option<&str>, thought: Option<&str>);
    pub fn stop(&mut self) -> Recording;
}

pub struct Recording {
    pub task_id: String,
    pub frames: Vec<Frame>,
    pub duration_ms: u64,
    pub total_actions: usize,
}
```

### 2. Integrar en el vision pipeline

```rust
// En pipeline/engine.rs, modo screen/command_then_screen:
// Antes de cada acción: recorder.capture_frame()
// Con la acción del agente y su "pensamiento" como metadata

async fn vision_loop(task: &str, state: &AppState) -> Result<TaskResult> {
    let mut recorder = ScreenRecorder::new(fps: 2);
    recorder.start();
    
    for step in 0..max_steps {
        let screenshot = capture_screen()?;
        let analysis = vision_analyze(&screenshot, task).await?;
        
        recorder.capture_frame(
            Some(&format!("{:?}", analysis.action)),
            Some(&analysis.description),
        );
        
        execute_action(&analysis.action).await?;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    
    let recording = recorder.stop();
    save_recording(&recording).await?;
    Ok(result)
}
```

### 3. Guardar recordings

```rust
// Guardar como directorio de frames + metadata JSON:
// recordings/{task_id}/
//   ├── metadata.json    ← {task_id, duration, total_frames, actions}
//   ├── frame_0000.jpg
//   ├── frame_0001.jpg
//   └── ...

// O como GIF animado para fácil sharing:
// recordings/{task_id}.gif
```

### 4. Frontend: Replay player

```
TASK DETAIL                                    [▶ Watch Replay]
──────────────────────────────────────────────────

Click "Watch Replay":
┌──────────────────────────────────────────────────┐
│ REPLAY: "Install 7-Zip"             [⏮][▶][⏭]  │
│ Step 3 of 12                     ⏱ 0:15 / 0:45  │
│                                                   │
│ ┌───────────────────────────────────────────────┐ │
│ │                                               │ │
│ │           [screenshot del frame]              │ │
│ │                                               │ │
│ │         ● Click here (500, 300)               │ │  ← overlay mostrando la acción
│ │                                               │ │
│ └───────────────────────────────────────────────┘ │
│                                                   │
│ 🤖 "Found the Download button. Clicking it."     │  ← pensamiento del agente
│                                                   │
│ Timeline: ●──●──●──●──●──●──[●]──●──●──●──●──●  │  ← cada dot es un frame
│           click  type  wait  click  ...           │
│                                                   │
│ [◀ Prev step]                      [Next step ▶]  │
└──────────────────────────────────────────────────┘
```

### 5. IPC commands

```rust
#[tauri::command] async fn get_recording(task_id: String) -> Result<RecordingMeta, String>
#[tauri::command] async fn get_recording_frame(task_id: String, frame: usize) -> Result<FrameData, String>
#[tauri::command] async fn export_recording_gif(task_id: String) -> Result<String, String>  // path al GIF
#[tauri::command] async fn delete_recording(task_id: String) -> Result<(), String>
```

### 6. Storage management

```rust
// Los recordings ocupan espacio (cada frame ~50KB JPEG):
// 12 frames × 50KB = 600KB por tarea visual
// 100 tareas visuales = 60MB

// Auto-cleanup: borrar recordings > 30 días (configurable en Settings)
// Settings: "Keep recordings for: [7 days | 30 days | 90 days | forever]"
```

---

## Demo

1. "Abre la calculadora y calcula 42 × 18" → tarea ejecuta con vision
2. Ir al detalle de la tarea → "Watch Replay" → ver frame por frame cómo el agente navegó
3. Cada frame muestra: screenshot + overlay de la acción + pensamiento del agente
4. Exportar como GIF → archivo compartible
5. Recordings auto-cleanup: configurar 7 días → los viejos se borran
