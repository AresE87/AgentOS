# SPRINT PLAN — PHASE 3: EL CUERPO

**Proyecto:** AgentOS
**Fase:** 3 — The Body (Semanas 7–10)
**Sprints:** 4 (1 por semana)
**Preparado por:** Project Manager
**Fecha:** Marzo 2026
**Estado:** PENDIENTE APROBACIÓN DEL PRODUCT OWNER

---

## Objetivo de la fase

Convertir el agente Python (Phase 1+2) en una **aplicación de escritorio nativa** que cualquier usuario no-técnico pueda instalar con doble-click. La instalación debe sentirse como instalar un videojuego: sin terminal, sin PATH, sin instalar Python.

---

## Entregable final de la fase

Un archivo `AgentOS-Setup.msi` (Windows) de menos de 50 MB. El usuario hace doble-click, se instala, se abre un setup wizard de 5 pasos (< 2 minutos), y el agente empieza a funcionar con un ícono en la bandeja del sistema. El dashboard muestra tareas activas, playbooks instalados, chat integrado, y configuración.

---

## Resumen de tickets

| Ticket | Título | Sprint | Prioridad | Asignado a | Depende de |
|--------|--------|--------|-----------|------------|------------|
| AOS-020 | Tauri Shell — Aplicación nativa con IPC bridge | S7 | Crítica | Software Architect → DevOps | Phase 2 completa |
| AOS-021 | IPC Bridge — Comunicación Rust ↔ Python ↔ React | S7 | Crítica | API Designer → Backend Dev | AOS-020 |
| AOS-022 | Setup Wizard — Onboarding de 5 pasos | S8 | Crítica | UX/UI → Frontend Dev | AOS-021 |
| AOS-023 | System Tray — Ícono, menú, notificaciones | S8 | Alta | Frontend Dev + DevOps | AOS-020 |
| AOS-024 | Dashboard Home — Tareas activas, stats, quick actions | S9 | Crítica | UX/UI → Frontend Dev | AOS-021 |
| AOS-025 | Dashboard Playbooks — Browse, install, manage | S9 | Alta | Frontend Dev | AOS-021 |
| AOS-026 | Dashboard Chat — Conversación integrada con el agente | S9 | Alta | Frontend Dev | AOS-021 |
| AOS-027 | Dashboard Settings — API keys, permisos, configuración | S9 | Alta | Frontend Dev + CISO | AOS-021 |
| AOS-028 | Python Bundling — Empaquetar Python dentro de Tauri | S10 | Crítica | DevOps | AOS-020 |
| AOS-029 | Auto-Update — Mecanismo de actualización silenciosa | S10 | Alta | DevOps | AOS-028 |
| AOS-030 | Build & Package — Instalador .msi y firma | S10 | Crítica | DevOps | AOS-028, AOS-029 |
| AOS-031 | Integración E2E Phase 3 y demo de instalación | S10 | Crítica | QA | Todo |

---

## Diagrama de dependencias

```
Phase 2 completa
    │
    ├── AOS-020 (Tauri Shell) ──┬── AOS-021 (IPC Bridge)
    │                           │       ├── AOS-022 (Setup Wizard)
    │                           │       ├── AOS-024 (Dashboard Home)
    │                           │       ├── AOS-025 (Dashboard Playbooks)
    │                           │       ├── AOS-026 (Dashboard Chat)
    │                           │       └── AOS-027 (Dashboard Settings)
    │                           │
    │                           ├── AOS-023 (System Tray)
    │                           │
    │                           └── AOS-028 (Python Bundling)
    │                                   ├── AOS-029 (Auto-Update)
    │                                   └── AOS-030 (Build & Package)
    │                                           │
    └────────────────────────── AOS-031 (E2E Phase 3)
```

---

## SPRINT 7 — FUNDACIÓN DESKTOP (Semana 7)

**Objetivo:** Tauri shell funcional con comunicación bidireccional Rust ↔ Python ↔ React.

### TICKET: AOS-020
**TITLE:** Tauri Shell — Aplicación nativa con IPC bridge
**SPRINT:** 7
**PRIORITY:** Crítica
**ASSIGNED TO:** Software Architect → DevOps

#### Descripción
Crear la aplicación Tauri funcional: ventana principal, configuración de permisos Tauri (allowlist), y el mecanismo para lanzar el proceso Python del agente como child process desde Rust.

#### Criterios de aceptación
- [ ] `cargo tauri dev` abre una ventana con el frontend React
- [ ] Tauri lanza el proceso Python del agente como sidecar/child process
- [ ] Rust puede enviar comandos al proceso Python (stdin/stdout JSON-RPC)
- [ ] Rust puede recibir eventos del proceso Python
- [ ] Si el proceso Python crashea, Tauri lo detecta y lo reinicia
- [ ] tauri.conf.json con permisos mínimos (allowlist)
- [ ] Funciona en Windows y Linux

### TICKET: AOS-021
**TITLE:** IPC Bridge — Comunicación Rust ↔ Python ↔ React
**SPRINT:** 7
**PRIORITY:** Crítica
**ASSIGNED TO:** API Designer → Backend Dev

