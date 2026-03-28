# SPRINT PLAN — PHASE 8: LA API

**Proyecto:** AgentOS
**Fase:** 8 — The API (Semanas 27–30)
**Sprints:** 4 (1 por semana)
**Preparado por:** Project Manager
**Fecha:** Marzo 2026
**Estado:** PENDIENTE APROBACIÓN DEL PRODUCT OWNER

---

## Objetivo de la fase

Abrir AgentOS al mundo de desarrolladores con una **API REST pública**, un **SDK oficial**, **webhooks** para integración con servicios externos, y **documentación de calidad producción**. Esto transforma AgentOS de "aplicación de escritorio" a "plataforma programable" — terceros pueden construir sobre AgentOS.

---

## Entregable final de la fase

Un desarrollador externo puede: registrarse en developer.agentos.com, obtener un API key, enviar tareas a su instancia de AgentOS vía REST, recibir resultados por webhook, integrar con su sistema (Zapier, Make, n8n, custom code), y usar el SDK de Python para scripting. La documentación es comparable a la de Stripe o Twilio.

---

## Resumen de tickets

| Ticket | Título | Sprint | Prioridad | Asignado a | Depende de |
|--------|--------|--------|-----------|------------|------------|
| AOS-071 | Public REST API — Endpoints para tasks, playbooks, status | S27 | Crítica | API Designer → Backend Dev | Phase 7 completa |
| AOS-072 | API Authentication — API keys, rate limiting, scopes | S27 | Crítica | CISO → Backend Dev | AOS-071 |
| AOS-073 | Webhooks — Notificaciones push a URLs externas | S28 | Alta | Backend Dev | AOS-071 |
| AOS-074 | Python SDK — `pip install agentos-sdk` | S28 | Alta | Backend Dev + Tech Writer | AOS-071 |
| AOS-075 | API Documentation — OpenAPI spec, docs site, examples | S29 | Crítica | API Designer + Tech Writer | AOS-071, AOS-074 |
| AOS-076 | Integration Templates — Zapier, Make, n8n, GitHub Actions | S29 | Alta | Backend Dev | AOS-073 |
| AOS-077 | Developer Portal — Registro, API keys, usage dashboard | S30 | Alta | Frontend Dev | AOS-072 |
| AOS-078 | CLI Tool — `agentos` command line para power users | S30 | Media | Backend Dev | AOS-074 |
| AOS-079 | Integración E2E Phase 8 | S30 | Crítica | QA | Todo |

---

## Diagrama de dependencias

```
Phase 7 completa
    │
    ├── AOS-071 (REST API) ──┬── AOS-072 (Auth + Rate Limit)
    │                        ├── AOS-073 (Webhooks)
    │                        ├── AOS-074 (Python SDK)
    │                        │       └── AOS-078 (CLI Tool)
    │                        ├── AOS-075 (Documentation)
    │                        └── AOS-076 (Integration Templates)
    │
    ├── AOS-077 (Developer Portal)
    └── AOS-079 (E2E Phase 8)
```

---

## SPRINT 27 — API CORE (Semana 27)

### TICKET: AOS-071
**TITLE:** Public REST API — Endpoints para tasks, playbooks, status
**SPRINT:** 27
**PRIORITY:** Crítica

#### Descripción
Exponer las capacidades de AgentOS via REST API. Esto extiende la FastAPI del marketplace (Phase 5) con endpoints para operación del agente.

#### Endpoints

```
# Tasks
POST   /api/v1/tasks                    — Crear y ejecutar una tarea
GET    /api/v1/tasks                    — Listar tareas recientes
GET    /api/v1/tasks/{id}               — Detalle de una tarea
GET    /api/v1/tasks/{id}/chain         — Sub-tareas si es cadena
DELETE /api/v1/tasks/{id}               — Cancelar tarea en progreso

# Playbooks
GET    /api/v1/playbooks                — Listar playbooks instalados
POST   /api/v1/playbooks/activate       — Activar un playbook
GET    /api/v1/playbooks/{name}         — Detalle de un playbook

# Agent
GET    /api/v1/status                   — Estado del agente
GET    /api/v1/health                   — Health check de providers
GET    /api/v1/usage                    — Resumen de uso y costos

# Mesh
GET    /api/v1/mesh/nodes               — Nodos en la mesh
GET    /api/v1/mesh/nodes/{id}          — Detalle de un nodo
```

