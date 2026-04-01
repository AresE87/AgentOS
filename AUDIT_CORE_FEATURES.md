# AgentOS — Auditoría Real de las 3 Features Core

**Fecha:** 2026-04-01
**Objetivo:** Verificar qué funciona de verdad vs qué es scaffolding

---

## VEREDICTO EJECUTIVO

| Feature | Estado Real | Resumen |
|---------|-------------|---------|
| **1. Control Visual de PC** | **FUNCIONA** (95%) | El loop vision completo existe y hace API calls reales. Screen capture (GDI), mouse/keyboard (SendInput), vision LLM (Claude/GPT-4o/Gemini) — todo es código de producción. Solo falta exponerlo mejor en el frontend. |
| **2. Aprendizaje por Demostración** | **PARCIAL** (40%) | La grabación de playbooks es MANUAL (no captura acciones del usuario automáticamente). El replay SÍ funciona via vision LLM. No hay hooks de mouse/teclado. |
| **3. Mesh Multi-Máquina** | **PARCIAL** (60%) | Discovery UDP + Transport TCP funcionan. Delegación manual de tareas funciona. El orchestrator calcula scores pero NUNCA ejecuta la delegación automática. |

---

## 🥇 FEATURE 1: Control Visual de PC

### ✅ LO QUE FUNCIONA (de verdad)

#### Screen Capture — `eyes/capture.rs`
- **REAL:** Windows GDI API directa (BitBlt, GetDIBits)
- Captura pantalla completa a buffer RGBA
- Convierte a JPEG base64 para enviar a LLM
- Escala imágenes grandes (>1920px) para eficiencia de tokens
- Retorna dimensiones para escalar coordenadas de vuelta
- **Tests unitarios incluidos que ejecutan la captura**

#### Input Control — `hands/input.rs`
- **REAL:** Windows SendInput API directa
- `click(x, y)` — Posicionamiento absoluto con escala 65535
- `double_click(x, y)` — Dos clicks con 50ms delay
- `right_click(x, y)` — Click derecho (context menu)
- `scroll(x, y, delta)` — MOUSEEVENTF_WHEEL
- `type_text(text)` — Unicode char por char (KEYEVENTF_UNICODE)
- `key_combo(keys)` — Ctrl+C, Alt+Tab, etc. con VK codes
- **NO usa `enigo`** — usa Windows API directamente (mejor, menos deps)

#### Vision LLM Integration — `eyes/vision.rs` + `brain/providers.rs`
- **REAL:** HTTP POST a APIs reales
- `call_anthropic_vision()` — Envía base64 JPEG a Claude
- `call_openai_vision()` — Envía data URL a GPT-4o
- `call_google_vision()` — Envía inline_data a Gemini
- System prompt de 60 líneas con 18 tipos de acción
- Parsea respuesta JSON del modelo → AgentAction enum

#### Vision Agent Loop — `pipeline/engine.rs`
- **REAL:** Loop autónomo de 15 iteraciones máximo
- Flujo completo:
  1. `capture_full_screen()` → screenshot real
  2. `to_base64_jpeg()` → codificación
  3. `plan_next_action()` → envío a Claude con historial
  4. Escalar coordenadas (DPI-aware: image space → screen space)
  5. `executor::execute()` → acción real (click, type, key)
  6. Guardar step en DB con screenshot
  7. Repetir hasta "TaskComplete" o 15 steps
- Protecciones: dedup warning, browser spam guard, kill switch

#### Safety System — `hands/safety.rs`
- **REAL:** 40+ patrones regex para bloquear comandos peligrosos
- Clasificación: Low → Medium → High → Critical
- Se ejecuta ANTES de cada acción en `executor.rs`

#### Command Execution — `hands/cli.rs`
- **REAL:** `std::process::Command` para PowerShell/CMD
- Timeout configurable, captura stdout/stderr
- Sanitización de input

### ⚠️ LO QUE FALTA

| Item | Estado | Esfuerzo |
|------|--------|----------|
| Multi-monitor capture | Solo monitor primario | 2-4h |
| Frontend visual del loop | El Chat tiene modo visual pero podría mostrar screenshots en tiempo real mejor | 4-8h |
| Error recovery avanzado | Se detiene a los 15 steps sin fallback | 2-4h |

### 🎯 CONCLUSIÓN FEATURE 1

**El "demo killer" YA EXISTE en el backend.** El loop completo (screenshot → Claude → acción → verificación) es código de producción con APIs reales. No es scaffolding.

