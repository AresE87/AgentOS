# AgentOS v2 — Roadmap R11-R20: De "compila" a "funciona de verdad"

**Fecha:** 29 de marzo de 2026
**Estado:** R1-R10 completas. 132 tests, 39 IPC, UI con datos reales.
**Problema:** Muchas features son estructura sin funcionalidad real. La app compila y se ve bien, pero el agente no es autónomo de verdad.

---

## Diagnóstico honesto del estado actual

| Feature | Dice que funciona | Realmente funciona |
|---------|------------------|-------------------|
| Chat con LLM | ✅ Sí | ✅ Sí — probado |
| PowerShell commands | ✅ Sí | ✅ Sí — probado |
| Auto-retry | ✅ Sí | ✅ Sí — probado |
| Clasificador | ✅ Sí | ⚠️ Parcial — reglas básicas, no siempre elige bien |
| 40 especialistas | ✅ Sí | ⚠️ Son system prompts, no se seleccionan inteligentemente |
| Vision mode | ⚠️ Código existe | ❌ Nunca se probó E2E con una tarea real |
| Playbook recorder | ⚠️ IPC existe | ❌ Nadie grabó un playbook y lo reprodujo |
| Playbook player | ⚠️ IPC existe | ❌ Nunca reprodujo un playbook de verdad |
| Board Kanban | ⚠️ UI existe | ❌ Las cadenas no se ejecutan de verdad |
| Task decomposition | ⚠️ IPC existe | ❌ decompose_task existe pero no se integra al pipeline |
| Telegram | ⚠️ Reescrito | ❌ Nunca se probó con un bot real |
| Discord | ⚠️ HTTP only | ❌ No funciona en servidores reales (necesita WebSocket) |
| Analytics | ⚠️ Queries existen | ⚠️ Funciona si hay datos, pero charts pueden estar vacíos |
| Suggestions | ⚠️ Engine existe | ⚠️ Solo detecta repeticiones, muy limitado |
| Mesh discovery | ⚠️ Stub | ❌ No descubre nodos reales |
| Mesh communication | ❌ No existe | ❌ No hay transporte WebSocket |
| Tray icon | ❌ No existe | ❌ No hay system tray |
| Auto-update | ❌ No existe | ❌ No hay updater |
| Triggers/scheduled | ❌ No existe | ❌ |

**El patrón:** Code implementó las interfaces y la UI para todo, pero muchos backends son stubs que retornan datos vacíos o hardcoded. La app se VE completa pero no HACE la mitad de lo que muestra.

---

## Las 10 fases R11-R20

| Fase | Nombre | Qué resuelve |
|------|--------|-------------|
| R11 | **Vision funcional** | El agente VE y ACTÚA en la pantalla de verdad — probado con 5 tareas reales |
| R12 | **Orchestrator real** | Las cadenas de sub-tareas se ejecutan DE VERDAD, no solo se muestran |
| R13 | **Playbooks que funcionan** | Grabar una tarea real, reproducirla real, con UI completa |
| R14 | **Canales probados** | Telegram funciona con bot real. Discord funciona con bot real. |
| R15 | **System tray + lifecycle** | Tray icon, minimizar a tray, auto-start, ciclo de vida completo |
| R16 | **Mesh funcional** | Dos PCs se descubren, se conectan, se envían tareas DE VERDAD |
| R17 | **Especialistas inteligentes** | El orchestrator ELIGE el especialista correcto y SE NOTA la diferencia |
| R18 | **Triggers y automatización** | Tareas programadas: cron, file watchers, acciones automáticas |
| R19 | **Web browsing real** | El agente navega websites de verdad (no solo Invoke-WebRequest) |
| R20 | **Hardening + Release real** | Auto-update, firma, tray, onboarding perfecto, release público |

---

## Principio de esta ronda

**Nada de scaffolding.** Cada fase termina con una DEMOSTRACIÓN que se puede grabar en video. Si no podés mostrarlo funcionando, no está hecho.