#### Descripción
Definir e implementar el protocolo de comunicación entre las tres capas: React (frontend) ↔ Rust (shell) ↔ Python (agente). React invoca Tauri commands → Rust traduce a JSON-RPC → Python procesa y responde.

#### Criterios de aceptación
- [ ] Protocolo JSON-RPC definido entre Rust y Python
- [ ] Tauri commands definidos para: get_status, process_message, get_tasks, get_playbooks, get_settings, update_settings
- [ ] React hooks para llamar a cada Tauri command con tipos TypeScript
- [ ] Manejo de errores: si Python no responde en 10s → timeout
- [ ] Manejo de eventos async: Python puede enviar notificaciones a React (task_completed, error, typing)
- [ ] Tests del bridge con mock de Python process

---

## SPRINT 8 — ONBOARDING (Semana 8)

**Objetivo:** Setup wizard completo y system tray funcional.

### TICKET: AOS-022
**TITLE:** Setup Wizard — Onboarding de 5 pasos
**SPRINT:** 8
**PRIORITY:** Crítica
**ASSIGNED TO:** UX/UI → Frontend Dev

#### Descripción
Implementar el wizard de configuración inicial que guía al usuario en 5 pasos. Se muestra solo la primera vez (o si el usuario resetea).

#### Pasos del wizard
1. **Welcome** — Logo, tagline, botón "Get Started"
2. **AI Provider** — Opción A: Plan Managed (default). Opción B: BYOK (pegar API keys). Validación de keys en tiempo real.
3. **Messaging** — Conectar Telegram (pegar bot token). Opcional: WhatsApp. Test de conexión.
4. **Permissions** — Checkboxes: Screen Control, CLI Execution, File System Access, Network. Explicación de cada uno.
5. **Finish** — Barra de progreso de inicialización. Mensaje de éxito. Botón "Open Dashboard".

#### Criterios de aceptación
- [ ] 5 pasos con navegación forward/back
- [ ] Validación de API keys (llama al Gateway health_check via IPC)
- [ ] Validación de Telegram token (llama al bot via IPC)
- [ ] Estado persistido: si el usuario cierra a mitad, retoma donde quedó
- [ ] Settings se guardan en el vault encriptado
- [ ] Responsive: funciona en ventana de 800x600 mínimo
- [ ] Se muestra solo si es primera ejecución

### TICKET: AOS-023
**TITLE:** System Tray — Ícono, menú, notificaciones
**SPRINT:** 8
**PRIORITY:** Alta
**ASSIGNED TO:** Frontend Dev + DevOps

#### Criterios de aceptación
- [ ] Ícono en system tray que refleja estado: idle (gris), working (violeta), error (rojo)
- [ ] Click izquierdo: abre/focaliza ventana del dashboard
- [ ] Click derecho: menú con Start/Pause Agent, Open Dashboard, Recent Tasks, Settings, Quit
- [ ] Notificaciones del sistema (toasts) para: tarea completada, error, mensaje requiere input
- [ ] El agente sigue corriendo si se cierra la ventana (solo tray)
- [ ] "Quit" cierra todo (ventana + tray + proceso Python)

---

## SPRINT 9 — DASHBOARD (Semana 9)

**Objetivo:** Las 4 secciones del dashboard funcionales.

### TICKET: AOS-024
**TITLE:** Dashboard Home — Tareas activas, stats, quick actions
**SPRINT:** 9
**PRIORITY:** Crítica
**ASSIGNED TO:** UX/UI → Frontend Dev

#### Criterios de aceptación
- [ ] Lista de tareas activas (status, input truncado, modelo, tiempo)
- [ ] Últimas 10 tareas completadas
- [ ] Stats: tareas hoy, tokens usados, costo estimado de la sesión
- [ ] Quick action: campo de texto para enviar mensaje al agente
- [ ] Auto-refresh cada 5 segundos
- [ ] Estado vacío: mensaje helpful si no hay tareas

### TICKET: AOS-025
**TITLE:** Dashboard Playbooks — Browse, install, manage
**SPRINT:** 9
**PRIORITY:** Alta
**ASSIGNED TO:** Frontend Dev

#### Criterios de aceptación
- [ ] Lista de playbooks instalados (nombre, descripción, tier, permisos)
- [ ] Detalle de playbook: instrucciones, config, steps/ si existen
- [ ] Botón "Set Active" para activar un playbook
- [ ] Botón "Record Steps" que inicia el Step Recorder (Phase 2)
- [ ] Placeholder para marketplace (Phase 5): "Coming Soon"
- [ ] Carpeta de playbooks configurable en Settings

### TICKET: AOS-026
**TITLE:** Dashboard Chat — Conversación integrada con el agente
**SPRINT:** 9
**PRIORITY:** Alta
**ASSIGNED TO:** Frontend Dev

#### Criterios de aceptación
- [ ] Interfaz de chat tipo messaging (bubbles, timestamps)
- [ ] Enviar mensaje → procesar via IPC → mostrar respuesta
- [ ] Indicador de typing mientras el agente procesa
- [ ] Scroll automático al último mensaje
- [ ] Code blocks con syntax highlighting para respuestas con código
- [ ] Botón de copiar en code blocks
- [ ] Historial de la sesión actual (no persistido entre reinicios en v1)

