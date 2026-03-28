# Data Design: AOS-006 — SQLite Task Store — Persistencia de tareas y logging

**Ticket:** AOS-006
**Rol:** Database Architect + CISO (combinados por eficiencia)
**Input:** AOS-001 Architecture, AOS-002 API Contract (LLMResponse), AOS-004 Architecture (ExecutionResult)
**Fecha:** Marzo 2026

---

## Schema

### Tabla: tasks

Registro principal de cada tarea procesada por el agente.

```sql
CREATE TABLE IF NOT EXISTS tasks (
    id              TEXT PRIMARY KEY,           -- UUID v4
    source          TEXT NOT NULL,              -- 'telegram', 'chat', 'api'
    user_id         TEXT NOT NULL,              -- ID del usuario en la plataforma
    chat_id         TEXT NOT NULL,              -- ID del chat/conversación
    input_text      TEXT NOT NULL,              -- Mensaje original del usuario
    
    -- Clasificación
    task_type       TEXT,                       -- 'text', 'code', 'vision', 'generation', 'data'
    complexity      INTEGER,                    -- 1-5
    tier            INTEGER,                    -- 1, 2, 3
    
    -- Resultado LLM
    model_used      TEXT,                       -- ID del modelo (ej: 'claude-3-haiku-20240307')
    provider        TEXT,                       -- 'anthropic', 'openai', 'google'
    tokens_in       INTEGER DEFAULT 0,
    tokens_out      INTEGER DEFAULT 0,
    cost_estimate   REAL DEFAULT 0.0,          -- USD
    
    -- Estado y resultado
    status          TEXT NOT NULL DEFAULT 'pending',  -- 'pending', 'running', 'completed', 'failed'
    output_text     TEXT,                       -- Respuesta final al usuario
    error_message   TEXT,                       -- Si falló, por qué
    
    -- Timestamps
    created_at      TEXT NOT NULL,              -- ISO 8601 UTC
    started_at      TEXT,                       -- Cuando empezó a procesarse
    completed_at    TEXT,                       -- Cuando terminó
    duration_ms     REAL                        -- Duración total en ms
);
```

### Tabla: execution_log

Log de cada acción ejecutada (CLI, API, Screen). Una tarea puede tener múltiples ejecuciones.

```sql
CREATE TABLE IF NOT EXISTS execution_log (
    id              TEXT PRIMARY KEY,           -- UUID v4
    task_id         TEXT NOT NULL,              -- FK → tasks.id
    executor_type   TEXT NOT NULL,              -- 'cli', 'api', 'screen'
    command         TEXT NOT NULL,              -- Comando o acción ejecutada
    exit_code       INTEGER,                    -- Exit code del proceso (NULL si no aplica)
    stdout          TEXT,                       -- Output truncado (max 10KB en DB)
    stderr          TEXT,                       -- Error output truncado
    duration_ms     REAL NOT NULL,              -- Duración de la ejecución
    success         INTEGER NOT NULL DEFAULT 0, -- 0=false, 1=true (SQLite no tiene bool)
    created_at      TEXT NOT NULL,              -- ISO 8601 UTC
    
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);
```

### Tabla: llm_usage

Registro detallado de cada llamada al LLM Gateway. Incluye fallbacks.

```sql
CREATE TABLE IF NOT EXISTS llm_usage (
    id              TEXT PRIMARY KEY,           -- UUID v4
    task_id         TEXT NOT NULL,              -- FK → tasks.id
    provider        TEXT NOT NULL,              -- 'anthropic', 'openai', 'google'
    model           TEXT NOT NULL,              -- ID completo del modelo
    tokens_in       INTEGER NOT NULL DEFAULT 0,
    tokens_out      INTEGER NOT NULL DEFAULT 0,
    cost_estimate   REAL NOT NULL DEFAULT 0.0,  -- USD
    latency_ms      REAL NOT NULL,              -- Latencia de la llamada
    success         INTEGER NOT NULL DEFAULT 0, -- 0=false, 1=true
    error_type      TEXT,                       -- Tipo de error si falló (nunca el mensaje completo con keys)
    fallback_index  INTEGER NOT NULL DEFAULT 0, -- 0=primer intento, 1=primer fallback, etc.
    created_at      TEXT NOT NULL,              -- ISO 8601 UTC
    
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);
```

---

## Indexes

```sql
-- Tasks: queries frecuentes
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_created_at ON tasks(created_at);
CREATE INDEX IF NOT EXISTS idx_tasks_source ON tasks(source);
CREATE INDEX IF NOT EXISTS idx_tasks_user_id ON tasks(user_id);

-- Execution log: buscar por tarea
CREATE INDEX IF NOT EXISTS idx_exec_task_id ON execution_log(task_id);
CREATE INDEX IF NOT EXISTS idx_exec_created_at ON execution_log(created_at);

-- LLM usage: reportes de costo
CREATE INDEX IF NOT EXISTS idx_llm_task_id ON llm_usage(task_id);
CREATE INDEX IF NOT EXISTS idx_llm_provider ON llm_usage(provider);
CREATE INDEX IF NOT EXISTS idx_llm_created_at ON llm_usage(created_at);
CREATE INDEX IF NOT EXISTS idx_llm_model ON llm_usage(model);
```

---

## Key queries (pre-escritas)

### 1. Tareas recientes
```sql
SELECT * FROM tasks ORDER BY created_at DESC LIMIT ?;
```

### 2. Tareas por estado
```sql
SELECT * FROM tasks WHERE status = ? ORDER BY created_at DESC LIMIT ?;
```

