# SPRINT PLAN — PHASE 2: LOS OJOS

**Proyecto:** AgentOS
**Fase:** 2 — The Eyes (Semanas 4–6)
**Sprints:** 3 (1 por semana)
**Preparado por:** Project Manager
**Fecha:** Marzo 2026
**Estado:** PENDIENTE APROBACIÓN DEL PRODUCT OWNER

---

## Objetivo de la fase

Darle al agente la capacidad de **ver y controlar la pantalla**. Al final de esta fase, el agente puede navegar una aplicación de escritorio siguiendo un playbook visual — screenshots de cada paso que el usuario grabó previamente.

Esto completa la cadena de ejecución: Phase 1 = CLI, Phase 2 = Screen Control. Con ambos modos, el agente puede operar cualquier software de la PC.

---

## Entregable final de la fase

El usuario graba un proceso (ej: abrir Chrome, ir a Gmail, escribir un email). AgentOS guarda screenshots de cada paso. Después, cuando el usuario pide "escríbele un email a Juan", el agente busca los screenshots más similares, reconoce la pantalla actual con un modelo de visión, y controla mouse/teclado para completar la tarea. Si hay fallback disponible (CLI o API), lo usa primero.

---

## Prerequisitos de Phase 1

| Componente de Phase 1 | Cómo lo usa Phase 2 |
|----------------------|---------------------|
| LLM Gateway (AOS-002) | Las llamadas al modelo de visión (Gemini Flash, GPT-4o) pasan por el Gateway |
| Task Classifier (AOS-003) | Se extiende para detectar tareas tipo VISION |
| CLI Executor (AOS-004) | Fallback chain: si Screen falla, intenta CLI |
| Context Folder Parser (AOS-005) | Se extiende para parsear `steps/` (screenshots + annotations) |
| Task Store (AOS-006) | Los screenshots y resultados de visión se registran |
| Agent Core (AOS-009) | Se extiende con el nuevo ScreenExecutor en el pipeline |

---

## Resumen de tickets

| Ticket | Título | Sprint | Prioridad | Asignado a | Depende de |
|--------|--------|--------|-----------|------------|------------|
| AOS-011 | Screen Capture — Captura y análisis de pantalla | S4 | Crítica | Software Architect → Backend Dev | Phase 1 completa |
| AOS-012 | Screen Controller — Control de mouse y teclado | S4 | Crítica | Software Architect → CISO → Backend Dev | AOS-011 |
| AOS-013 | Vision Model Integration — Análisis de UI con LLM | S4 | Alta | ML/AI Engineer | AOS-011, AOS-002 |
| AOS-014 | Visual Memory — Indexación de screenshots con CLIP | S5 | Crítica | ML/AI Engineer | AOS-011 |
| AOS-015 | Step Recorder — Grabación de procesos paso a paso | S5 | Alta | Software Architect → Backend Dev | AOS-011, AOS-014 |
| AOS-016 | CFP v2 — Extensión del parser para steps/ y annotations | S5 | Alta | API Designer → Backend Dev | AOS-005, AOS-014 |
| AOS-017 | Smart Mode Selection — Fallback chain API > CLI > Screen | S6 | Crítica | Software Architect → Backend Dev | AOS-012, AOS-004 |
| AOS-018 | Screen Executor — Integración en el pipeline del agente | S6 | Crítica | Software Architect → Backend Dev | AOS-012, AOS-013, AOS-017 |
| AOS-019 | Integración E2E Phase 2 y demo visual | S6 | Crítica | QA, Security Auditor | AOS-018 |

---

## Diagrama de dependencias

```
Phase 1 completa
    │
    ├── AOS-011 (Screen Capture) ─────┬── AOS-013 (Vision Model)
    │                                 ├── AOS-014 (Visual Memory / CLIP)
    │                                 │       ├── AOS-015 (Step Recorder)
    │                                 │       └── AOS-016 (CFP v2 Parser)
    │                                 │
    ├── AOS-012 (Screen Controller) ──┼── AOS-017 (Smart Mode Selection)
    │                                 │       │
    │                                 └───────┴── AOS-018 (Screen Executor)
    │                                                  │
    └──────────────────────────────────────── AOS-019 (E2E Phase 2)
```

