# CONSOLIDACIÓN C9 — DESKTOP WIDGETS REALES

**Estado actual:** 🔲 Widget configs en memoria. NO crea ventanas flotantes reales.
**Objetivo:** Quick Task widget como ventana Tauri secondary: floating, always-on-top, sin decoraciones. El usuario escribe una tarea sin abrir la app.

---

## Qué YA existe

```
src-tauri/src/widgets/ — struct WidgetConfig, WidgetManager
- create_widget(), destroy_widget(), list_widgets()
- TODO en memoria, no crea ventanas reales
- Frontend tiene configuración en Settings
```

## Qué REEMPLAZAR

### 1. Crear ventana Tauri real

```rust
// REEMPLAZAR el stub con:
pub fn create_quick_task_widget(app: &AppHandle) -> Result<WebviewWindow> {
    let widget = WebviewWindowBuilder::new(
        app,
        "quick-task",
        WebviewUrl::App("widget-quick-task.html".into()),
    )
    .title("")
    .inner_size(400.0, 60.0)
    .position(
        // Bottom-right corner de la pantalla
        (screen_width - 420) as f64,
        (screen_height - 80) as f64,
    )
    .decorations(false)
    .always_on_top(true)
    .transparent(true)
    .skip_taskbar(true)
    .resizable(false)
    .build()?;
    
    Ok(widget)
}
```

### 2. Frontend: widget-quick-task.html

```html
<!-- Página mínima para el widget, cargada en la secondary window -->
<!-- Usar mismo sistema de IPC que la app principal -->

<div style="
    background: rgba(13, 17, 23, 0.95);
    border: 1px solid #00E5E5;
    border-radius: 8px;
    padding: 8px 12px;
    display: flex;
    align-items: center;
    gap: 8px;
">
    <span style="color: #00E5E5; font-size: 16px;">✦</span>
    <input id="task-input" placeholder="Ask AgentOS..." style="
        flex: 1; background: transparent; border: none; color: #E6EDF3;
        font-family: Inter; font-size: 14px; outline: none;
    " />
    <button onclick="sendTask()" style="
        background: #00E5E5; color: #0A0E14; border: none; border-radius: 4px;
        padding: 4px 12px; font-weight: 600; cursor: pointer;
    ">Send</button>
</div>

<script>
async function sendTask() {
    const input = document.getElementById('task-input');
    const text = input.value.trim();
    if (!text) return;
    input.value = '';
    // Enviar tarea via IPC al backend principal
    await window.__TAURI__.core.invoke('process_message', { text });
    // Resultado aparece como toast notification del sistema (R15 system tray)
}
document.getElementById('task-input').addEventListener('keydown', (e) => {
    if (e.key === 'Enter') sendTask();
});
</script>
```

### 3. Hotkey global para mostrar/ocultar widget

```rust
// Registrar hotkey: Ctrl+Shift+Space → toggle widget
use tauri_plugin_global_shortcut::GlobalShortcutExt;

app.global_shortcut().register("CmdOrCtrl+Shift+Space", move |_app, _shortcut, event| {
    if event.state == ShortcutState::Pressed {
        if let Some(w) = _app.get_webview_window("quick-task") {
            if w.is_visible().unwrap_or(false) {
                w.hide().ok();
            } else {
                w.show().ok();
                w.set_focus().ok();
            }
        }
    }
})?;
```

### 4. Toggle en Settings

```
WIDGETS
  [x] Quick Task widget    Hotkey: Ctrl+Shift+Space
      Position: [Bottom Right ▾]
```

---

## Verificación

1. ✅ Widget aparece en bottom-right: barra mini con input + botón Send
2. ✅ Ctrl+Shift+Space → widget aparece/desaparece
3. ✅ Escribir "qué hora es" → Enter → resultado como toast notification
4. ✅ Widget es always-on-top, no tiene decoración, es semi-transparente
5. ✅ Settings → desactivar → widget desaparece
