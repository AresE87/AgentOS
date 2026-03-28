# SPRINT PLAN — PHASE 2: LOS OJOS

**Proyecto:** AgentOS
**Fase:** 2 — The Eyes (Semanas 4–6)
**Sprints:** 3 (1 por semana)
**Preparado por:** Project Manager
**Fecha:** Marzo 2026
**Estado:** PENDIENTE APROBACIÓN DEL PRODUCT OWNER

---

## Objetivo de la fase

Darle al agente la capacidad de VER y CONTROLAR la pantalla del usuario. Esto transforma a AgentOS de un agente que solo ejecuta comandos CLI a un agente que puede operar cualquier aplicación con interfaz gráfica — como lo haría un humano mirando la pantalla.

---

## Entregable final de la fase

El agente recibe la instrucción "abre Chrome, navega a gmail.com y lee el último email". Toma un screenshot, identifica elementos de la UI vía un modelo de visión, controla el mouse y teclado para navegar, y devuelve el contenido del email al usuario por Telegram. Además, el usuario puede grabar una tarea paso a paso y el agente la replica automáticamente.

---

## Prerequisitos

- **Phase 1 completa:** Gateway, Classifier, Executor CLI, Parser, Store, Telegram, Agent Core — todos funcionando.
- **El Classifier de Phase 1 ya soporta TaskType.VISION** — en Phase 2 implementamos el executor que lo procesa.

---

## Resumen de tickets

| Ticket | Título | Sprint | Prioridad | Asignado a | Depende de |
|--------|--------|--------|-----------|------------|------------|
| AOS-011 | Screen Capture — Servicio de captura de pantalla | S4 | Crítica | Software Architect → Backend Dev | Phase 1 completa |
| AOS-012 | Vision Analyzer — Integración con modelo de visión | S4 | Crítica | ML/AI Engineer → Backend Dev | AOS-011 |
| AOS-013 | Screen Controller — Control de mouse y teclado | S4 | Crítica | Software Architect → Backend Dev | AOS-011 |
| AOS-014 | Visual Memory — Indexación de screenshots con CLIP | S5 | Alta | ML/AI Engineer → Backend Dev | AOS-011, AOS-012 |
| AOS-015 | Screen Executor — Ejecución de tareas vía GUI | S5 | Crítica | Software Architect → Backend Dev | AOS-012, AOS-013 |
| AOS-016 | Step Recorder — Modo de grabación paso a paso | S5 | Alta | Backend Dev | AOS-011, AOS-013, AOS-014 |
| AOS-017 | Smart Mode Selection — Fallback API > CLI > Screen | S6 | Crítica | Software Architect → Backend Dev | AOS-015 |
| AOS-018 | Integración Phase 2 — E2E y demo visual | S6 | Crítica | QA, Security Auditor | AOS-015, AOS-016, AOS-017 |

---

## Diagrama de dependencias

```
Phase 1 (completa)
    │
    └── AOS-011 (Screen Capture) ──────────────────────────────┐
            ├── AOS-012 (Vision Analyzer) ──┬── AOS-015 (Screen Executor)
            ├── AOS-013 (Screen Controller) ┘         │
            │         │                               │
            ├── AOS-014 (Visual Memory) ──── AOS-016 (Step Recorder)
            │                                         │
            └─────────────────────────── AOS-017 (Smart Mode Selection)
                                                      │
                                              AOS-018 (E2E Phase 2)
```

---

## SPRINT 4 — VISIÓN Y CONTROL (Semana 4)

**Objetivo:** El agente puede tomar screenshots, entender qué hay en pantalla, y mover el mouse/teclado.

---

### TICKET: AOS-011
**TITLE:** Screen Capture — Servicio de captura de pantalla
**PHASE:** 2-eyes
**SPRINT:** 4
**PRIORITY:** Crítica
**ASSIGNED TO:** Software Architect (arquitectura) → Backend Dev (implementación)
**DEPENDS ON:** Phase 1 completa
**BLOCKED BY:** Ninguno

#### Descripción
Implementar un servicio de captura de pantalla que tome screenshots del escritorio completo o de regiones específicas, los guarde como imágenes optimizadas, y los haga disponibles para el Vision Analyzer y el Visual Memory.

#### Criterios de aceptación
- [ ] Captura de pantalla completa funcional (PNG)
- [ ] Captura de región específica (x, y, width, height)
- [ ] Optimización de imagen: resize a resolución configurable para enviar al modelo de visión (default: 1280px de ancho)
- [ ] Encoding a base64 para envío al LLM
- [ ] Captura periódica configurable (para monitoring de pantalla)
- [ ] Funciona en Linux (Phase 2), Windows se agrega en Phase 3
- [ ] Async interface compatible con el event loop del agente
- [ ] Tests con screenshots sintéticos (no depende de display real para CI)