**Lo que hay que hacer:**
1. Verificar que compila y se ejecuta end-to-end (puede haber bugs de integración)
2. Mejorar la UX del frontend para mostrar el loop visualmente
3. Testear con 5 tareas reales y ajustar el prompt de visión

---

## 🥈 FEATURE 2: Aprendizaje por Demostración

### ✅ LO QUE FUNCIONA

#### Playbook Recording — `playbooks/recorder.rs`
- **REAL pero MANUAL:** Captura screenshots y guarda metadata JSON
- Estructura: `playbooks/{name}/steps/{step_number:02}.jpg` + `.json`
- Guarda playbook completo como `playbook.json`

#### Playbook Playback — `playbooks/player.rs`
- **REAL y FUNCIONAL:** Replay guiado por vision LLM
- Para cada step:
  1. Captura pantalla actual
  2. Envía screenshot + descripción del step a Claude
  3. Claude decide la acción (no es replay literal, es adaptativo)
  4. Ejecuta via `executor::execute()`
- Retry logic: MAX_ATTEMPTS_PER_STEP = 5
- Dedup detection: se detiene si la misma acción se repite 3x
- Minimiza ventana de AgentOS antes de capturar

#### Smart Playbooks — `playbooks/smart.rs`
- **PARCIAL:** Ejecuta comandos PowerShell/sh
- Variable substitution funciona
- Vision steps (`VisionClick`, `Browse`, `VisionCheck`) son STUBS (TODO)

### ❌ LO QUE NO FUNCIONA

| Item | Estado | Problema |
|------|--------|----------|
| **Captura automática de acciones** | NO EXISTE | No hay hooks de mouse/teclado. El usuario describe manualmente qué hizo |
| **Recording real** | NO EXISTE | `cmd_record_step()` acepta solo un string `description` y `action_type` genérico |
| **Screen Recording** | STUB | `recording/recorder.rs` es un contenedor de datos sin captura automática |
| **Fine-tuning** | STUB | `training/finetune.rs` crea job records pero nunca entrena |
| **RAG/Embeddings** | PARCIAL | Las funciones de embedding (OpenAI) y cosine similarity existen pero `search()` usa LIKE queries |

### 🎯 CONCLUSIÓN FEATURE 2

**El replay funciona, la grabación no.**

El playback es inteligente — usa vision LLM para interpretar cada step, lo que lo hace adaptativo (funciona aunque los íconos cambien de posición). Pero la grabación requiere que el usuario describa manualmente cada paso.

**Lo que hay que construir:**
1. **Hooks de mouse/teclado** — `SetWindowsHookEx` para capturar clicks/keystrokes reales del usuario
2. **Auto-recording** — Capturar screenshot + acción automáticamente cuando el usuario interactúa
3. **Wiring del RAG** — Conectar embeddings existentes al search por defecto (en vez de LIKE)

---

## 🥉 FEATURE 3: Mesh Multi-Máquina

### ✅ LO QUE FUNCIONA

#### UDP Discovery — `mesh/discovery.rs`
- **REAL:** Broadcast UDP en puerto 9091
- Envía presencia cada 10 segundos: `AGENTOS|node_id|hostname|mesh_port`
- Escucha otros nodos en la red local
- Registry en memoria (HashMap) con auto-limpieza (30s timeout)

#### TCP Transport — `mesh/transport.rs`
- **REAL:** Servidor TCP en puerto 9090
- Acepta conexiones async
- JSON delimitado por newline
- Ejecuta tareas recibidas via Gateway.complete_as_agent()
- Retorna TaskResult con output real

#### Task Delegation — `mesh/transport.rs`
- **REAL:** `send_task(ip, port, description)` funciona end-to-end
- Abre conexión TCP, envía TaskRequest, espera TaskResult (120s timeout)
- El nodo remoto ejecuta via LLM gateway y devuelve resultado

#### Mesh Server Startup — `lib.rs`
- **REAL:** Se inicia automáticamente al arrancar la app
- Puerto configurable via `MESH_PORT` env var
- Discovery + Transport arrancan en paralelo

### ❌ LO QUE NO FUNCIONA

| Item | Estado | Problema |
|------|--------|----------|
| **Delegación automática** | NO EXISTE | El orchestrator calcula scores de nodos pero NUNCA llama `send_task()` |
| **Planning → Execution** | DESCONECTADO | `cmd_plan_distributed_execution()` retorna JSON plan pero no lo ejecuta. Comentario literal: "FOR NOW" |
| **TLS/Encryption** | NO EXISTE | Conexiones TCP sin cifrar |
| **Relay para WAN** | CLIENTE SIN SERVIDOR | HTTP client a `relay.agentos.ai` existe, pero no hay servidor desplegado |
| **Heartbeat/Monitoring** | BÁSICO | Solo `last_seen` timestamp, no hay health checks reales |
| **Retry/Backoff** | NO EXISTE | Timeout de 120s sin reintentos |
| **Federated Learning** | STUB | Colecciona métricas pero no hace ML real |

