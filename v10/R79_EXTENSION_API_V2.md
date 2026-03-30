# FASE R79 — EXTENSION API V2: Plugins crean páginas completas

**Objetivo:** Los plugins (R34) ahora pueden crear páginas enteras en el dashboard, no solo acciones. Un plugin "CRM" agrega una pestaña "CRM" en el sidebar con su propia UI. Un plugin "Kanban" agrega un board personalizado.

---

## Tareas

### 1. Plugin UI API

```rust
// Extension manifest con UI:
pub struct PluginManifest {
    // ...campos existentes de R34...
    pub ui: Option<PluginUI>,
}

pub struct PluginUI {
    pub pages: Vec<PluginPage>,
    pub widgets: Vec<PluginWidget>,
}

pub struct PluginPage {
    pub id: String,
    pub title: String,
    pub icon: String,           // Lucide icon name
    pub sidebar_position: usize, // Después de qué item
    pub html_file: String,      // "ui/page.html" dentro del plugin
}

pub struct PluginWidget {
    pub id: String,
    pub title: String,
    pub target: String,         // "home", "settings", "analytics"
    pub position: String,       // "top", "bottom", "sidebar"
    pub html_file: String,
}
```

### 2. Plugin UI rendering

```typescript
// El dashboard carga plugin pages como iframes sandboxed:
// <iframe src="plugin://crm-plugin/ui/page.html" sandbox="allow-scripts" />

// El plugin comunica con AgentOS via postMessage:
// window.parent.postMessage({type: "invoke", command: "get_tasks", args: {}}, "*")
// window.parent.postMessage({type: "navigate", page: "chat"}, "*")
// window.parent.postMessage({type: "notify", title: "CRM", message: "New lead!"}, "*")
```

### 3. Plugin SDK para UI (JavaScript)

```javascript
// agentos-plugin-ui.js — incluido en cada plugin page

const AgentOS = {
    // Llamar IPC commands del agente
    async invoke(command, args) { /* postMessage wrapper */ },
    
    // Navegar a otra página
    navigate(page) { /* postMessage */ },
    
    // Mostrar notificación
    notify(title, message) { /* postMessage */ },
    
    // Acceder al theme actual
    getTheme() { /* retorna Design System tokens */ },
    
    // Storage del plugin (key-value persistido)
    async storage: {
        get(key) { },
        set(key, value) { },
        delete(key) { },
    }
};
```

### 4. Crear 3 plugin de ejemplo con UI

```
1. CRM Plugin — Página "CRM" en sidebar con lista de contactos + pipeline
2. Notes Plugin — Página "Notes" con editor markdown + folder structure
3. Pomodoro Plugin — Widget en Home con timer pomodoro + stats
```

### 5. Plugin UI en el sidebar

```
Sidebar con plugins:
  Home
  Playbooks
  Chat
  Board
  Terminal        ← R78
  📇 CRM          ← plugin page
  📝 Notes         ← plugin page
  Mesh
  Analytics
  Developer
  Settings
```

### 6. IPC commands

```rust
#[tauri::command] async fn plugin_get_ui_config() -> Result<Vec<PluginUIConfig>, String>
#[tauri::command] async fn plugin_invoke(plugin_id: String, command: String, args: Value) -> Result<Value, String>
#[tauri::command] async fn plugin_storage_get(plugin_id: String, key: String) -> Result<Option<String>, String>
#[tauri::command] async fn plugin_storage_set(plugin_id: String, key: String, value: String) -> Result<(), String>
```

---

## Demo

1. Instalar CRM plugin → "CRM" aparece en sidebar → página con contactos y pipeline
2. Instalar Notes plugin → "Notes" aparece → editor markdown funcional
3. Instalar Pomodoro → widget en Home → timer funciona → stats se persisten
4. Desinstalar plugin → página desaparece del sidebar inmediatamente
5. Plugin usa AgentOS.invoke("process_message", "analyze this contact") → AI responde dentro del plugin
