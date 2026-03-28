# SPRINT PLAN — PHASE 9: LA INTELIGENCIA

**Proyecto:** AgentOS
**Fase:** 9 — The Intelligence (Semanas 31–34)
**Sprints:** 4 (1 por semana)
**Preparado por:** Project Manager
**Fecha:** Marzo 2026
**Estado:** PENDIENTE APROBACIÓN DEL PRODUCT OWNER

---

## Objetivo de la fase

Hacer que AgentOS sea **proactivo e inteligente** — no solo espera instrucciones, sino que aprende de los patrones del usuario, sugiere tareas, anticipa necesidades, y mejora con el tiempo. Incluye analytics avanzados para que el usuario entienda cómo trabaja su agente.

---

## Entregable final de la fase

El agente nota que todos los lunes a las 9am el usuario le pide "check system status" y ofrece hacerlo automáticamente. El dashboard muestra analytics detallados: tareas por tipo, costo por modelo, tiempo ahorrado, tasa de éxito por specialist. El routing table se auto-optimiza basado en el historial de éxito/fallo.

---

## Resumen de tickets

| Ticket | Título | Sprint | Prioridad | Asignado a | Depende de |
|--------|--------|--------|-----------|------------|------------|
| AOS-080 | Analytics Engine — Métricas avanzadas y reportes | S31 | Alta | Backend Dev | Phase 8 completa |
| AOS-081 | Analytics Dashboard — Visualización de datos de uso | S31 | Alta | Frontend Dev | AOS-080 |
| AOS-082 | Proactive Suggestions — El agente sugiere tareas | S32 | Crítica | ML/AI Engineer → Backend Dev | AOS-080 |
| AOS-083 | Scheduled Tasks — Cron/triggers automáticos | S32 | Alta | Backend Dev | AOS-082 |
| AOS-084 | Routing Optimizer — Auto-mejora de la routing table | S33 | Alta | ML/AI Engineer | AOS-080 |
| AOS-085 | Learning from Corrections — El usuario corrige, el agente aprende | S33 | Alta | ML/AI Engineer | AOS-084 |
| AOS-086 | Smart Notifications — Notificaciones contextuales inteligentes | S34 | Media | Backend Dev + Frontend Dev | AOS-082 |
| AOS-087 | Usage Insights — Reportes semanales automáticos | S34 | Media | Backend Dev + Tech Writer | AOS-080 |
| AOS-088 | Integración E2E Phase 9 | S34 | Crítica | QA | Todo |

---

## SPRINT 31 — ANALYTICS (Semana 31)

### TICKET: AOS-080
**TITLE:** Analytics Engine — Métricas avanzadas y reportes
**SPRINT:** 31
**PRIORITY:** Alta

#### Descripción
Motor de analytics que procesa el historial de TaskStore y produce métricas agregadas. Base para todo lo que viene en esta phase.

#### Métricas a computar

**Por período (día/semana/mes):**
- Total de tareas, completadas, fallidas, canceladas
- Tasa de éxito global y por tipo de tarea
- Tokens consumidos y costo total (por provider, por modelo)
- Latencia promedio (total y por modelo)
- Distribución de tipos de tarea (pie chart data)
- Top 5 playbooks más usados
- Top 5 specialists más usados
- Tiempo estimado ahorrado (basado en complexity × estimated_manual_minutes)

**Por specialist/playbook:**
- Tasa de éxito
- Costo promedio por tarea
- Latencia promedio
- Número de usos

#### Criterios de aceptación
- [ ] `AnalyticsEngine.compute(period) → AnalyticsReport`
- [ ] Períodos: "today", "this_week", "this_month", "last_30_days", custom range
- [ ] Resultados cacheados (recalcula cada 5 min, no en cada request)
- [ ] API endpoint: `GET /api/v1/analytics?period=this_week`
- [ ] Export: JSON y CSV
- [ ] Tests con datos de prueba insertados en TaskStore

### TICKET: AOS-081
**TITLE:** Analytics Dashboard — Visualización de datos de uso
**SPRINT:** 31
**PRIORITY:** Alta

