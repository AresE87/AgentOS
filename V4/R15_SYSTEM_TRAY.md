# FASE R15 — SYSTEM TRAY + LIFECYCLE: La app vive en tu PC

**Objetivo:** AgentOS tiene ícono en el system tray, sigue corriendo cuando cerrás la ventana, tiene menú contextual, muestra notificaciones del sistema, y puede auto-iniciarse con Windows.

---

## Tareas

### 1. System tray con ícono y estados

```rust
// En lib.rs o un módulo tray.rs:
// Usar tauri::tray::TrayIconBuilder

// Estados del ícono:
// - Idle: ícono gris/default
// - Working: ícono cyan (o con indicador visual)
// - Error: ícono rojo

// El estado cambia cada vez que una tarea empieza/termina/falla
```

### 2. Menú contextual del tray

```
Click derecho en tray:
├── Open Dashboard     → abre/focaliza la ventana
├── ─────
├── Quick Task...      → mini-input para enviar tarea rápida
├── ─────
├── Recent Tasks       → sub-menú con últimas 3 tareas
├── ─────
├── Pause Agent        → pausa el procesamiento
├── Settings           → abre Settings
├── ─────
├── Quit AgentOS       → cierra todo (ventana + tray + backend)
```

### 3. Cerrar ventana ≠ cerrar app

```rust
// En Tauri config o evento handler:
// Cuando el usuario cierra la ventana (X):
// - NO cerrar la app
// - Solo ocultar la ventana
// - Mostrar toast: "AgentOS sigue corriendo en la bandeja del sistema"
// - Click en tray → reabre la ventana

// "Quit" en el menú del tray → cierra TODO
```

### 4. Notificaciones del sistema

```rust
// Cuando una tarea completa:
app_handle.notification()
    .title("Task completed")
    .body("Disk check done — 64% used")
    .icon("icons/icon.png")
    .show()?;

// Cuando una tarea falla:
app_handle.notification()
    .title("Task failed")
    .body("Could not install VLC: access denied")
    .show()?;
```

### 5. Auto-start con Windows (opcional)

```rust
// Plugin: tauri-plugin-autostart
// En Settings: toggle "Start with Windows"
// Agrega/remueve entrada en HKCU\Software\Microsoft\Windows\CurrentVersion\Run
```

---

## Cómo verificar

1. App abierta → cerrar ventana (X) → el ícono sigue en el tray
2. Click en el tray → ventana reaparece
3. Click derecho → menú con todas las opciones
4. Enviar tarea por Telegram con ventana cerrada → notificación del sistema aparece
5. "Quit" en el menú → app se cierra completamente
6. (Si auto-start) Reiniciar Windows → AgentOS aparece en tray automáticamente
