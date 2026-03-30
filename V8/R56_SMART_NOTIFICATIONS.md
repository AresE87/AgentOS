# FASE R56 — SMART NOTIFICATIONS: El agente te avisa antes de que preguntes

**Objetivo:** El agente monitorea patrones y condiciones en background y te notifica proactivamente: "Tu disco está al 90%", "Hay un archivo grande nuevo en Downloads", "Tu backup no corrió ayer".

---

## Tareas

### 1. Background monitor system

```rust
// Nuevo: src-tauri/src/monitors/

pub trait Monitor: Send + Sync {
    fn name(&self) -> &str;
    fn check_interval(&self) -> Duration;
    async fn check(&self) -> Option<Notification>;
}

pub struct MonitorManager {
    monitors: Vec<Box<dyn Monitor>>,
}

impl MonitorManager {
    pub async fn start(&self) {
        for monitor in &self.monitors {
            tokio::spawn(async move {
                loop {
                    if let Some(notification) = monitor.check().await {
                        emit_notification(notification);
                    }
                    tokio::time::sleep(monitor.check_interval()).await;
                }
            });
        }
    }
}
```

### 2. Built-in monitors (5)

```rust
// 1. Disk space monitor
struct DiskMonitor;  // Alerta si disco > 85%

// 2. Large file detector
struct LargeFileMonitor;  // Detecta archivos > 1GB nuevos

// 3. System health
struct SystemHealthMonitor;  // CPU > 90% sostenido, RAM > 90%

// 4. Backup reminder
struct BackupMonitor;  // Si no se hizo backup en X días

// 5. Update checker
struct UpdateMonitor;  // Si hay nueva versión de AgentOS
```

### 3. Notification center mejorado

```
🔔 NOTIFICATIONS (3 new)                    [Settings] [Clear all]
──────────────────────────────────────────────────────────────

🔴 NOW   Disk space critical: C: drive at 92%
          Free up space or expand your drive.
          [Run disk cleanup] [Dismiss]

🟡 1h    Large file detected: video_edit.mp4 (4.2GB) in Downloads
          [Organize file] [Ignore] [Dismiss]

🔵 3h    System update available: AgentOS v1.1.2
          Bug fixes and performance improvements.
          [Install now] [Later]

── Earlier ──────────────────────────────────────
✅ Yesterday  Weekly backup completed successfully
ℹ  2 days    Routing table optimized: 3 model changes
```

### 4. Notification actions

Cada notificación puede tener acciones que ejecutan tareas:
- "Run disk cleanup" → ejecuta `run_pc_task("clean up temporary files and large unused files")`
- "Organize file" → ejecuta el playbook file-organizer con el archivo detectado
- "Install now" → ejecuta auto-update

### 5. Settings: configurar monitors

```
SMART NOTIFICATIONS
  [x] Disk space alerts         Threshold: [85% ▾]
  [x] Large file detection      Minimum: [1 GB ▾]
  [x] System health             CPU/RAM: [90% ▾]
  [x] Backup reminders          Remind after: [7 days ▾]
  [x] Update notifications      
  
  Notify via:
  [x] Desktop notification
  [x] Dashboard notification center
  [ ] Telegram
  [ ] Discord
```

---

## Demo

1. Copiar un archivo de 2GB a Downloads → notificación aparece en < 1 minuto
2. Llenar el disco a 90% → alerta roja con botón "Run cleanup"
3. Click "Run cleanup" → el agente ejecuta limpieza → notificación de resultado
4. Settings: desactivar disk alerts → ya no notifica
5. Notification center: historial de todas las alertas con acciones