---

## SPRINT 4 — VISIÓN (Semana 4)

**Objetivo:** El agente puede capturar screenshots, analizar contenido de pantalla con un LLM de visión, y controlar mouse/teclado.

### TICKET: AOS-011
**TITLE:** Screen Capture — Captura y análisis de pantalla
**PHASE:** 2-eyes
**SPRINT:** 4
**PRIORITY:** Crítica
**ASSIGNED TO:** Software Architect → Backend Dev
**DEPENDS ON:** Phase 1 completa

#### Descripción
Implementar la capacidad de capturar screenshots de la pantalla del usuario. Esto es la base de todo Phase 2 — sin captura, no hay visión. Incluye: captura completa, captura de región, captura de ventana activa, y conversión a formato compatible con los LLM de visión (base64 PNG/JPEG).

#### Criterios de aceptación
- [ ] Captura de pantalla completa funciona (PNG)
- [ ] Captura de región específica (x, y, width, height)
- [ ] Captura de ventana activa
- [ ] Conversión a base64 para envío a LLMs de visión
- [ ] Compresión configurable (JPEG quality para reducir tokens)
- [ ] Funciona en Linux (Windows en Phase 3)
- [ ] Tests con screenshots de prueba (no depende de display real en CI)

---

### TICKET: AOS-012
**TITLE:** Screen Controller — Control de mouse y teclado
**PHASE:** 2-eyes
**SPRINT:** 4
**PRIORITY:** Crítica
**ASSIGNED TO:** Software Architect → CISO → Backend Dev
**DEPENDS ON:** AOS-011

#### Descripción
Implementar control de mouse (mover, click, double-click, right-click, drag, scroll) y teclado (type text, hotkeys, key combinations) usando pyautogui. Incluye safety features: fail-safe corner, delay entre acciones, y confirmación para acciones destructivas.

#### Criterios de aceptación
- [ ] Acciones de mouse: move, click, double_click, right_click, drag, scroll
- [ ] Acciones de teclado: type_text, press_key, hotkey (ej: Ctrl+C)
- [ ] Fail-safe: mover mouse a esquina superior-izquierda aborta la ejecución
- [ ] Delay configurable entre acciones (default: 0.5s)
- [ ] Logging de cada acción ejecutada
- [ ] Safety: confirmación requerida para acciones que podrían ser destructivas
- [ ] Tests con mocks de pyautogui

---

### TICKET: AOS-013
**TITLE:** Vision Model Integration — Análisis de UI con LLM
**PHASE:** 2-eyes
**SPRINT:** 4
**PRIORITY:** Alta
**ASSIGNED TO:** ML/AI Engineer
**DEPENDS ON:** AOS-011, AOS-002

#### Descripción
Integrar modelos de visión (Gemini Flash, GPT-4o, Claude Sonnet) a través del LLM Gateway existente para analizar screenshots. El agente envía un screenshot + prompt al modelo de visión y recibe una descripción estructurada de lo que ve: elementos UI, texto visible, estado de la aplicación, y qué acciones tomar.

#### Criterios de aceptación
- [ ] Enviar screenshot + prompt al LLM Gateway (nueva funcionalidad vision)
- [ ] Recibir respuesta estructurada: elementos UI detectados, texto visible, acción sugerida
- [ ] Prompt templates para: "describe screen", "find element", "what changed"
- [ ] Funciona con al menos 2 proveedores de visión (ej: Gemini Flash + GPT-4o)
- [ ] Compresión inteligente: reduce resolución si el screenshot es muy grande (ahorra tokens)
- [ ] Tests con screenshots estáticos (no depende de display)

---

## SPRINT 5 — MEMORIA VISUAL (Semana 5)

**Objetivo:** El agente puede indexar screenshots, buscar los más similares, y grabar procesos paso a paso.

