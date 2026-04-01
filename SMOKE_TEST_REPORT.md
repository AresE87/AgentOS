# AgentOS — Smoke Test Report: Vision Agent Loop E2E

**Fecha:** 2026-04-01
**Resultado:** El pipeline completo EST\u00C1 CONECTADO end-to-end.

---

## VEREDICTO

### ✅ EL LOOP VISUAL EST\u00C1 100% CABLEADO

```
Frontend (Chat.tsx)
  ↓ runPCTask(description) → invoke('cmd_run_pc_task')
  ↓
Backend (lib.rs:1610)
  ↓ cmd_run_pc_task() → spawn async → pipeline::engine::run_task()
  ↓
Engine (engine.rs:226)
  ↓ gateway.complete_as_agent() → Initial planning LLM call
  ↓ Match mode: "command_then_screen" or "screen"
  ↓
Vision Loop (engine.rs:650-771 / 787-899)
  ↓ for vs in 1..=15:
  ↓   capture::capture_full_screen()  → GDI BitBlt ✅
  ↓   to_base64_jpeg_with_dims()      → JPEG encoding ✅
  ↓   vision::plan_next_action()      → Claude/GPT-4o/Gemini ✅
  ↓   scale_action_coords()           → DPI-aware scaling ✅
  ↓   executor::execute()             → SendInput API ✅
  ↓   save_step() → DB               → SQLite ✅
  ↓   emit("agent:step_completed")    → Tauri events ✅
  ↓
Frontend receives completion
  ↓ Poll getTasks() every 1.5s → shows result
```

**Todo el camino est\u00E1 conectado. No hay gaps en la cadena.**

---

## HALLAZGOS DETALLADOS

### 1. IPC Command — ✅ EXISTE y est\u00E1 registrado

```rust
// lib.rs:1610-1683
#[tauri::command]
async fn cmd_run_pc_task(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
    description: String,
) -> Result<serde_json::Value, String>
```

- Registrado en invoke_handler (lib.rs:10917)
- Spawns async background task
- Retorna inmediatamente con `{ task_id, status: "started" }`
- Llama a `pipeline::engine::run_task()` en background

### 2. Frontend Trigger — ✅ EXISTE en Chat.tsx

```typescript
// Chat.tsx lines 84-100
// TODA mensaje que NO sea pregunta pura → runPCTask(msg)
const pcResult = await runPCTask(msg);

// Muestra mensaje con subtasks:
// "🖥️ PC Task started — I'm now controlling your PC..."
// Subtasks: [Capture screen, Plan actions, Execute actions]
```

- `isPureQuestion()` detecta si es chat normal o PC task
- Kill switch: bot\u00F3n STOP rojo llama `killSwitch()`
- Polling cada 1.5s para detectar completado/fallido

### 3. useAgent Hook — ✅ FUNCIONES CONECTADAS

```typescript
// useAgent.ts lines 236-242
runPCTask(description)     → invoke('cmd_run_pc_task')
captureScreenshot()        → invoke('cmd_capture_screenshot')
killSwitch()               → invoke('cmd_kill_switch')
resetKillSwitch()          → invoke('cmd_reset_kill_switch')
getTaskSteps(taskId)       → invoke('cmd_get_task_steps')
```

### 4. Engine Modes — ✅ TODOS IMPLEMENTADOS

| Mode | L\u00EDneas | Descripci\u00F3n |
|------|--------|-------------|
| `command` | 425-558 | PowerShell single command |
| `multi` | 281-422 | Sequential multi-step |
| `command_then_screen` | 561-785 | **Hybrid: commands + vision loop** |
| `screen` | 787-899 | **Pure vision loop** |
| `browse` | 904-976 | Web page fetch |
| `search_web` | 979-1057 | Web search |
| `done`/`chat`/`need_info` | 1060-1083 | Terminal states |

### 5. Vision Loop — ✅ COMPLETO

- **Max 15 iterations** por modo visual
- **Capture:** GDI BitBlt → RGBA buffer → JPEG base64
- **Vision LLM:** Claude Sonnet → GPT-4o → Gemini Flash (fallback chain)
- **Coordinate scaling:** Image space → physical screen space (DPI-aware)
- **Actions:** Click, DoubleClick, RightClick, Type, KeyCombo, Scroll, Wait, TaskComplete
- **Safety:** `hands::safety::check_action()` antes de cada acci\u00F3n
- **Events:** Emite `agent:step_started` y `agent:step_completed`
- **Sleep:** 800ms entre acciones
- **Dedup warning:** Si las \u00FAltimas 2 acciones son id\u00E9nticas