#### Criterios de aceptación
- [ ] Nueva sección en dashboard: "Analytics" (6to item en sidebar)
- [ ] Period selector: Today / This Week / This Month / Custom
- [ ] Cards de KPI: tareas totales, tasa de éxito, costo total, tiempo ahorrado
- [ ] Line chart: tareas por día (últimos 30 días)
- [ ] Pie chart: distribución por tipo de tarea
- [ ] Bar chart: costo por provider
- [ ] Tabla: top playbooks y specialists con métricas
- [ ] Export button: descargar reporte como CSV
- [ ] Usar Recharts o Chart.js para gráficos

---

## SPRINT 32 — PROACTIVIDAD (Semana 32)

### TICKET: AOS-082
**TITLE:** Proactive Suggestions — El agente sugiere tareas
**SPRINT:** 32
**PRIORITY:** Crítica

#### Descripción
El agente analiza patrones de uso y sugiere acciones proactivas. El usuario puede aceptar o ignorar las sugerencias.

#### Patrones a detectar

| Patrón | Ejemplo | Sugerencia |
|--------|---------|-----------|
| **Tarea recurrente** | "check disk" todos los lunes 9am | "I noticed you check disk every Monday. Want me to do this automatically?" |
| **Secuencia frecuente** | Siempre hace A, luego B, luego C | "You often do A→B→C together. Want me to create a playbook for this?" |
| **Tarea pendiente** | Empezó algo ayer, no terminó | "You started X yesterday but didn't finish. Want to continue?" |
| **Mantenimiento** | No ha checkeado updates en 7 días | "It's been a week since your last system check. Want me to run one?" |
| **Optimización** | Usa Tier 3 para tareas simples | "You could save $X/month by using a cheaper model for simple tasks." |

#### Criterios de aceptación
- [ ] `ProactiveEngine` analiza TaskStore y genera sugerencias
- [ ] Máximo 3 sugerencias activas a la vez (no spamear)
- [ ] UI: banner suave en Home con sugerencias dismissible
- [ ] El usuario puede: aceptar (ejecutar), snooze (recordar mañana), dismiss (nunca más)
- [ ] Frecuencia: analiza cada hora, no en real-time
- [ ] Privacidad: todo es local, no se envía análisis de uso a ningún servidor
- [ ] Tests con datos sintéticos que simulan patrones

### TICKET: AOS-083
**TITLE:** Scheduled Tasks — Cron/triggers automáticos
**SPRINT:** 32
**PRIORITY:** Alta

#### Descripción
Permitir al usuario programar tareas recurrentes. Extiende el Context Folder Protocol con `triggers.yaml` (mencionado en la spec como "v2").

#### Format triggers.yaml
```yaml
triggers:
  - type: cron
    schedule: "0 9 * * MON"        # Lunes a las 9am
    task: "Run system health check"
    playbook: "system_monitor"

  - type: file_watch
    path: "~/Downloads/"
    event: "created"                # Archivo nuevo en Downloads
    task: "Organize this file: {filename}"
    playbook: "file_organizer"

  - type: webhook
    path: "/triggers/deploy"        # POST a este path
    task: "Run deployment checklist"
    playbook: "deploy_checker"
```

#### Criterios de aceptación
- [ ] Parser de triggers.yaml (extensión del CFP)
- [ ] Scheduler basado en cron (usar `croniter` o `apscheduler`)
- [ ] File watcher (usar `watchdog`)
- [ ] Webhook triggers (endpoints en la API)
- [ ] Dashboard: ver/editar/enable/disable triggers por playbook
- [ ] Log de ejecuciones de triggers
- [ ] Tests de cada tipo de trigger

---

## SPRINT 33 — AUTO-MEJORA (Semana 33)

### TICKET: AOS-084
**TITLE:** Routing Optimizer — Auto-mejora de la routing table
**SPRINT:** 33
**PRIORITY:** Alta

#### Descripción
El routing table actual es estático (YAML). El Routing Optimizer analiza el historial de éxito/fallo/costo/latencia y ajusta la tabla automáticamente.

