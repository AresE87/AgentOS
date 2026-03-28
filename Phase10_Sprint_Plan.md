# SPRINT PLAN — PHASE 10: EL MÓVIL

**Proyecto:** AgentOS
**Fase:** 10 — The Mobile (Semanas 35–38)
**Sprints:** 4 (1 por semana)
**Preparado por:** Project Manager
**Fecha:** Marzo 2026
**Estado:** PENDIENTE APROBACIÓN DEL PRODUCT OWNER

---

## Objetivo de la fase

Lanzar una **app móvil companion** (iOS + Android) que permite al usuario controlar su agente de escritorio desde el teléfono. No es un agente independiente — es un control remoto elegante para el AgentOS que corre en su PC.

---

## Entregable final de la fase

El usuario abre la app en su teléfono, ve el estado de su AgentOS (online, procesando, idle), envía una tarea desde el chat móvil, ve el resultado, revisa analytics, recibe push notifications cuando una tarea completa, y puede gestionar playbooks y settings remotamente. Todo via la API pública (Phase 8).

---

## Decisión tecnológica: React Native

| Aspecto | React Native (elección) | Flutter (alternativa) | Native (alternativa) |
|---------|------------------------|----------------------|---------------------|
| Code sharing | 90%+ entre iOS y Android | 95%+ | 0% |
| Reutilización | Comparte lógica con dashboard web (React) | Dart — nuevo lenguaje | Swift + Kotlin |
| Ecosystem | Enorme, maduro | Grande, creciendo | Plataforma nativa |
| Performance | Bueno para esta app (no es gaming) | Excelente | Excelente |
| Time to market | 4 semanas | 4 semanas | 8 semanas |

React Native gana por la reutilización de conocimiento con el frontend web (React + TypeScript).

---

## Resumen de tickets

| Ticket | Título | Sprint | Prioridad | Asignado a | Depende de |
|--------|--------|--------|-----------|------------|------------|
| AOS-089 | Mobile Scaffold — React Native project + navigation | S35 | Crítica | Frontend Dev | Phase 8 (API) |
| AOS-090 | Mobile Auth — Login y conexión con AgentOS desktop | S35 | Crítica | Frontend Dev + CISO | AOS-089 |
| AOS-091 | Mobile Chat — Enviar tareas y ver respuestas | S36 | Crítica | Frontend Dev | AOS-090 |
| AOS-092 | Mobile Dashboard — Status, tasks, analytics | S36 | Alta | Frontend Dev | AOS-090 |
| AOS-093 | Push Notifications — Alertas cuando tareas completan | S37 | Alta | Backend Dev + DevOps | AOS-090 |
| AOS-094 | Mobile Playbooks — Browse marketplace e install desde el teléfono | S37 | Alta | Frontend Dev | AOS-090 |
| AOS-095 | Mobile Settings — Config remota del agente | S37 | Media | Frontend Dev | AOS-090 |
| AOS-096 | Offline Sync — Cola de tareas cuando el teléfono está offline | S38 | Media | Frontend Dev | AOS-091 |
| AOS-097 | App Store Submission — Build, firma, publicación | S38 | Crítica | DevOps | Todo |
| AOS-098 | Integración E2E Phase 10 | S38 | Crítica | QA | Todo |

---

## SPRINT 35 — SCAFFOLD Y AUTH (Semana 35)

### TICKET: AOS-089
**TITLE:** Mobile Scaffold — React Native project + navigation
**SPRINT:** 35
**PRIORITY:** Crítica

#### Criterios de aceptación
- [ ] React Native project inicializado con TypeScript
- [ ] Navigation: Tab bar con 4 tabs (Chat, Tasks, Playbooks, Settings)
- [ ] Design system: mismos colores/tipografía que desktop (dark theme)
- [ ] API client: wrapper del SDK de Python adaptado para React Native (fetch-based)
- [ ] Splash screen con logo de AgentOS
- [ ] Corre en iOS Simulator y Android Emulator
- [ ] Estructura de carpetas limpia: screens/, components/, hooks/, api/, types/