### TICKET: AOS-027
**TITLE:** Dashboard Settings — API keys, permisos, configuración
**SPRINT:** 9
**PRIORITY:** Alta
**ASSIGNED TO:** Frontend Dev + CISO

#### Criterios de aceptación
- [ ] Sección AI Providers: ver/editar API keys (redactadas en UI), test de conexión
- [ ] Sección Messaging: Telegram token, estado de conexión
- [ ] Sección Permissions: toggles para screen/cli/files/network
- [ ] Sección Agent: active playbook, default tier, max cost per task, log level
- [ ] Sección About: versión, links, reset wizard
- [ ] Cambios se guardan inmediatamente via IPC
- [ ] API keys NUNCA se muestran completas en la UI (redactadas como ***...xyz)

---

## SPRINT 10 — PACKAGING (Semana 10)

**Objetivo:** Instalador funcional descargable.

### TICKET: AOS-028
**TITLE:** Python Bundling — Empaquetar Python dentro de Tauri
**SPRINT:** 10
**PRIORITY:** Crítica
**ASSIGNED TO:** DevOps

#### Descripción
Empaquetar el runtime de Python y todas las dependencias del agente dentro del bundle de Tauri, para que el usuario no necesite instalar Python.

#### Criterios de aceptación
- [ ] Python 3.11+ embebido dentro del bundle (no system Python)
- [ ] Todas las dependencias pip pre-instaladas en un virtualenv incluido
- [ ] El proceso Python se lanza desde Tauri usando el Python embebido
- [ ] Tamaño total del bundle Python < 100 MB (comprimido < 40 MB)
- [ ] Funciona en Windows (Linux en v2)

### TICKET: AOS-029
**TITLE:** Auto-Update — Mecanismo de actualización silenciosa
**SPRINT:** 10
**PRIORITY:** Alta
**ASSIGNED TO:** DevOps

#### Criterios de aceptación
- [ ] Check de actualizaciones al iniciar (configurable)
- [ ] Descarga en background (no bloquea al usuario)
- [ ] Notificación: "Actualización disponible. ¿Instalar ahora o después?"
- [ ] Instalación silenciosa al reiniciar la app
- [ ] Rollback si la actualización falla
- [ ] Servidor de updates configurable (URL en config)

### TICKET: AOS-030
**TITLE:** Build & Package — Instalador .msi y firma
**SPRINT:** 10
**PRIORITY:** Crítica
**ASSIGNED TO:** DevOps

#### Criterios de aceptación
- [ ] `cargo tauri build` genera .msi funcional
- [ ] Instalador < 50 MB
- [ ] Instalación silenciosa: doble-click → progress bar → listo
- [ ] Desinstalación limpia (Add/Remove Programs)
- [ ] Icono de escritorio y acceso directo en Start Menu
- [ ] Build reproducible desde un script/CI

### TICKET: AOS-031
**TITLE:** Integración E2E Phase 3 y demo de instalación
**SPRINT:** 10
**PRIORITY:** Crítica
**ASSIGNED TO:** QA

#### Criterios de aceptación
- [ ] **Demo:** Instalar .msi en Windows limpio → setup wizard → enviar mensaje desde chat → agente responde
- [ ] **Setup wizard:** Los 5 pasos funcionan end-to-end
- [ ] **System tray:** Ícono, menú, notificaciones funcionan
- [ ] **Dashboard:** Las 4 secciones muestran datos reales del agente
- [ ] **Chat:** Enviar mensaje → recibir respuesta del agente
- [ ] **Settings:** Cambiar API key → Gateway usa la nueva key
- [ ] **Auto-update:** Check funciona (aunque no haya update disponible)
- [ ] Todos los tests de Phase 1 y Phase 2 siguen pasando

---

## Riesgos

| Riesgo | Probabilidad | Impacto | Mitigación |
|--------|-------------|---------|------------|
| Bundling de Python es demasiado grande | Media | Alto | Usar PyInstaller/Nuitka para compilar, o Python embeddable minimal |
| IPC Rust ↔ Python es inestable | Media | Crítico | Usar JSON-RPC sobre stdio (simple, probado). Fallback: HTTP local. |
| Tauri en Windows tiene bugs | Baja | Alto | Tauri 1.x es estable en Windows. Testar en múltiples versiones. |
| El wizard es confuso para no-técnicos | Media | Alto | User testing con 3 personas no-técnicas antes de cerrar. |
| Auto-update rompe la instalación | Media | Crítico | Rollback automático. Backup antes de update. |

---

## Criterios de éxito de Phase 3

| Métrica | Target |
|---------|--------|
| Tamaño del instalador .msi | < 50 MB |
| Tiempo de instalación | < 60 segundos |
| Setup wizard completion rate | > 90% (de los que empiezan, terminan) |
| Cold start (click icono → dashboard visible) | < 3 segundos |
| Memoria base (Tauri + Python idle) | < 200 MB |
| Chat response time (mensaje → respuesta visible) | < 2s + LLM latency |