#### Notas
Dependencias: `Pillow` para manipulación de imagen, `mss` (multi-screen shot) para captura cross-platform. `mss` es más rápido y portable que `pyautogui.screenshot()`.

---

### TICKET: AOS-012
**TITLE:** Vision Analyzer — Integración con modelo de visión
**PHASE:** 2-eyes
**SPRINT:** 4
**PRIORITY:** Crítica
**ASSIGNED TO:** ML/AI Engineer (diseño) → Backend Dev (implementación)
**DEPENDS ON:** AOS-011
**BLOCKED BY:** Ninguno

#### Descripción
Integrar el LLM Gateway con capacidad de enviar imágenes (screenshots) al modelo de visión. El Vision Analyzer recibe un screenshot + una instrucción en texto, y retorna un análisis estructurado de la pantalla: qué elementos hay, dónde están, qué texto se ve.

#### Criterios de aceptación
- [ ] Envío de imagen + prompt al LLM a través del Gateway existente (multimodal)
- [ ] Soporte para al menos 2 providers de visión: GPT-4o (OpenAI) y Gemini Flash (Google)
- [ ] Respuesta estructurada: lista de elementos UI detectados con tipo, texto, y coordenadas aproximadas
- [ ] Modo "describe": dado un screenshot, describe qué se ve en lenguaje natural
- [ ] Modo "locate": dado un screenshot + "encuentra el botón Submit", retorna coordenadas
- [ ] Modo "read": dado un screenshot, extrae todo el texto visible (OCR vía visión)
- [ ] Caching: si el screenshot no cambió (hash), reusar análisis previo
- [ ] Latencia del análisis documentada (target: < 3s para describe, < 2s para locate)
- [ ] Tests con imágenes sintéticas + mocks del LLM

---

### TICKET: AOS-013
**TITLE:** Screen Controller — Control de mouse y teclado
**PHASE:** 2-eyes
**SPRINT:** 4
**PRIORITY:** Crítica
**ASSIGNED TO:** Software Architect (arquitectura) → Backend Dev (implementación)
**DEPENDS ON:** AOS-011
**BLOCKED BY:** Ninguno

#### Descripción
Implementar un controlador que pueda mover el mouse, hacer click, escribir texto, y ejecutar atajos de teclado de forma programática. Este es el "cuerpo" del agente en la pantalla.

#### Criterios de aceptación
- [ ] Mover mouse a coordenadas (x, y) con velocidad configurable (no teleport instantáneo)
- [ ] Click: left, right, double, drag
- [ ] Escribir texto con velocidad configurable (simula typing humano)
- [ ] Atajos de teclado: Ctrl+C, Ctrl+V, Alt+Tab, etc.
- [ ] Scroll: up, down, con cantidad configurable
- [ ] Wait/delay entre acciones (configurable)
- [ ] Screenshot de confirmación después de cada acción (para verificar resultado)
- [ ] Mecanismo de "kill switch": tecla configurable que detiene todas las acciones inmediatamente
- [ ] Funciona en Linux (Phase 2), Windows en Phase 3
- [ ] Tests unitarios (mocks de pyautogui), tests de integración opcionales (requieren display)

#### Notas
Dependencias: `pyautogui` para control básico. Considerar `pynput` como alternativa para el kill switch (listener de teclado en background).

---

## SPRINT 5 — MEMORIA Y EJECUCIÓN VISUAL (Semana 5)

**Objetivo:** El agente puede recordar lo que vio, ejecutar tareas completas vía GUI, y grabar tareas del usuario.

---

### TICKET: AOS-014
**TITLE:** Visual Memory — Indexación de screenshots con CLIP embeddings
**PHASE:** 2-eyes
**SPRINT:** 5
**PRIORITY:** Alta
**ASSIGNED TO:** ML/AI Engineer (diseño) → Backend Dev (implementación)
**DEPENDS ON:** AOS-011, AOS-012
**BLOCKED BY:** Ninguno

#### Descripción
Implementar un sistema de memoria visual que almacena screenshots con embeddings CLIP, permitiendo buscar por similitud visual o por texto. Esto permite al agente recordar "ya vi esta pantalla antes" y reutilizar acciones previas exitosas.