### TICKET: AOS-014
**TITLE:** Visual Memory — Indexación de screenshots con CLIP
**PHASE:** 2-eyes
**SPRINT:** 5
**PRIORITY:** Crítica
**ASSIGNED TO:** ML/AI Engineer
**DEPENDS ON:** AOS-011

#### Descripción
Implementar el sistema de memoria visual usando CLIP embeddings. Cada screenshot se convierte en un vector embedding. Cuando el agente necesita encontrar el paso más relevante para la pantalla actual, compara el embedding del screenshot actual con los embeddings almacenados y retorna los más similares.

#### Criterios de aceptación
- [ ] Generar CLIP embedding de un screenshot (vector de 512 o 768 dims)
- [ ] Almacenar embeddings en SQLite (tabla nueva: visual_memory)
- [ ] Búsqueda por similitud: dado un screenshot, encontrar los top-N más similares
- [ ] Funciona offline después de descargar el modelo CLIP una vez
- [ ] Funciona en CPU (no requiere GPU)
- [ ] Latencia de búsqueda < 100ms para 1000 screenshots indexados
- [ ] Tests con set de screenshots de prueba

---

### TICKET: AOS-015
**TITLE:** Step Recorder — Grabación de procesos paso a paso
**PHASE:** 2-eyes
**SPRINT:** 5
**PRIORITY:** Alta
**ASSIGNED TO:** Software Architect → Backend Dev
**DEPENDS ON:** AOS-011, AOS-014

#### Descripción
Implementar el modo de grabación: el usuario realiza una tarea manualmente mientras AgentOS captura screenshots en cada paso significativo. Cada screenshot se indexa con CLIP y se guarda en la carpeta `steps/` del playbook activo. El usuario puede agregar anotaciones markdown opcionales.

#### Criterios de aceptación
- [ ] Iniciar/detener grabación
- [ ] Captura automática en eventos: click de mouse, tecla Enter, cambio de ventana activa
- [ ] Captura manual (hotkey configurable, ej: F9)
- [ ] Cada screenshot se nombra secuencialmente: 01-*.png, 02-*.png...
- [ ] Cada screenshot se indexa con CLIP automáticamente
- [ ] El usuario puede agregar anotación markdown para cada paso
- [ ] Los screenshots se guardan en steps/ del playbook activo
- [ ] Tests del flujo de grabación con mocks

---

### TICKET: AOS-016
**TITLE:** CFP v2 — Extensión del parser para steps/ y annotations
**PHASE:** 2-eyes
**SPRINT:** 5
**PRIORITY:** Alta
**ASSIGNED TO:** API Designer → Backend Dev
**DEPENDS ON:** AOS-005, AOS-014

#### Descripción
Extender el Context Folder Protocol Parser (AOS-005) para soportar la carpeta `steps/` con screenshots y anotaciones markdown. El parser debe leer los screenshots, cargar sus embeddings CLIP si existen, y asociar cada screenshot con su anotación.

#### Criterios de aceptación
- [ ] Parser lee `steps/*.png` y los ordena por número
- [ ] Parser lee `steps/*.md` y los asocia al screenshot correspondiente
- [ ] ContextFolder ahora tiene `steps: list[StepRecord]` con imagen + anotación + embedding
- [ ] Backward compatible: playbooks sin steps/ siguen funcionando (v1)
- [ ] Validación: si hay .md sin .png correspondiente, warning en log
- [ ] Tests con playbooks v1 (sin steps) y v2 (con steps)

---

## SPRINT 6 — EJECUCIÓN VISUAL (Semana 6)

**Objetivo:** El agente ejecuta tareas usando control de pantalla, con fallback inteligente.

### TICKET: AOS-017
**TITLE:** Smart Mode Selection — Fallback chain API > CLI > Screen
**PHASE:** 2-eyes
**SPRINT:** 6
**PRIORITY:** Crítica
**ASSIGNED TO:** Software Architect → Backend Dev
**DEPENDS ON:** AOS-012, AOS-004