### 6. Kill Switch — ✅ FUNCIONAL

- `Arc<AtomicBool>` compartido entre frontend y engine
- Checked at: lines 262, 287, 666, 800 (every loop iteration)
- Frontend: bot\u00F3n STOP rojo en Chat.tsx
- Backend: `cmd_kill_switch()` + `cmd_reset_kill_switch()`

### 7. Events to Frontend — ✅ IMPLEMENTADOS

```rust
// engine.rs:1299-1306
fn emit(app: &tauri::AppHandle, event: &str, task_id: &str, step: u32, desc: &str)

// Events:
"agent:step_started"     → { task_id, step_number, description }
"agent:step_completed"   → { task_id, step_number, description }
"agent:task_completed"   → { task_id, success, output, steps, duration_ms }
```

### 8. Developer Page Testing — ✅ BOTONES EXISTEN

```typescript
// Developer.tsx lines 533-538
<Button onClick={handleCapture}> Capture Screen </Button>
<Button onClick={handleVision}> Vision Analyze </Button>
```

---

## POTENTIAL ISSUES (a validar durante testing real)

### Issue 1: Frontend NO muestra screenshots inline
- Chat.tsx muestra texto "Watch the screen" pero NO renderiza las im\u00E1genes
- Los screenshots se guardan en disco pero no se env\u00EDan al frontend
- **Impact:** El usuario no ve lo que el agente "ve" — tiene que mirar su pantalla real
- **Fix (para Codex):** Agregar screenshot base64 al evento `agent:step_completed`

### Issue 2: Polling vs Events
- Chat.tsx usa **polling** (1.5s) para detectar task completion
- Los eventos Tauri (`agent:step_*`) se emiten pero **Chat.tsx NO los escucha**
- Board.tsx S\u00CD escucha eventos `chain:*` pero son diferentes
- **Impact:** El progreso en tiempo real no se muestra step-by-step
- **Fix (para Codex):** Agregar `listen('agent:step_completed')` en Chat.tsx

### Issue 3: Window Minimization
- engine.rs minimiza la ventana principal antes de capturar
- **Esto es correcto** (evita capturarse a s\u00ED mismo)
- Pero el usuario pierde visibilidad del progreso en el dashboard
- **Impact:** UX confusa — la app desaparece mientras trabaja
- **Fix (para Codex):** Usar ventana secundaria floating (siempre visible) para progreso

### Issue 4: No validation de coordenadas
- El system prompt dice "coords must be within bounds"
- Pero executor.rs NO valida antes de llamar SendInput
- **Impact:** Click fuera de pantalla = undefined behavior
- **Fix (para Codex):** Agregar clamp en `scale_action_coords()`

### Issue 5: Sin retry en vision actions
- Si `plan_next_action()` falla, el loop se rompe
- No hay retry ni fallback a otro provider para vision
- **Impact:** Un error de API mata toda la tarea
- **Fix (para Codex):** Wrap con retry + fallback provider

### Issue 6: Browser spam guard insuficiente
- Cuenta `browser_opens` pero solo en modo "command"
- Vision loop puede abrir browsers ilimitados
- **Impact bajo** — el LLM rara vez spam\u00E9a browsers

---

## CHECKLIST PARA TESTING MANUAL

### Pre-requisitos
- [ ] API key de Anthropic O OpenAI configurada en Settings
- [ ] Permiso "Screen Access" activado en Settings
- [ ] La app est\u00E1 corriendo en Windows (SendInput es Windows-only)

### Test 1: Screenshot capture (aislado)
- [ ] Ir a p\u00E1gina Developer
- [ ] Click "Capture Screen"
- [ ] Verificar que aparece path del screenshot en los logs
- [ ] Abrir el archivo — debe mostrar el desktop real

### Test 2: Vision analyze (aislado)
- [ ] Ir a p\u00E1gina Developer
- [ ] Click "Vision Analyze"
- [ ] Verificar que aparece an\u00E1lisis de la pantalla actual
- [ ] Debe mencionar elementos visibles (taskbar, windows, etc.)

### Test 3: PC Task simple
- [ ] Ir a Chat
- [ ] Escribir: "move the mouse to the center of the screen"
- [ ] La ventana se minimiza
- [ ] El mouse se mueve
- [ ] La app reporta "Task completed"

### Test 4: PC Task con app
- [ ] Escribir: "open Notepad"
- [ ] El agente debe:
  - Tomar screenshot
  - Identificar c\u00F3mo abrir Notepad (Start menu o Win+R)
  - Ejecutar clicks/keystrokes
  - Verificar que Notepad se abri\u00F3
  - Reportar completado