#### Criterios de aceptación
- [ ] Generar embeddings CLIP para cada screenshot capturado
- [ ] Almacenar embeddings + metadata en SQLite (nueva tabla `visual_memory`)
- [ ] Búsqueda por similitud de imagen: dado un screenshot, encontrar los más similares
- [ ] Búsqueda por texto: dado "pantalla de login de Gmail", encontrar screenshots que matcheen
- [ ] Cleanup automático: limitar storage a N screenshots configurables (default: 1000)
- [ ] LRU eviction: cuando se llena, eliminar el más antiguo no-pinned
- [ ] Funciona con modelo CLIP local (no requiere API call para embeddings)
- [ ] Tests con imágenes sintéticas

#### Notas
Dependencias: `transformers` + `torch` (CLIP model), o `open_clip_torch` para una versión más ligera. El modelo CLIP corre localmente — no consume tokens de API. El peso del modelo (~400 MB) se descarga al primer uso.

---

### TICKET: AOS-015
**TITLE:** Screen Executor — Ejecución de tareas vía GUI
**PHASE:** 2-eyes
**SPRINT:** 5
**PRIORITY:** Crítica
**ASSIGNED TO:** Software Architect (arquitectura) → Backend Dev (implementación)
**DEPENDS ON:** AOS-012, AOS-013
**BLOCKED BY:** Ninguno

#### Descripción
Implementar el executor que combina Vision Analyzer + Screen Controller para ejecutar tareas completas en la GUI. Este es el equivalente "visual" del CLI Executor: recibe una instrucción, usa visión para entender la pantalla, y controla mouse/teclado para ejecutarla.

#### Criterios de aceptación
- [ ] Recibe instrucción en texto natural + ejecuta vía GUI
- [ ] Loop de ejecución: screenshot → analyze → act → screenshot → verify → repeat
- [ ] Máximo N iteraciones por tarea (configurable, default: 20) para prevenir loops infinitos
- [ ] Detección de "tarea completada" vía análisis visual
- [ ] Detección de "estoy atascado" (misma pantalla después de actuar) → retry o reportar error
- [ ] Genera log visual: secuencia de screenshots con anotaciones de cada acción
- [ ] Retorna ExecutionResult compatible con el pipeline existente (ExecutorType.SCREEN)
- [ ] Safety: pregunta al usuario antes de acciones destructivas (si detecta diálogos de confirmación)
- [ ] Tests con mocks del Vision Analyzer y Screen Controller

---

### TICKET: AOS-016
**TITLE:** Step Recorder — Modo de grabación paso a paso
**PHASE:** 2-eyes
**SPRINT:** 5
**PRIORITY:** Alta
**ASSIGNED TO:** Backend Dev
**DEPENDS ON:** AOS-011, AOS-013, AOS-014
**BLOCKED BY:** Ninguno

#### Descripción
Implementar un modo donde el usuario ejecuta una tarea manualmente mientras el agente observa: captura cada screenshot, registra cada acción de mouse/teclado, y genera un playbook visual que puede replicar automáticamente.

#### Criterios de aceptación
- [ ] Iniciar grabación: el agente empieza a capturar screenshots + eventos de input
- [ ] Capturar cada click, keystroke, y scroll con timestamps y coordenadas
- [ ] Capturar screenshot antes y después de cada acción
- [ ] Detener grabación: el agente procesa la secuencia y genera un playbook
- [ ] El playbook generado incluye: screenshots de referencia + instrucciones en lenguaje natural
- [ ] El playbook se guarda como Context Folder (CFP v2 con directorio `steps/`)
- [ ] Replay: el agente puede reproducir el playbook grabado
- [ ] El replay usa Visual Memory para adaptarse si la pantalla cambió ligeramente
- [ ] Tests con secuencias de eventos sintéticos

#### Notas
Dependencias: `pynput` para capturar eventos de mouse/teclado en background.

---

## SPRINT 6 — INTEGRACIÓN Y SMART MODE (Semana 6)

**Objetivo:** Todo conectado — el agente elige automáticamente entre API, CLI, y Screen, y la demo visual funciona end-to-end.

---

### TICKET: AOS-017
**TITLE:** Smart Mode Selection — Fallback API > CLI > Screen
**PHASE:** 2-eyes
**SPRINT:** 6
**PRIORITY:** Crítica
**ASSIGNED TO:** Software Architect (arquitectura) → Backend Dev (implementación)
**DEPENDS ON:** AOS-015
**BLOCKED BY:** Ninguno

#### Descripción
Implementar el sistema de selección inteligente de modo de ejecución. El agente debe preferir API > CLI > Screen Control (en ese orden), con fallback automático si un modo falla.

