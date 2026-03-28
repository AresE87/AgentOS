# Architecture: AOS-020 + AOS-021 — Tauri Shell e IPC Bridge

**Tickets:** AOS-020 (Tauri Shell), AOS-021 (IPC Bridge)
**Rol:** Software Architect + API Designer
**Fecha:** Marzo 2026

---

## Arquitectura de 3 capas

```
┌───────────────────────────────────────────────────┐
│                 REACT FRONTEND                     │
│           (TypeScript + Tailwind CSS)              │
│                                                   │
│  Setup Wizard │ Dashboard │ Chat │ Settings       │
│                                                   │
│  Comunica via: invoke("command_name", {payload})  │
└────────────────────┬──────────────────────────────┘
                     │ Tauri IPC (WebView ↔ Rust)
                     ▼
┌───────────────────────────────────────────────────┐
│                 RUST SHELL (Tauri)                  │
│                                                   │
│  - Ventana principal                              │
│  - System tray                                    │
│  - Tauri commands (IPC handlers)                  │
│  - Python process manager                         │
│  - Auto-update checker                            │
│                                                   │
│  Comunica via: JSON-RPC sobre stdin/stdout        │
└────────────────────┬──────────────────────────────┘
                     │ stdin/stdout (JSON-RPC 2.0)
                     ▼
┌───────────────────────────────────────────────────┐
│              PYTHON AGENT (Phase 1+2)              │
│                                                   │
│  AgentCore + Gateway + Executor + Store + ...     │
│                                                   │
│  Nuevo: ipc_server.py — JSON-RPC server on stdio  │
└───────────────────────────────────────────────────┘
```

---

## IPC Protocol: JSON-RPC 2.0 sobre stdio

Rust escribe JSON en stdin del proceso Python. Python responde en stdout.

### Request (Rust → Python)
```json
{"jsonrpc": "2.0", "method": "process_message", "params": {"text": "hello", "source": "chat"}, "id": 1}
```

### Response (Python → Rust)
```json
{"jsonrpc": "2.0", "result": {"task_id": "abc-123", "status": "completed", "output": "Hello! How can I help?"}, "id": 1}
```

### Event (Python → Rust, sin id = notificación)
```json
{"jsonrpc": "2.0", "method": "event", "params": {"type": "task_started", "task_id": "abc-123"}}
```

---

## Métodos JSON-RPC

| Method | Params | Response | Descripción |
|--------|--------|----------|-------------|
| `get_status` | `{}` | `{state, providers, active_playbook, session_stats}` | Estado general del agente |
| `process_message` | `{text, source}` | `{task_id, status, output, model, cost, duration}` | Procesar un mensaje |
| `get_tasks` | `{limit?, status?}` | `{tasks: [...]}` | Listar tareas recientes |
| `get_playbooks` | `{}` | `{playbooks: [...]}` | Listar playbooks instalados |
| `set_active_playbook` | `{path}` | `{ok: true}` | Cambiar playbook activo |
| `get_settings` | `{}` | `{settings}` | Obtener configuración actual |
| `update_settings` | `{key, value}` | `{ok: true}` | Actualizar una setting |
| `health_check` | `{}` | `{providers: {name: healthy}}` | Verificar proveedores |
| `start_recording` | `{playbook_path}` | `{ok: true}` | Iniciar Step Recorder |
| `stop_recording` | `{}` | `{steps_count}` | Detener Step Recorder |
| `get_usage_summary` | `{period}` | `{summary}` | Resumen de costos |

### Eventos (Python → Rust → React)

| Event type | Params | Cuándo |
|-----------|--------|--------|
| `task_started` | `{task_id}` | Agente empezó a procesar |
| `task_completed` | `{task_id, output, cost}` | Tarea terminada |
| `task_failed` | `{task_id, error}` | Tarea falló |
| `typing` | `{task_id}` | Agente está procesando (para UI indicator) |
| `agent_error` | `{message}` | Error general del agente |

---

## Python: IPC Server

```python
# agentos/ipc_server.py

class IPCServer:
    """JSON-RPC server que escucha en stdin y responde en stdout.

    Lanzado por Tauri como child process.
    Cada línea en stdin es un request JSON-RPC.
    Cada línea en stdout es una response o event JSON-RPC.
    stderr se usa para logging (no para IPC).
    """

    def __init__(self, agent_core: AgentCore) -> None:
        ...

    async def run(self) -> None:
        """Loop principal: lee stdin → procesa → escribe stdout."""
        ...

    async def handle_request(self, request: dict) -> dict:
        """Dispatch un request al método correcto."""
        ...

    def send_event(self, event_type: str, params: dict) -> None:
        """Envía una notificación (sin id) a Rust via stdout."""
        ...
```

---

## Rust: Tauri Commands

```rust
// src-tauri/src/main.rs

#[tauri::command]
async fn get_status(state: State<AgentProcess>) -> Result<Value, String> {
    state.send_request("get_status", json!({})).await
}

#[tauri::command]
async fn process_message(text: String, state: State<AgentProcess>) -> Result<Value, String> {
    state.send_request("process_message", json!({"text": text, "source": "chat"})).await
}

// ... un command por cada método JSON-RPC
```

---

## React: TypeScript hooks

```typescript
// frontend/src/hooks/useAgent.ts

export function useAgent() {
    const getStatus = () => invoke<AgentStatus>("get_status");
    const processMessage = (text: string) => invoke<TaskResult>("process_message", { text });
    const getTasks = (limit?: number) => invoke<TaskList>("get_tasks", { limit });
    // ...
    return { getStatus, processMessage, getTasks, ... };
}
```

---

## Proceso de arranque

```
1. Usuario abre AgentOS (click en icono)
2. Tauri inicia
3. Tauri lanza Python como child process: `python -m agentos.ipc_server`
4. Tauri espera "ready" event de Python (max 10s)
5. Si primera ejecución → mostrar Setup Wizard
6. Si no → mostrar Dashboard
7. System tray se activa
8. Agente empieza a escuchar mensajes (Telegram + Chat)
```

---

## ADR: JSON-RPC sobre stdio (no HTTP local)

- **Status:** Accepted
- **Context:** Necesitamos comunicación Rust ↔ Python. Opciones: HTTP localhost, gRPC, Unix socket, stdio.
- **Decision:** JSON-RPC 2.0 sobre stdin/stdout. Una línea = un mensaje.
- **Consequences:** Simple, no requiere puerto (evita conflictos), no requiere dependencias extra. Más difícil de debuggear que HTTP, pero `stderr` se usa para logs.

## ADR: Python como sidecar process (no embebido)

- **Status:** Accepted
- **Context:** Podríamos compilar Python a .so e integrarlo en Rust via PyO3, pero es complejo.
- **Decision:** Python corre como proceso independiente lanzado por Tauri. Se comunican por stdio.
- **Consequences:** Más simple. Python puede crashear sin tirar Tauri. Tauri puede reiniciar Python. Desventaja: overhead de serialización JSON.