### TICKET: AOS-090
**TITLE:** Mobile Auth — Login y conexión con AgentOS desktop
**SPRINT:** 35
**PRIORITY:** Crítica

#### Descripción
El usuario necesita conectar la app móvil con su instancia de AgentOS. Dos métodos: QR code (escanear desde desktop) o API key manual.

#### Criterios de aceptación
- [ ] **Método 1 (QR):** Desktop muestra QR con `{api_url, api_key_temp}` → móvil escanea → conectado
- [ ] **Método 2 (Manual):** El usuario pega la API URL + API key en la app
- [ ] Conexión persistida en secure storage del móvil (Keychain iOS / Keystore Android)
- [ ] Health check al iniciar: verifica que el desktop está online
- [ ] Si desktop offline → mensaje: "Your AgentOS is not reachable. Make sure your PC is on."
- [ ] Logout / disconnect: borra credentials del móvil
- [ ] Soporte para múltiples instancias: "Office PC", "Home PC" (switch entre ellas)
- [ ] Tests del flujo de auth

---

## SPRINT 36 — CHAT Y DASHBOARD MÓVIL (Semana 36)

### TICKET: AOS-091
**TITLE:** Mobile Chat — Enviar tareas y ver respuestas
**SPRINT:** 36
**PRIORITY:** Crítica

#### Criterios de aceptación
- [ ] UI de chat tipo iMessage/WhatsApp (bubbles, timestamps)
- [ ] Enviar mensaje → POST /api/v1/tasks → polling por resultado
- [ ] Typing indicator mientras el agente procesa
- [ ] Code blocks con syntax highlighting y botón copiar
- [ ] Imágenes inline (si el agente retorna screenshots)
- [ ] Pull-to-refresh para actualizar
- [ ] Historial de chat persistido localmente (AsyncStorage)
- [ ] Haptic feedback al enviar/recibir mensaje

### TICKET: AOS-092
**TITLE:** Mobile Dashboard — Status, tasks, analytics
**SPRINT:** 36
**PRIORITY:** Alta

#### Criterios de aceptación
- [ ] **Home screen:** Estado del agente (online/offline/busy), stats del día (tareas, costo)
- [ ] **Tasks list:** Últimas 20 tareas con estado, modelo, costo. Pull-to-refresh.
- [ ] **Task detail:** Tap en tarea → detalle completo. Si es cadena → sub-tareas.
- [ ] **Analytics mini:** Gráfico simple de tareas por día (últimos 7 días)
- [ ] Loading states y error handling en cada pantalla
- [ ] Skeleton loaders mientras carga datos

---

## SPRINT 37 — NOTIFICACIONES Y PLAYBOOKS (Semana 37)

### TICKET: AOS-093
**TITLE:** Push Notifications — Alertas cuando tareas completan
**SPRINT:** 37
**PRIORITY:** Alta

#### Descripción
Cuando una tarea completa o falla en el desktop, el usuario recibe una push notification en el teléfono.

#### Criterios de aceptación
- [ ] Push notifications via Firebase Cloud Messaging (FCM) para Android + APNs para iOS
- [ ] Backend: cuando tarea completa → enviar push al device token registrado
- [ ] Registro de device token: POST /api/v1/devices con token de FCM/APNs
- [ ] Notificaciones para: task.completed, task.failed, proactive suggestion
- [ ] Tap en notificación → abre la app en el detalle de la tarea
- [ ] Settings: enable/disable por tipo de notificación
- [ ] Badge count: número de tareas pendientes de review
- [ ] Tests con mock de FCM

### TICKET: AOS-094
**TITLE:** Mobile Playbooks — Browse marketplace e install desde el teléfono
**SPRINT:** 37
**PRIORITY:** Alta

