# FASE R46 — OBSERVABILITY: Logging, tracing, alertas, health

**Objetivo:** Saber exactamente qué está pasando dentro de AgentOS en todo momento. Logs estructurados, tracing distribuido para cadenas mesh, alertas cuando algo falla, y un health dashboard que un IT admin puede monitorear.

---

## Tareas

### 1. Structured logging (JSON)

```rust
// Reemplazar println!/log! con structured logging:
// Crate: tracing + tracing-subscriber

use tracing::{info, warn, error, instrument};

#[instrument(skip(state))]
async fn process_message(text: &str, state: &AppState) -> Result<TaskResult> {
    info!(task_text = text, "Processing new task");
    
    let classification = classify(text);
    info!(task_type = %classification.task_type, complexity = classification.complexity, "Task classified");
    
    let result = engine.execute(text).await;
    match &result {
        Ok(r) => info!(model = %r.model, cost = r.cost, latency_ms = r.latency_ms, "Task completed"),
        Err(e) => error!(error = %e, "Task failed"),
    }
    result
}

// Output: JSON lines en archivo rotativo (max 10MB, keep 5 files)
// Location: AppData/AgentOS/logs/agentos.log
```

### 2. Tracing distribuido (mesh)

```rust
// Cada tarea tiene un trace_id que se propaga a través de la cadena y el mesh
// Cuando una sub-tarea se envía a otro nodo, el trace_id viaja con ella
// Esto permite reconstruir el timeline completo de una cadena distribuida

pub struct TraceContext {
    pub trace_id: String,      // UUID, compartido por toda la cadena
    pub span_id: String,       // UUID, único por sub-tarea
    pub parent_span_id: Option<String>,  // La sub-tarea padre
}

// En el Agent Log del Board, cada evento tiene trace_id
// Clickear trace_id → ver timeline completa aunque cruce nodos
```

### 3. Alertas

```rust
// Alertas automáticas cuando:
// - Tasa de error > 20% en la última hora
// - LLM provider no responde (3 fallos seguidos)
// - Disco < 10% espacio libre
// - Mesh: nodo no responde en 5 minutos
// - API: rate limit exceeded
// - Vault: intento de acceso fallido

pub struct AlertManager {
    rules: Vec<AlertRule>,
    notifiers: Vec<Box<dyn AlertNotifier>>,  // toast, telegram, email
}

pub struct AlertRule {
    name: String,
    condition: String,      // "error_rate_1h > 0.2"
    severity: String,       // info, warning, critical
    cooldown_minutes: u32,  // No repetir por N minutos
}
```

### 4. Health dashboard (para IT admins)

```
SYSTEM HEALTH                              [Export Logs]
──────────────────────────────────────────
ENGINE      ● HEALTHY     uptime: 14h 23m
LLM         ● HEALTHY     Anthropic: OK · OpenAI: OK · Local: N/A
DATABASE    ● HEALTHY     size: 12MB · queries/min: 23
MESH        ● WARNING     1/2 nodes offline
TELEGRAM    ● HEALTHY     polling active · 47 messages today
DISK        ● HEALTHY     64% used

RECENT ALERTS
⚠ 2h ago  Mesh: Home-PC disconnected (reconnected after 5min)
ℹ 6h ago  LLM: Switched from Anthropic to OpenAI (rate limited)

LOGS (live tail)
[streaming log output, filterable by level/module]
```

### 5. IPC commands

```rust
#[tauri::command] async fn get_system_health() -> Result<HealthReport, String>
#[tauri::command] async fn get_recent_alerts() -> Result<Vec<Alert>, String>
#[tauri::command] async fn get_logs(level: String, module: String, limit: usize) -> Result<Vec<LogEntry>, String>
#[tauri::command] async fn export_logs(start: String, end: String) -> Result<String, String>
```

---

## Demo

1. Health dashboard: todos los sistemas ● HEALTHY
2. Desconectar internet → LLM muestra ● WARNING → alerta aparece
3. Logs en live tail: ver eventos en tiempo real, filtrar por módulo
4. Export logs → JSON con todos los eventos del período
5. Trace distribuido: cadena en mesh → timeline muestra todo el recorrido
