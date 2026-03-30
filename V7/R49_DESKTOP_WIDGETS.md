# FASE R49 — DESKTOP WIDGETS: Mini-interfaces siempre visibles

**Objetivo:** Pequeños widgets floating que el usuario puede tener siempre visibles en su escritorio: quick task input, agent status, notificaciones, mini-analytics. Sin abrir la app completa.

---

## Tareas

### 1. Widget framework (Tauri secondary windows)

```rust
// Tauri v2 soporta múltiples ventanas
// Cada widget es una ventana secondary, siempre-en-top, sin decoración

pub fn create_widget(name: &str, width: u32, height: u32, x: i32, y: i32) -> Result<WebviewWindow> {
    let window = WebviewWindowBuilder::new(app, name, url)
        .title("")
        .inner_size(width as f64, height as f64)
        .position(x as f64, y as f64)
        .decorations(false)        // Sin barra de título
        .always_on_top(true)       // Siempre visible
        .transparent(true)         // Fondo transparente
        .skip_taskbar(true)        // No aparece en taskbar
        .build()?;
    Ok(window)
}
```

### 2. Quick Task widget (mini input)

```
┌──────────────────────────────────────┐
│ ✦ [Ask AgentOS something...] [Send] │
└──────────────────────────────────────┘
```

- 400x50px, esquina inferior derecha
- Siempre visible (configurable)
- Escribir → Enter → resultado aparece como toast notification
- Hotkey para focus: Ctrl+Shift+Space

### 3. Status widget (mini dashboard)

```
┌──────────────────────────┐
│ ✦ AgentOS        ● Idle  │
│ Tasks: 42  Cost: $0.34   │
│ ⏰ Next: disk check 9am  │
└──────────────────────────┘
```

- 250x80px, siempre visible
- Muestra: estado, stats del día, próximo trigger
- Click → abre la app completa
- Draggable a cualquier posición

### 4. Notification widget (mini feed)

```
┌──────────────────────────────┐
│ ✅ Disk check done — 64%     │ ← auto-fade después de 5s
│ 💡 Automate your daily task? │ ← persist hasta dismiss
└──────────────────────────────┘
```

- Stack vertical en esquina superior derecha
- Notifications aparecen y desaparecen
- Click → abre la app en el contexto relevante

### 5. Widget manager en Settings

```
WIDGETS
  [x] Quick Task input          Position: [Bottom Right ▾]
  [x] Status widget             Position: [Bottom Left ▾]
  [x] Notification popups       Position: [Top Right ▾]
  
  Opacity: [80% ▾]
  Show on all virtual desktops: [ON]
```

### 6. IPC para widgets

```rust
// Los widgets usan los mismos IPC commands que la app principal
// Pero con vistas simplificadas (HTML/React mínimo)

// Widget Quick Task: usa process_message
// Widget Status: usa get_status + get_usage_summary
// Widget Notifications: escucha eventos chain_update, task_completed, etc.
```

---

## Demo

1. Quick Task widget visible en el escritorio → escribir "qué hora es" → toast con respuesta
2. Status widget muestra stats en tiempo real mientras trabajás en otra app
3. Notification popup aparece cuando tarea completa → fade out después de 5s
4. Drag widgets a cualquier posición → se recuerda al reiniciar
5. Settings → desactivar widgets → desaparecen