#### Criterios de aceptación
- [ ] Browse marketplace (grid de playbooks como en desktop pero optimizado para mobile)
- [ ] Search y filtros (categoría, precio)
- [ ] Detalle de playbook con screenshots (swipeable gallery)
- [ ] Install button → llama a la API del desktop → el desktop instala
- [ ] Reviews: ver y escribir
- [ ] Mis playbooks instalados: lista con toggle activate/deactivate

### TICKET: AOS-095
**TITLE:** Mobile Settings — Config remota del agente
**SPRINT:** 37
**PRIORITY:** Media

#### Criterios de aceptación
- [ ] Ver providers configurados (redactados: ***xyz)
- [ ] Ver/cambiar: default tier, max cost, active playbook
- [ ] Ver status de messaging (Telegram, WhatsApp, Discord)
- [ ] Ver nodos de la mesh (si Phase 7 activa)
- [ ] Cambios se envían via API al desktop
- [ ] NO se pueden editar API keys desde el móvil (seguridad)

---

## SPRINT 38 — OFFLINE SYNC Y PUBLICACIÓN (Semana 38)

### TICKET: AOS-096
**TITLE:** Offline Sync — Cola de tareas cuando el teléfono está offline
**SPRINT:** 38
**PRIORITY:** Media

#### Criterios de aceptación
- [ ] Si el teléfono no tiene conexión → la tarea se encola localmente
- [ ] Cuando recupera conexión → enviar tareas encoladas en orden
- [ ] UI: indicador "offline" + badge "2 queued tasks"
- [ ] Si el desktop está offline → mostrar "Your PC is not reachable" (distinto a sin internet)
- [ ] Cache de datos: tasks y analytics cacheados localmente para viewing offline

### TICKET: AOS-097
**TITLE:** App Store Submission — Build, firma, publicación
**SPRINT:** 38
**PRIORITY:** Crítica

#### Criterios de aceptación
- [ ] **iOS:** Build con Xcode → IPA firmado → TestFlight → App Store submission
- [ ] **Android:** Build con Gradle → AAB firmado → Google Play Console → publicación
- [ ] App icons, splash screen, screenshots para store listings
- [ ] Store listing: título, descripción, keywords, categoría, privacy policy URL
- [ ] Privacy policy page: qué datos recolecta la app (respuesta: casi nada — todo es local en el desktop)
- [ ] Age rating: 4+ (no hay contenido restringido)
- [ ] Tamaño de la app < 30 MB

### TICKET: AOS-098
**TITLE:** Integración E2E Phase 10
**SPRINT:** 38
**PRIORITY:** Crítica

#### Criterios de aceptación
- [ ] QR login: desktop muestra QR → móvil escanea → conectado
- [ ] Mobile chat: enviar tarea → resultado aparece en el teléfono
- [ ] Push notification: tarea completa → push llega al teléfono
- [ ] Marketplace mobile: browse → install (desktop instala)
- [ ] Offline: enviar tarea sin conexión → se envía cuando reconecta
- [ ] La app funciona con desktop en la misma LAN y remotamente (via API pública)
- [ ] Todos los tests Phase 1-9 siguen pasando (solo backend)

---

## Riesgos

| Riesgo | Probabilidad | Impacto | Mitigación |
|--------|-------------|---------|------------|
| Apple rechaza la app (guidelines) | Media | Alto | Review pre-submission. La app no ejecuta código, solo es control remoto. |
| Latencia API desde móvil en 4G | Media | Medio | Cache agresivo. Optimistic UI. Offline queue. |
| El usuario no sabe su API URL | Alta | Medio | QR code elimina esta fricción. Relay server puede hacer matching automático. |
| Push notifications unreliable | Baja | Bajo | Polling como fallback cada 30s cuando la app está abierta. |

---

## Criterios de éxito de Phase 10

| Métrica | Target |
|---------|--------|
| App size | < 30 MB |
| Login time (QR) | < 10 seconds |
| Chat response display latency | < 500ms post API response |
| Push notification delivery | > 95% |
| Offline queue reliability | 100% (no se pierde ninguna tarea) |
| App Store approval | First submission |
| App crash rate | < 1% |