#### Criterios de aceptación
- [ ] Selector de modo integrado en el Agent Core (modifica AOS-009)
- [ ] Reglas de selección: si la tarea tiene API disponible → API, si no → CLI, si no → Screen
- [ ] Fallback automático: si CLI falla con "command not found" → intenta Screen
- [ ] Registro de qué modo se usó y por qué en el TaskStore
- [ ] El Classifier de AOS-003 se extiende para sugerir modo preferido
- [ ] Modo manual: el usuario puede forzar un modo específico ("usa screen control para esto")
- [ ] Tests: simular fallo de CLI → fallback a Screen funciona

#### Notas
En Phase 2, el modo API no está implementado (viene en Phase 3+). El selector solo elige entre CLI y Screen. La interfaz debe soportar los 3 modos para futura extensión.

---

### TICKET: AOS-018
**TITLE:** Integración Phase 2 — E2E y demo visual
**PHASE:** 2-eyes
**SPRINT:** 6
**PRIORITY:** Crítica
**ASSIGNED TO:** QA, Security Auditor
**DEPENDS ON:** AOS-015, AOS-016, AOS-017
**BLOCKED BY:** Ninguno

#### Descripción
Verificación end-to-end de toda la Phase 2: el agente ve la pantalla, la controla, recuerda lo que vio, graba tareas, y selecciona el modo óptimo.

#### Criterios de aceptación
- [ ] **Demo funcional:** "abre el navegador, navega a example.com, lee el título de la página" — funciona E2E
- [ ] **Step recording demo:** grabar una tarea, reproducirla — funciona
- [ ] **Smart mode:** enviar tarea CLI → se ejecuta vía CLI. Enviar tarea visual → se ejecuta vía Screen
- [ ] **Visual memory:** buscar "pantalla de login" → encuentra screenshots relevantes
- [ ] **Kill switch:** presionar tecla de emergencia → todas las acciones de pantalla se detienen
- [ ] **Security audit:** screen control requiere permiso explícito del playbook
- [ ] **Security audit:** screenshots en Visual Memory no contienen información sensible visible (o se advierte)
- [ ] **Performance:** screenshot + analyze + act < 5 segundos por iteración
- [ ] Todos los tests de Phase 1 siguen pasando (no regression)

---

## Nuevas dependencias Python para Phase 2

```
mss >= 9.0              # Screen capture (multi-platform, fast)
pyautogui >= 0.9.54     # Mouse/keyboard control
pynput >= 1.7           # Keyboard/mouse event listener (for recorder + kill switch)
Pillow >= 10.0          # Image processing
open-clip-torch >= 2.24 # CLIP embeddings (local model)
torch >= 2.2            # PyTorch (CLIP dependency)
numpy >= 1.26           # Array operations for embeddings
```

Nota: `torch` es pesado (~2 GB). Considerar `torch-cpu` para instalaciones sin GPU.

---

## Nuevos módulos en el repositorio

```
agentos/
├── screen/                    # NUEVO — Módulo de control de pantalla
│   ├── __init__.py
│   ├── capture.py             # AOS-011: ScreenCapture service
│   ├── analyzer.py            # AOS-012: VisionAnalyzer
│   ├── controller.py          # AOS-013: ScreenController
│   ├── memory.py              # AOS-014: VisualMemory (CLIP)
│   ├── executor.py            # AOS-015: ScreenExecutor
│   └── recorder.py            # AOS-016: StepRecorder
├── executor/
│   ├── cli.py                 # Existente (Phase 1)
│   ├── safety.py              # Existente (Phase 1)
│   └── mode_selector.py       # AOS-017: SmartModeSelector (NUEVO)
```

---

## Riesgos identificados para Phase 2

| Riesgo | Probabilidad | Impacto | Mitigación |
|--------|-------------|---------|------------|
| pyautogui no funciona en headless/CI | Alta | Medio | Tests con mocks. Tests de integración solo en máquina con display. |
| Modelo de visión impreciso en UI elements | Media | Alto | Usar modelos multimodal de última generación. Retry con prompt refinado. |
| CLIP model download es lento/grande (2 GB) | Media | Medio | Usar open_clip con modelo más ligero (ViT-B/32 ~400 MB). Lazy download. |
| Screen control causa daños (click en lugar equivocado) | Media | Alto | Kill switch, confirmación para acciones destructivas, max iterations. |
| Step Recorder captura passwords/info sensible | Alta | Alto | Warning al usuario. Opción de pausar grabación. No grabar keypresses en campos password. |

---

## Criterios de éxito de Phase 2

| Métrica | Target |
|---------|--------|
| Task success rate (screen control) | > 70% |
| Screenshot → analyze → act latency | < 5 seconds |
| Visual memory search accuracy | > 80% relevance |
| Step recording fidelity | > 85% replay success |
| Kill switch response time | < 500ms |
| Phase 1 regression | 0 failures |