#### Criterios de aceptación
- [ ] Todos los endpoints implementados con FastAPI
- [ ] Respuestas en JSON con schema consistente: `{data, error, meta}`
- [ ] Paginación: `?page=1&per_page=20`
- [ ] Filtros y sorting en list endpoints
- [ ] Versionado: `/api/v1/` (preparado para v2)
- [ ] OpenAPI schema auto-generado por FastAPI
- [ ] POST /tasks es async: retorna task_id inmediatamente, resultado vía polling o webhook
- [ ] Tests de cada endpoint

### TICKET: AOS-072
**TITLE:** API Authentication — API keys, rate limiting, scopes
**SPRINT:** 27
**PRIORITY:** Crítica

#### Criterios de aceptación
- [ ] API keys generadas por el usuario en el Developer Portal
- [ ] Auth via header: `Authorization: Bearer aos_key_xxx`
- [ ] Scopes: `tasks:read`, `tasks:write`, `playbooks:read`, `playbooks:write`, `admin`
- [ ] Rate limiting: 100 req/min para free, 1000/min para Pro, configurable para Enterprise
- [ ] Rate limit headers: `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`
- [ ] API keys almacenadas como hash (bcrypt) en DB — nunca en plaintext
- [ ] Logging de cada request (sin body completo — solo method, path, status, latency)
- [ ] Tests de auth, rate limiting, y scopes

---

## SPRINT 28 — WEBHOOKS Y SDK (Semana 28)

### TICKET: AOS-073
**TITLE:** Webhooks — Notificaciones push a URLs externas
**SPRINT:** 28
**PRIORITY:** Alta

#### Descripción
Cuando una tarea completa (o falla), AgentOS puede enviar un HTTP POST a una URL configurada por el usuario. Esto permite integraciones real-time sin polling.

#### Criterios de aceptación
- [ ] Configurar webhook URLs en Settings: `POST /api/v1/webhooks` con url + events
- [ ] Events soportados: `task.completed`, `task.failed`, `task.started`, `chain.completed`
- [ ] Payload: JSON con task result completo + metadata
- [ ] Firma HMAC-SHA256 en header `X-AgentOS-Signature` para verificación
- [ ] Retry: 3 intentos con backoff exponencial si el endpoint no responde
- [ ] Webhook logs: historial de deliveries con status (éxito/fallo)
- [ ] Tests con endpoint mock

### TICKET: AOS-074
**TITLE:** Python SDK — `pip install agentos-sdk`
**SPRINT:** 28
**PRIORITY:** Alta

#### Descripción
SDK oficial de Python que wrappea la REST API. Permite a developers integrar AgentOS en sus scripts y aplicaciones.

#### Criterios de aceptación
- [ ] Package publicable: `agentos-sdk` (PyPI-ready)
- [ ] Client class: `AgentOS(api_key="...", base_url="...")`
- [ ] Métodos: `run_task()`, `get_task()`, `list_tasks()`, `get_status()`, etc.
- [ ] Async support: `AsyncAgentOS` con `await run_task()`
- [ ] Streaming: `for chunk in client.run_task_stream(...)` para respuestas en tiempo real
- [ ] Error handling: excepciones tipadas (AuthError, RateLimitError, TaskError)
- [ ] Type hints completos (compatible con mypy strict)
- [ ] README con quickstart y ejemplos
- [ ] Tests del SDK contra API mock

```python
# Ejemplo de uso del SDK
from agentos_sdk import AgentOS

agent = AgentOS(api_key="aos_key_xxx")

# Sync
result = agent.run_task("Check disk space on my server")
print(result.output)

# Async
async with AsyncAgentOS(api_key="aos_key_xxx") as agent:
    result = await agent.run_task("Analyze this CSV", attachments=["data.csv"])
    print(result.output)
```

---

## SPRINT 29 — DOCUMENTACIÓN E INTEGRACIONES (Semana 29)