#### Lógica
```
Para cada (task_type, tier):
    1. Obtener historial de los últimos 100 tasks de este tipo/tier
    2. Para cada modelo usado:
       - success_rate = successful / total
       - avg_cost = sum(costs) / total
       - avg_latency = sum(latencies) / total
       - score = success_rate * 0.5 + (1/avg_cost) * 0.3 + (1/avg_latency) * 0.2
    3. Ordenar modelos por score descendente
    4. Actualizar routing table con el nuevo orden
```

#### Criterios de aceptación
- [ ] `RoutingOptimizer.optimize() → dict` (nueva routing table)
- [ ] Se ejecuta cada 24 horas (o bajo demanda)
- [ ] Mínimo 20 tasks por combinación para optimizar (no actúa con pocos datos)
- [ ] La tabla optimizada se guarda como `config/routing_optimized.yaml`
- [ ] El router carga la optimizada si existe, sino la default
- [ ] Dashboard: sección en Analytics que muestra "routing changes" con justificación
- [ ] Rollback: si la tabla optimizada da peores resultados, volver a la default
- [ ] Tests con datos sintéticos

### TICKET: AOS-085
**TITLE:** Learning from Corrections — El usuario corrige, el agente aprende
**SPRINT:** 33
**PRIORITY:** Alta

#### Descripción
Cuando el usuario corrige al agente (ej: "no, use the Senior agent for this" o thumbs down en una respuesta), esa corrección se registra y ajusta el comportamiento futuro.

#### Criterios de aceptación
- [ ] UI: botón 👍/👎 en cada respuesta del Chat
- [ ] Si 👎 → prompt: "What went wrong?" con opciones: wrong model, too slow, wrong answer, other
- [ ] Feedback almacenado en tabla `task_feedback` en SQLite
- [ ] El clasificador usa feedback para ajustar thresholds de complejidad
- [ ] El routing optimizer incorpora feedback en su scoring
- [ ] Privacy: feedback es local, nunca se envía a servidores
- [ ] Tests del flujo de feedback y su efecto en clasificación/routing

---

## SPRINT 34 — NOTIFICACIONES Y INSIGHTS (Semana 34)

### TICKET: AOS-086
**TITLE:** Smart Notifications — Notificaciones contextuales inteligentes
**SPRINT:** 34
**PRIORITY:** Media

#### Criterios de aceptación
- [ ] Notifications center en el dashboard (icono campana con badge)
- [ ] Tipos: sugerencia proactiva, trigger ejecutado, tarea larga completada, error recurrente, update disponible
- [ ] Agrupación: si hay 5 triggers ejecutados, mostrar como 1 notificación agrupada
- [ ] Settings: el usuario elige qué tipos de notificación recibir
- [ ] System notifications (toast del OS) para eventos críticos
- [ ] Mark as read / dismiss all

### TICKET: AOS-087
**TITLE:** Usage Insights — Reportes semanales automáticos
**SPRINT:** 34
**PRIORITY:** Media

#### Descripción
Cada lunes, el agente genera un resumen semanal automático y lo envía al canal de mensajería preferido del usuario.

#### Criterios de aceptación
- [ ] Reporte semanal generado automáticamente (lunes 8am)
- [ ] Contenido: tareas completadas, tasa de éxito, costo total, top playbooks, sugerencias de optimización
- [ ] Enviado por: Telegram/WhatsApp/Discord (el canal preferido del usuario) + visible en dashboard
- [ ] Formato: Markdown limpio, conciso (< 500 palabras)
- [ ] Configurable: enable/disable, día/hora, canal
- [ ] Tests con datos de una semana sintética

### TICKET: AOS-088
**TITLE:** Integración E2E Phase 9
**SPRINT:** 34
**PRIORITY:** Crítica

#### Criterios de aceptación
- [ ] Analytics dashboard muestra datos reales con gráficos
- [ ] Proactive suggestion aparece después de simular patrón recurrente
- [ ] Scheduled task se ejecuta según cron
- [ ] File watcher trigger se dispara cuando se crea un archivo
- [ ] Routing optimizer mejora la tabla con datos históricos
- [ ] Feedback 👍/👎 se registra y afecta el routing
- [ ] Weekly insight se genera y se envía
- [ ] Todos los tests Phase 1-8 siguen pasando