### 3. Costo total por período
```sql
SELECT 
    SUM(cost_estimate) as total_cost,
    SUM(tokens_in) as total_tokens_in,
    SUM(tokens_out) as total_tokens_out,
    COUNT(*) as total_calls
FROM llm_usage 
WHERE created_at >= ? AND created_at < ?;
```

### 4. Costo por proveedor
```sql
SELECT 
    provider,
    SUM(cost_estimate) as total_cost,
    COUNT(*) as call_count,
    AVG(latency_ms) as avg_latency
FROM llm_usage 
WHERE created_at >= ? AND created_at < ?
GROUP BY provider;
```

### 5. Costo por modelo
```sql
SELECT 
    model,
    SUM(cost_estimate) as total_cost,
    COUNT(*) as call_count,
    SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as success_count
FROM llm_usage 
WHERE created_at >= ? AND created_at < ?
GROUP BY model;
```

### 6. Tasa de éxito global
```sql
SELECT 
    COUNT(*) as total,
    SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as completed,
    SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as failed
FROM tasks
WHERE created_at >= ? AND created_at < ?;
```

### 7. Log de ejecución de una tarea
```sql
SELECT * FROM execution_log WHERE task_id = ? ORDER BY created_at ASC;
```

### 8. Historial de uso LLM de una tarea (incluyendo fallbacks)
```sql
SELECT * FROM llm_usage WHERE task_id = ? ORDER BY fallback_index ASC;
```

---

## Interface: TaskStore

```python
class TaskStore:
    """Async SQLite store para tareas, logs de ejecución y uso de LLM.

    Uso:
        store = TaskStore(db_path=Path("data/agentos.db"))
        await store.initialize()  # Crea tablas si no existen
        task_id = await store.create_task(task_input)
        await store.update_task_status(task_id, TaskStatus.RUNNING)
        await store.save_execution(task_id, execution_result)
        await store.save_llm_usage(task_id, llm_response, fallback_index)
        await store.complete_task(task_id, output_text)
        await store.close()
    """

    def __init__(self, db_path: Path) -> None: ...
    async def initialize(self) -> None: ...
    async def close(self) -> None: ...

    # Tasks CRUD
    async def create_task(self, task_input: TaskInput) -> str: ...
    async def update_task_status(self, task_id: str, status: TaskStatus) -> None: ...
    async def update_task_classification(self, task_id: str, classification: TaskClassification) -> None: ...
    async def complete_task(self, task_id: str, output: str, llm_response: LLMResponse | None) -> None: ...
    async def fail_task(self, task_id: str, error: str) -> None: ...
    async def get_task(self, task_id: str) -> TaskResult | None: ...
    async def get_recent_tasks(self, limit: int = 10) -> list[TaskResult]: ...

    # Execution log
    async def save_execution(self, task_id: str, result: ExecutionResult) -> None: ...

    # LLM usage
    async def save_llm_usage(self, task_id: str, response: LLMResponse, fallback_index: int = 0) -> None: ...
    async def get_usage_summary(self, start: str, end: str) -> UsageSummary: ...
```

---

## Migration

```python
SCHEMA_VERSION = 1

async def initialize(self) -> None:
    """Crea la base de datos y las tablas si no existen.

    Usa una tabla interna `_schema_version` para trackear la versión.
    Si la versión en DB < SCHEMA_VERSION, ejecuta migraciones pendientes.
    """
    ...
```

Tabla interna de versión:
```sql
CREATE TABLE IF NOT EXISTS _schema_version (
    version INTEGER NOT NULL,
    applied_at TEXT NOT NULL
);
```

---

## Storage estimates

| Tabla | Por tarea | 1000 tareas/mes | 1 año |
|-------|----------|-----------------|-------|
| tasks | ~1 KB | ~1 MB | ~12 MB |
| execution_log | ~2 KB (con output truncado) | ~2 MB | ~24 MB |
| llm_usage | ~0.5 KB × avg 1.3 calls | ~0.65 MB | ~8 MB |
| **Total** | | ~3.65 MB/mes | **~44 MB/año** |

SQLite maneja esto sin problema. No se necesita cleanup automático en v1. Policy de retención recomendada para v2+: archivar datos > 6 meses.

---

## Security Requirements (CISO)

### [MUST] Datos que NUNCA se almacenan en estas tablas

- **SEC-040**: API keys — ni completas, ni parciales, ni hasheadas
- **SEC-041**: Tokens de sesión de Telegram/WhatsApp/Discord
- **SEC-042**: Contraseñas o credentials del usuario
- **SEC-043**: El campo `error_type` en `llm_usage` solo almacena el TIPO de error (ej: "rate_limit", "auth_error"), nunca el mensaje completo (podría contener keys)

### [MUST] Protección de datos del usuario

- **SEC-044**: `input_text` y `output_text` en `tasks` contienen datos del usuario. La DB debe estar en un directorio protegido (permisos 700 en Unix).
- **SEC-045**: `stdout` y `stderr` en `execution_log` se truncan a 10 KB máximo en la DB (el output completo se muestra al usuario pero no se persiste completo).
- **SEC-046**: La DB file path se configura en Settings, no se hardcodea.

### [MUST] Integridad

- **SEC-047**: Usar WAL mode para resistencia a crashes: `PRAGMA journal_mode=WAL;`
- **SEC-048**: IDs son UUID v4 generados con `uuid.uuid4()`, no auto-increment (previene enumeración).
- **SEC-049**: Todos los timestamps en ISO 8601 UTC. Nunca local time.
