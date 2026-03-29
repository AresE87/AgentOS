# FASE R6 — BOARD DE AGENTES: Tablero Kanban en tiempo real

**Objetivo:** Cuando el agente trabaja en una tarea compleja (cadena de sub-tareas), el usuario ve un tablero Kanban donde las cards se mueven en tiempo real entre columnas, y los agentes reportan su progreso.

**Prerequisito:** R3 (frontend conectado)

---

## Concepto

El Board responde a la pregunta "¿qué está haciendo el agente AHORA?". Hoy, el usuario envía un mensaje y espera. Con el Board, ve:

```
QUEUED          IN PROGRESS        DONE
─────────       ──────────────     ─────────
📝 Report       📊 Spreadsheet     🔍 Research
Senior           Specialist         Senior
Waiting: #1      Data Analyst       Sales Rschr
                 gpt-4o             sonnet
                 ████░░ 60%        ✅ 8.2s
```

---

## Tareas

### 1. Backend: Eventos de progreso de la cadena

En `pipeline/engine.rs` y/o `agents/hierarchy.rs`, emitir eventos cuando:
- El Orchestrator descompone una tarea en sub-tareas
- Una sub-tarea cambia de estado (queued → running → done/failed)
- Un agente reporta progreso ("Processing 127 rows...")

```rust
// Emitir evento via Tauri:
app_handle.emit("chain_update", ChainEvent {
    chain_id: "chain_001".into(),
    subtask_id: "subtask_2".into(),
    status: "running".into(),
    agent_level: "specialist".into(),
    agent_name: "Data Analyst".into(),
    model: "gpt-4o".into(),
    progress: 0.6,
    message: "Processing 127 rows from research data".into(),
});
```

### 2. Backend: IPC commands para el Board

```rust
#[tauri::command]
async fn get_active_chain() -> Result<Option<ChainState>, String>
// Retorna la cadena activa con todas sus sub-tareas y estados

#[tauri::command]
async fn get_chain_history(limit: usize) -> Result<Vec<ChainSummary>, String>
// Últimas N cadenas completadas

#[tauri::command]
async fn get_chain_detail(chain_id: String) -> Result<ChainDetail, String>
// Detalle completo: sub-tareas, log de eventos, outputs
```

### 3. Backend: Tabla chain_log en SQLite

```sql
CREATE TABLE IF NOT EXISTS chain_log (
    id          TEXT PRIMARY KEY,
    chain_id    TEXT NOT NULL,
    timestamp   TEXT NOT NULL,
    agent_name  TEXT NOT NULL,
    agent_level TEXT NOT NULL,
    event_type  TEXT NOT NULL,  -- info, progress, decision, error, complete
    message     TEXT NOT NULL,
    metadata    TEXT,           -- JSON optional
    FOREIGN KEY (chain_id) REFERENCES tasks(id)
);
```

### 4. Frontend: Página Board

Nueva página accesible desde sidebar posición 4.

**Vista Kanban (4 columnas):** QUEUED | IN PROGRESS | REVIEW | DONE

Cada card muestra:
- Nombre de la sub-tarea
- Nivel del agente (badge coloreado)
- Nombre del especialista
- Modelo + nodo (si mesh)
- Progress bar (si in progress)
- Último mensaje del agente
- Tiempo transcurrido

**Escuchar eventos en tiempo real:**
```typescript
import { listen } from '@tauri-apps/api/event';

useEffect(() => {
    const unlisten = listen('chain_update', (event) => {
        // Actualizar el estado de la card correspondiente
        updateSubtask(event.payload);
    });
    return () => { unlisten.then(fn => fn()); };
}, []);
```

### 5. Frontend: Agent Log (panel inferior)

Timeline cronológica de todos los eventos:
```
10:42:01  Orchestrator     Decomposed into 3 sub-tasks
10:42:02  Sales Researcher Started research
10:42:10  Sales Researcher ✅ Completed — 2,400 tokens
```

Colores por nivel de agente:
- Orchestrator: cyan
- Junior: green  
- Specialist: purple
- Senior: blue
- Manager: amber

### 6. Frontend: Vista History

Si no hay cadena activa, mostrar historial de cadenas pasadas con resumen.

### 7. Integración con Chat

Cuando una respuesta en Chat es de una cadena (tiene sub-tareas), agregar botón "View in Board →" que navega al Board con esa cadena seleccionada.

---

## Cómo verificar

1. Enviar tarea compleja: "Research Rust vs Go, create a comparison table, and write a summary"
2. Ir al Board → ver 3 cards moverse entre columnas en tiempo real
3. Cada card muestra agente, modelo, progreso
4. Agent Log muestra eventos cronológicos
5. Cuando termina, aparece en History
6. En Chat, la respuesta tiene "View in Board →"

---

## Nota importante

Si el sistema de cadenas (hierarchy) no está funcionando en el backend Rust, esta fase requiere primero hacer que el Orchestrator descomponga tareas de verdad. Si hoy TODA tarea va directo al engine sin descomposición, hay que:

1. En `pipeline/engine.rs`: si complexity >= 3, llamar al LLM para descomponer
2. Ejecutar cada sub-tarea secuencialmente (paralelo viene después)
3. Compilar los resultados en una respuesta unificada

Esto puede ser un pre-requisito significativo.
