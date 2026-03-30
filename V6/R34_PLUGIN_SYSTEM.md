# FASE R34 — PLUGIN SYSTEM: Terceros extienden AgentOS

**Objetivo:** Developers pueden crear plugins que agregan nuevas capacidades al agente sin tocar el core. Un plugin puede agregar: nuevos ejecutores (ej: Docker executor), nuevos canales (ej: Slack), nuevos providers (ej: Mistral API), o nuevas acciones de UI.

---

## Arquitectura

```
plugins/
├── docker-executor/
│   ├── plugin.json       ← metadata: name, version, type, entry_point
│   ├── executor.wasm     ← lógica (WebAssembly para sandboxing)
│   └── README.md
├── slack-channel/
│   ├── plugin.json
│   ├── channel.wasm
│   └── README.md
└── mistral-provider/
    ├── plugin.json
    ├── provider.wasm
    └── README.md
```

### Plugin types

| Type | Interface | Ejemplo |
|------|-----------|---------|
| `executor` | Recibe task → retorna output | Docker executor, SSH executor |
| `channel` | Recibe mensaje → envía al engine → retorna respuesta | Slack, Teams, Matrix |
| `provider` | Recibe prompt → retorna completion | Mistral, Cohere, local model |
| `action` | Nueva acción que el LLM puede invocar | Send email, create calendar event |
| `widget` | UI component en el dashboard | Custom chart, integration panel |

### Plugin manifest (plugin.json)

```json
{
  "name": "docker-executor",
  "version": "1.0.0",
  "type": "executor",
  "description": "Execute tasks inside Docker containers",
  "author": "AgentOS Community",
  "permissions": ["network", "docker_socket"],
  "entry_point": "executor.wasm",
  "config_schema": {
    "docker_host": {"type": "string", "default": "unix:///var/run/docker.sock"}
  }
}
```

### Sandboxing con WebAssembly

```rust
// Los plugins corren en un sandbox WASM (wasmtime)
// No tienen acceso directo al filesystem ni a la red
// Solo pueden usar las APIs que AgentOS les expone:
//   - log(message)
//   - http_get(url) / http_post(url, body) — con allowlist de dominios
//   - read_file(path) — solo dentro del directorio del plugin
//   - emit_event(type, data) — para comunicarse con la UI

// Crate: wasmtime
```

### Plugin registry (en el marketplace)

Plugins se distribuyen como .aosp (mismo formato que playbooks, con tipo "plugin" en metadata).

---

## Tareas

### 1. Plugin loader

```rust
pub struct PluginManager {
    plugins: HashMap<String, LoadedPlugin>,
}

impl PluginManager {
    pub fn discover(plugins_dir: &Path) -> Result<Vec<PluginManifest>>;
    pub fn load(&mut self, name: &str) -> Result<()>;
    pub fn unload(&mut self, name: &str) -> Result<()>;
    pub fn call(&self, name: &str, method: &str, input: &[u8]) -> Result<Vec<u8>>;
}
```

### 2. Plugin SDK (para creadores)

```rust
// SDK minimalista que los creadores usan:
// agentos_plugin_sdk (Rust crate que compila a WASM)

#[agentos_plugin]
pub fn execute(task: &str, config: &Config) -> Result<String> {
    // Lógica del plugin
}
```

### 3. Frontend: Plugin management en Settings

```
PLUGINS                                [+ Install Plugin]
┌──────────────────────────────────────────────────────┐
│ 🐳 Docker Executor       v1.0.0  executor   [ON][🗑] │
│    Execute tasks inside Docker containers             │
│                                                       │
│ 💬 Slack Channel          v1.2.0  channel    [ON][🗑] │
│    Receive and send messages via Slack                 │
└──────────────────────────────────────────────────────┘
```

### 4. Crear 3 plugins de ejemplo

1. **docker-executor** — Ejecuta comandos dentro de un container Docker
2. **clipboard-action** — Acción "copy_to_clipboard" que el LLM puede usar
3. **weather-widget** — Widget en Home que muestra el clima

---

## Demo

1. Instalar plugin docker-executor → enviar "run hello-world in Docker" → funciona
2. Instalar plugin clipboard-action → "copia este texto al portapapeles" → funciona
3. Desactivar plugin → la funcionalidad desaparece. Reactivar → vuelve.
4. Plugin malicioso (intenta leer /etc/passwd) → sandbox lo bloquea