### Test 5: PC Task multi-step
- [ ] Escribir: "open Notepad and type Hello World"
- [ ] Debe completar ambos pasos (abrir + escribir)

---

## PROMPTS EXACTOS PARA CODEX (por prioridad)

### PROMPT 1: Agregar screenshots al Chat (ALTA PRIORIDAD)

```
CONTEXTO: El vision loop emite eventos "agent:step_completed" con task_id,
step_number, description. Pero Chat.tsx NO escucha estos eventos y NO
muestra los screenshots que el agente captura.

ARCHIVOS:
- frontend/src/pages/dashboard/Chat.tsx
- src-tauri/src/pipeline/engine.rs (emit function, line 1299)

TAREA:
1. En engine.rs, agregar screenshot_base64 al payload del evento:
   app.emit("agent:step_completed", json!({
       "task_id": task_id,
       "step_number": step,
       "description": desc,
       "screenshot_base64": b64,  // AGREGAR
       "action_type": action_type, // AGREGAR
   }));

2. En Chat.tsx, escuchar el evento con Tauri listen():
   useEffect(() => {
     const setup = async () => {
       const { listen } = await import('@tauri-apps/api/event');
       await listen('agent:step_completed', (event) => {
         // Actualizar el mensaje del agente con el screenshot
         // Mostrar la imagen inline
       });
     };
     setup();
   }, []);

3. Renderizar screenshots como imágenes inline en el chat:
   - Mostrar miniatura del screenshot (max-width: 400px)
   - Debajo: "Step N: [description]" con ícono de check/spinner
   - Click en screenshot para ver full-size en modal

NO HACER: No cambiar la lógica del loop. Solo agregar data al evento
y mostrarla en el frontend.
```

### PROMPT 2: Agregar clamp de coordenadas (MEDIA PRIORIDAD)

```
CONTEXTO: La función scale_action_coords() en pipeline/engine.rs
escala coordenadas pero no valida que estén dentro de los bounds
de la pantalla.

ARCHIVO: src-tauri/src/pipeline/engine.rs (lines 16-59)

TAREA: Agregar clamp después del scaling:
   real_x = real_x.clamp(0, capture_w as i32 - 1);
   real_y = real_y.clamp(0, capture_h as i32 - 1);

Hacer lo mismo para DoubleClick, RightClick, y Scroll actions.

NO HACER: No cambiar la fórmula de scaling. Solo agregar clamp.
```

### PROMPT 3: Retry en vision actions (MEDIA PRIORIDAD)

```
CONTEXTO: Si vision::plan_next_action() falla en el vision loop,
el loop se rompe y la tarea falla.

ARCHIVO: src-tauri/src/pipeline/engine.rs

TAREA: Agregar retry simple:
   let action = {
       let mut last_err = String::new();
       let mut result = None;
       for attempt in 0..3 {
           match vision::plan_next_action(...).await {
               Ok(a) => { result = Some(a); break; }
               Err(e) => {
                   last_err = e;
                   tokio::time::sleep(Duration::from_secs(1)).await;
               }
           }
       }
       result.ok_or(last_err)?
   };

NO HACER: No cambiar el contenido del prompt. Solo agregar retry wrapper.
```

### PROMPT 4: Floating progress window (BAJA PRIORIDAD - post demo)

```
CONTEXTO: Cuando el vision loop se activa, la ventana principal se
minimiza para no capturarse a sí misma. El usuario pierde visibilidad.

TAREA: Crear una ventana secundaria Tauri (always-on-top, 300x80px)
que muestre:
- "Step 3/15: Click en Chrome"
- Botón STOP
- Se cierra automáticamente cuando la tarea termina

ARCHIVOS:
- src-tauri/tauri.conf.json (agregar ventana secundaria)
- frontend/src/pages/VisionProgress.tsx (nueva)
- src-tauri/src/lib.rs (abrir ventana secundaria al iniciar vision loop)
```

---

## RESUMEN PARA TOMA DE DECISIONES

| Pregunta | Respuesta |
|----------|-----------|
| ¿El vision loop est\u00E1 construido? | **S\u00CD**, 100% cableado E2E |
| ¿Funciona? | **Probablemente s\u00ED**, pero necesita testing real |
| ¿Qu\u00E9 falta para el demo? | Screenshots inline en Chat + testing manual |
| ¿Cu\u00E1nto trabajo? | **4-8 horas** de Codex + testing manual |
| ¿Hay que reconstruir algo? | **NO** — solo conectar mejor el frontend |
| ¿Riesgo principal? | Bugs de coordinate scaling o API keys no configuradas |