### TICKET: AOS-075
**TITLE:** API Documentation — OpenAPI spec, docs site, examples
**SPRINT:** 29
**PRIORITY:** Crítica

#### Criterios de aceptación
- [ ] OpenAPI 3.1 spec generada automáticamente + enriched con descripciones
- [ ] Docs site interactivo (Swagger UI o Redoc) servido en `/docs`
- [ ] Getting Started guide: registro → API key → primera tarea → webhook
- [ ] Guías por caso de uso: automate emails, CI/CD integration, data pipeline, monitoring
- [ ] Referencia completa de cada endpoint con request/response examples
- [ ] SDK quickstart con 5 ejemplos progresivos
- [ ] Error reference: cada error code con causa y solución
- [ ] Rate limit guide: límites por plan, best practices

### TICKET: AOS-076
**TITLE:** Integration Templates — Zapier, Make, n8n, GitHub Actions
**SPRINT:** 29
**PRIORITY:** Alta

#### Criterios de aceptación
- [ ] **Zapier:** Template de Zap: "When email received → Send to AgentOS → Forward result"
- [ ] **Make (Integromat):** Template de scenario con HTTP module
- [ ] **n8n:** Workflow template con HTTP Request node
- [ ] **GitHub Actions:** Action que envía tarea a AgentOS (ej: "review this PR")
- [ ] **cURL examples:** 10 ejemplos de cURL para copiar y pegar
- [ ] Cada template documentado con screenshots y step-by-step
- [ ] Templates como archivos descargables (.json para Zapier/Make/n8n, .yml para GH Actions)

---

## SPRINT 30 — PORTAL Y CLI (Semana 30)

### TICKET: AOS-077
**TITLE:** Developer Portal — Registro, API keys, usage dashboard
**SPRINT:** 30
**PRIORITY:** Alta

#### Criterios de aceptación
- [ ] Página web (o sección en dashboard): developer.agentos.com o Dashboard > Developer
- [ ] Registro: crear cuenta de developer (email + password o SSO)
- [ ] API Keys: generar, revocar, ver últimas 4 chars, asignar scopes
- [ ] Usage dashboard: requests hoy, esta semana, este mes. Gráfico de uso por día.
- [ ] Webhook management: agregar/editar/borrar webhooks, ver delivery logs
- [ ] Plan y billing info: cuántas requests quedan, upgrade link
- [ ] Quickstart guide inline (no sale del portal)

### TICKET: AOS-078
**TITLE:** CLI Tool — `agentos` command line para power users
**SPRINT:** 30
**PRIORITY:** Media

#### Descripción
Herramienta CLI que usa el SDK para interactuar con AgentOS desde la terminal. Para power users y scripting.

#### Criterios de aceptación
- [ ] `agentos run "check disk space"` — ejecutar tarea
- [ ] `agentos status` — estado del agente
- [ ] `agentos tasks` — listar tareas recientes
- [ ] `agentos playbooks` — listar playbooks
- [ ] `agentos pack <dir>` — empaquetar playbook (ya existe de Phase 5, unificar)
- [ ] `agentos install <file.aosp>` — instalar playbook
- [ ] `agentos config` — ver/editar configuración
- [ ] `agentos mesh` — ver nodos de la mesh
- [ ] Output formateado con rich (tablas, colores)
- [ ] Configurable: `~/.agentos/config.yaml` con api_key y base_url
- [ ] Autocompletado de shell (bash/zsh/fish)
- [ ] Publicable como `pip install agentos-cli`

### TICKET: AOS-079
**TITLE:** Integración E2E Phase 8
**SPRINT:** 30
**PRIORITY:** Crítica

#### Criterios de aceptación
- [ ] SDK: `agent.run_task("hello")` retorna resultado
- [ ] API: POST /api/v1/tasks funciona end-to-end
- [ ] Webhook: tarea completa → webhook delivered a mock endpoint
- [ ] Rate limiting: 101st request retorna 429
- [ ] Auth: request sin API key retorna 401
- [ ] CLI: `agentos run "hello"` retorna resultado
- [ ] Docs: /docs muestra Swagger UI funcional
- [ ] Integration template de GitHub Actions funciona (con mock)
- [ ] Todos los tests Phase 1-7 siguen pasando
