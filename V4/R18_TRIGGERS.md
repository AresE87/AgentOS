# FASE R18 — TRIGGERS Y AUTOMATIZACIÓN: El agente actúa solo

**Objetivo:** El usuario configura acciones automáticas: cada lunes a las 9am checkear el disco, cada vez que aparece un archivo en Downloads organizarlo, un webhook que ejecuta una tarea.

---

## Tareas

### 1. Cron scheduler

```rust
// Nuevo módulo: automation/scheduler.rs
// Crate: cron (para parsear expresiones cron) + tokio para scheduling

struct ScheduledTask {
    id: String,
    name: String,
    cron: String,           // "0 9 * * MON" = lunes 9am
    task_text: String,      // Lo que se envía al agente
    playbook: Option<String>, // Playbook a usar (opcional)
    enabled: bool,
    last_run: Option<DateTime>,
    next_run: DateTime,
}

// Loop:
loop {
    for task in scheduled_tasks.iter().filter(|t| t.enabled && t.next_run <= now) {
        execute_scheduled_task(task).await;
        task.last_run = Some(now);
        task.next_run = calculate_next(task.cron);
    }
    tokio::time::sleep(Duration::from_secs(30)).await;
}
```

### 2. File watcher

```rust
// Crate: notify (cross-platform file watching)

struct FileWatcher {
    path: PathBuf,          // Directorio a monitorear
    event: String,          // "created", "modified", "deleted"
    task_text: String,      // Template: "Organize this file: {filename}"
    enabled: bool,
}

// Cuando se detecta un evento:
notify::recommended_watcher(move |event| {
    if event.kind == Create {
        let task = task_text.replace("{filename}", &event.paths[0].display());
        execute_trigger_task(&task).await;
    }
});
```

### 3. Frontend: Triggers page (sub-section de Settings o nueva sección)

```
AUTOMATION                              [+ New Trigger]
─────────────────────────────────────
SCHEDULED
┌──────────────────────────────────────────────┐
│ ⏰ Monday disk check          [ON] [Edit] [🗑] │
│    Every Monday at 9:00 AM                    │
│    "Check disk space and report"              │
│    Last run: Mar 25, 9:00 AM — ✅             │
│    Next run: Mar 31, 9:00 AM                  │
└──────────────────────────────────────────────┘

FILE WATCHERS
┌──────────────────────────────────────────────┐
│ 📁 Downloads organizer        [ON] [Edit] [🗑] │
│    Watching: C:\Users\X\Downloads             │
│    On: new file created                       │
│    "Organize this file: {filename}"           │
│    Last triggered: 2 hours ago — ✅            │
└──────────────────────────────────────────────┘
```

### 4. SQLite table para triggers

```sql
CREATE TABLE IF NOT EXISTS triggers (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    type        TEXT NOT NULL,  -- "cron", "file_watch"
    config      TEXT NOT NULL,  -- JSON: {cron: "...", path: "...", event: "..."}
    task_text   TEXT NOT NULL,
    playbook    TEXT,
    enabled     INTEGER DEFAULT 1,
    last_run    TEXT,
    created_at  TEXT NOT NULL
);
```

### 5. IPC commands

```rust
#[tauri::command] async fn get_triggers() -> Result<Vec<Trigger>, String>
#[tauri::command] async fn create_trigger(trigger: TriggerInput) -> Result<String, String>
#[tauri::command] async fn update_trigger(id: String, trigger: TriggerInput) -> Result<(), String>
#[tauri::command] async fn delete_trigger(id: String) -> Result<(), String>
#[tauri::command] async fn toggle_trigger(id: String, enabled: bool) -> Result<(), String>
```

---

## Cómo verificar

1. Crear trigger cron: "cada 1 minuto, decí hola" → verificar que ejecuta cada 60s
2. Crear file watcher en Downloads → descargar un archivo → agente actúa automáticamente
3. Disable trigger → ya no ejecuta
4. Last run y next run se actualizan correctamente