#### Descripción
Implementar la lógica de selección inteligente de modo de ejecución. El agente analiza la tarea y decide: ¿se puede hacer por API? Si no, ¿se puede hacer por CLI? Si no, usar Screen Control. Si un modo falla, cae automáticamente al siguiente.

#### Criterios de aceptación
- [ ] Enum ExecutorMode: API, CLI, SCREEN con orden de preferencia
- [ ] Lógica de selección basada en: tipo de tarea, playbook config, permisos
- [ ] Fallback automático: si CLI falla, intenta Screen (si tiene permiso)
- [ ] Logging de cada decisión de modo y fallback
- [ ] El playbook config puede forzar un modo específico
- [ ] Tests de la cadena de fallback completa

---

### TICKET: AOS-018
**TITLE:** Screen Executor — Integración en el pipeline del agente
**PHASE:** 2-eyes
**SPRINT:** 6
**PRIORITY:** Crítica
**ASSIGNED TO:** Software Architect → Backend Dev
**DEPENDS ON:** AOS-012, AOS-013, AOS-017

#### Descripción
Implementar el ScreenExecutor que se integra al pipeline del AgentCore. El flujo es: capturar pantalla → enviar a vision model → recibir instrucciones → ejecutar acciones de mouse/teclado → verificar resultado con nueva captura → repetir hasta completar.

#### Criterios de aceptación
- [ ] ScreenExecutor implementa la misma interfaz que CLIExecutor
- [ ] Loop de ejecución: capture → analyze → act → verify → repeat
- [ ] Máximo de iterations configurable (default: 20, evita loops infinitos)
- [ ] Timeout global para la ejecución visual
- [ ] Integrado en AgentCore.process() — se selecciona vía Smart Mode Selection
- [ ] Busca en visual memory si hay playbook con steps/
- [ ] Tests del loop con screenshots y acciones mockeadas

---

### TICKET: AOS-019
**TITLE:** Integración E2E Phase 2 y demo visual
**PHASE:** 2-eyes
**SPRINT:** 6
**PRIORITY:** Crítica
**ASSIGNED TO:** QA, Security Auditor
**DEPENDS ON:** AOS-018

#### Criterios de aceptación
- [ ] **Demo:** El agente navega una aplicación siguiendo screenshots de un playbook visual
- [ ] **Fallback funciona:** Comando que puede hacerse por CLI se hace por CLI (no por Screen)
- [ ] **Recording funciona:** Grabar un proceso de 5 pasos, reproducirlo con el agente
- [ ] **Visual search funciona:** CLIP encuentra el screenshot correcto para la pantalla actual
- [ ] **Security:** El Screen Controller no puede ejecutar acciones sin permiso "screen" en el playbook
- [ ] **Performance:** Captura + análisis de visión < 3 segundos por paso
- [ ] Todos los tests de Phase 1 siguen pasando

---

## Riesgos

| Riesgo | Probabilidad | Impacto | Mitigación |
|--------|-------------|---------|------------|
| pyautogui no funciona en todos los desktop environments | Media | Alto | Fallback a xdotool en Linux. Testar en X11 y Wayland. |
| Modelo CLIP es demasiado pesado para CPU | Media | Alto | Usar CLIP ViT-B/32 (más pequeño). Quantización INT8 si necesario. |
| Vision models dan instrucciones incorrectas | Alta | Medio | Verificación post-acción: captura screenshot después de cada acción y compara. |
| Screenshots leakean información sensible | Media | Alto | NUNCA almacenar screenshots en logs/métricas. Solo en la carpeta del playbook. |

---

## Criterios de éxito de Phase 2

| Métrica | Target |
|---------|--------|
| Vision analysis accuracy (identifica elementos correctos) | > 80% |
| CLIP search accuracy (encuentra screenshot correcto) | > 85% para top-3 |
| Screen control task completion rate | > 70% |
| Captura + análisis por paso | < 3 segundos |
| CLIP model load time | < 5 segundos |
| CLIP search latency (1000 screenshots) | < 100ms |
| Memory overhead with CLIP loaded | < 500 MB |