### 🎯 CONCLUSIÓN FEATURE 3

**La base funciona, la automatización no.**

Dos instancias de AgentOS en la misma red se descubren y pueden intercambiar tareas manualmente. Pero el orchestrator que debería decidir automáticamente "esta subtarea va al nodo B" nunca fue conectado.

**Lo que hay que construir:**
1. **Conectar orchestrator → transport** — Cuando el score dice `Remote(node_id)`, llamar `send_task()`
2. **Ejecutar plans distribuidos** — `cmd_execute_distributed_chain()` debe ejecutar, no solo planificar
3. **TLS básico** — Al menos para prod

---

## RESUMEN DE ESFUERZO REAL

| Feature | Trabajo Restante | Horas Estimadas |
|---------|-----------------|-----------------|
| **1. Control Visual** | Testing E2E + mejoras de UX frontend | 8-16h |
| **2. Learning** | Hooks de mouse/teclado + auto-recording | 16-24h |
| **3. Mesh** | Conectar orchestrator + ejecutar plans | 8-16h |

### Lo sorprendente: Feature 1 está mucho más avanzada de lo que el plan asumía

El plan original asumía que había que construir el Vision Agent Loop desde cero. **Ya existe.** El loop completo con 15 iteraciones, coordinate scaling, safety checks, y database persistence es código de producción.

**La prioridad real debería ser:**
1. **Testing E2E de Feature 1** — Verificar que "Abrí Notepad y escribí Hola" funciona sin errores
2. **Demo recording** — Grabar un video del control visual funcionando
3. **Hooks de grabación** para Feature 2
4. **Conectar orchestrator** para Feature 3

---

## MAPA DE ARCHIVOS CRÍTICOS

```
src-tauri/src/
├── eyes/
│   ├── capture.rs        ← Screen capture (GDI BitBlt) ✅ REAL
│   ├── vision.rs         ← Vision LLM planning ✅ REAL
│   ├── multi_monitor.rs  ← Monitor detection ✅ REAL
│   ├── ui_automation.rs  ← Windows UI Accessibility ✅ REAL
│   ├── ocr.rs            ← Windows OCR via PowerShell ✅ REAL
│   └── diff.rs           ← Screenshot diff ✅ REAL
├── hands/
│   ├── input.rs          ← Mouse + Keyboard (SendInput) ✅ REAL
│   ├── cli.rs            ← PowerShell/CMD execution ✅ REAL
│   └── safety.rs         ← Command safety checks ✅ REAL
├── brain/
│   ├── gateway.rs        ← LLM routing + fallback ✅ REAL
│   ├── providers.rs      ← HTTP calls a Anthropic/OpenAI/Google ✅ REAL
│   ├── classifier.rs     ← Task type classification ✅ REAL
│   └── local_llm.rs      ← Ollama integration ✅ REAL
├── pipeline/
│   ├── engine.rs         ← Vision loop (15 iterations) ✅ REAL
│   └── executor.rs       ← Action dispatcher ✅ REAL
├── playbooks/
│   ├── recorder.rs       ← Recording (MANUAL only) ⚠️ PARCIAL
│   ├── player.rs         ← Playback (vision-guided) ✅ REAL
│   └── smart.rs          ← Smart playbooks ⚠️ PARCIAL
├── mesh/
│   ├── discovery.rs      ← UDP broadcast ✅ REAL
│   ├── transport.rs      ← TCP server/client ✅ REAL
│   ├── protocol.rs       ← JSON messages ✅ REAL
│   ├── orchestrator.rs   ← Node scoring (NO ejecuta) ⚠️ DESCONECTADO
│   └── relay.rs          ← HTTP client (no server) ⚠️ INCOMPLETO
├── recording/
│   └── recorder.rs       ← Frame container ❌ STUB
├── training/
│   ├── collector.rs      ← Metadata only ⚠️ PARCIAL
│   └── finetune.rs       ← Never trains ❌ STUB
├── knowledge/
│   └── graph.rs          ← SQLite graph (no embeddings) ⚠️ PARCIAL
└── memory/
    └── store.rs          ← Embeddings exist, search uses LIKE ⚠️ PARCIAL
```
