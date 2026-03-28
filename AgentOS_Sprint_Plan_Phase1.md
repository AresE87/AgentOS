# SPRINT PLAN — PHASE 1: EL CEREBRO

**Proyecto:** AgentOS
**Fase:** 1 — The Brain (Semanas 1–3)
**Sprints:** 3 (1 por semana)
**Preparado por:** Project Manager
**Fecha:** Marzo 2026
**Estado:** PENDIENTE APROBACIÓN DEL PRODUCT OWNER

---

## Objetivo de la fase

Entregar un agente funcional que reciba mensajes por Telegram, los procese a través de un LLM Gateway inteligente, ejecute comandos vía CLI en la PC, y devuelva el resultado al usuario. Esto prueba el loop central: **mensaje entra → cerebro decide → acción se ejecuta → resultado sale.**

---

## Entregable final de la fase

Un usuario envía un mensaje de Telegram al bot de AgentOS. El agente analiza el mensaje, selecciona el modelo de IA óptimo vía el LLM Gateway, ejecuta un comando de shell en la PC, y devuelve el resultado al usuario por Telegram. El historial completo queda registrado en SQLite.

---

## Resumen de tickets

| Ticket | Título | Sprint | Prioridad | Asignado a | Depende de |
|--------|--------|--------|-----------|------------|------------|
| AOS-001 | Estructura del repositorio y scaffold del proyecto | S1 | Crítica | Software Architect, DevOps | Ninguno |
| AOS-002 | LLM Gateway — Capa de abstracción de proveedores | S1 | Crítica | Software Architect → Backend Dev | AOS-001 |
| AOS-003 | LLM Gateway — Clasificador de tareas (reglas v1) | S1 | Alta | ML/AI Engineer | AOS-001 |
| AOS-004 | CLI Executor con soporte PTY | S2 | Crítica | Software Architect → Backend Dev | AOS-001 |
| AOS-005 | Context Folder Protocol — Parser básico | S2 | Alta | API Designer → Backend Dev | AOS-001 |
| AOS-006 | SQLite Task Store — Persistencia de tareas y logging | S2 | Alta | Database Architect → Backend Dev | AOS-001 |
| AOS-007 | LLM Gateway — Rastreo de costos y medición de uso | S2 | Media | Backend Dev | AOS-002 |
| AOS-008 | Integración con bot de Telegram | S3 | Crítica | Backend Dev | AOS-002, AOS-004 |
| AOS-009 | Agent Core — Pipeline de procesamiento de tareas | S3 | Crítica | Software Architect → Backend Dev | AOS-002, AOS-003, AOS-004, AOS-005, AOS-006 |
| AOS-010 | Integración end-to-end y demo funcional | S3 | Crítica | QA, Security Auditor, Perf Engineer | AOS-008, AOS-009 |

---

## Diagrama de dependencias

```
AOS-001 (Estructura del repo)
    ├── AOS-002 (Provider Abstraction) ──┬── AOS-007 (Cost Tracker)
    │                                    ├── AOS-008 (Telegram Bot)
    │                                    └── AOS-009 (Agent Core Pipeline)
    ├── AOS-003 (Task Classifier) ───────┘
    ├── AOS-004 (CLI Executor) ──────────┤
    ├── AOS-005 (CFP Parser) ────────────┤
    └── AOS-006 (SQLite Store) ──────────┘
                                         └── AOS-010 (Integración E2E)
```

---

## SPRINT 1 — FUNDACIÓN (Semana 1)

**Objetivo:** Establecer la base del proyecto y el componente más crítico: el LLM Gateway.

---

### TICKET: AOS-001
**TITLE:** Estructura del repositorio y scaffold del proyecto
**PHASE:** 1-brain
**SPRINT:** 1
**PRIORITY:** Crítica
**ASSIGNED TO:** Software Architect (arquitectura) → DevOps Engineer (configuración)
**DEPENDS ON:** Ninguno
**BLOCKED BY:** Ninguno

#### Descripción
Crear la estructura completa del repositorio con la organización de módulos, configuración de entorno, y scaffold base para que todos los demás tickets puedan trabajar en paralelo. Esta es la base sobre la que se construye todo.

#### Criterios de aceptación
- [ ] Repositorio inicializado con estructura de directorios definida por el Architect
- [ ] Estructura de módulos Python con `__init__.py` para: `gateway/`, `executor/`, `context/`, `store/`, `messaging/`, `core/`
- [ ] `pyproject.toml` con dependencias base (litellm, python-telegram-bot, rich, pyyaml, httpx)
- [ ] Scaffold de Tauri inicializado (Cargo.toml, tauri.conf.json, src-tauri/)
- [ ] Scaffold de React + TypeScript (src/, package.json, tsconfig.json)
- [ ] Archivo `.env.example` con todas las variables de entorno necesarias
- [ ] `Makefile` o scripts con comandos: `setup`, `dev`, `test`, `lint`, `format`
- [ ] Configuración de `ruff` para linting y formatting
- [ ] README.md con instrucciones de setup para desarrollo
- [ ] El comando `make test` ejecuta sin errores (aunque no haya tests aún)

#### Inputs
- Especificación de producto (secciones 3.1, 8.1, 8.2)
- Team Protocol (estructura de fases)

#### Output esperado
- Architecture Document: estructura de directorios, módulos, y sus responsabilidades
- Repositorio scaffold funcional

#### Notas
El Architect define la estructura; el DevOps la implementa y configura el tooling. El Frontend Developer no es necesario aún — el scaffold de React/Tauri es mínimo y lo puede hacer DevOps.

---

### TICKET: AOS-002
**TITLE:** LLM Gateway — Capa de abstracción de proveedores
**PHASE:** 1-brain
**SPRINT:** 1
**PRIORITY:** Crítica
**ASSIGNED TO:** Software Architect (arquitectura) → API Designer (contratos) → CISO (seguridad) → Backend Developer (implementación)
**DEPENDS ON:** AOS-001
**BLOCKED BY:** Ninguno

#### Descripción
Implementar la capa central del LLM Gateway que normaliza todas las APIs de proveedores (Anthropic, OpenAI, Google) en una interfaz unificada usando LiteLLM como base. Este es el sistema nervioso central — toda llamada de IA pasa por aquí. Incluye gestión segura de API keys y abstracción de modelos.

#### Criterios de aceptación
- [ ] Interfaz unificada `LLMProvider` con método async `complete(prompt, model_config) -> LLMResponse`
- [ ] Soporte para al menos 3 proveedores: Anthropic (Claude), OpenAI (GPT), Google (Gemini)
- [ ] Abstracción de modelos: el llamador pide un tier (1/2/3), no un modelo específico
- [ ] Tabla de enrutamiento configurable: mapea (tipo_tarea, complejidad) → modelo
- [ ] Gestión de API keys a través del vault encriptado (nunca en texto plano, nunca en logs)
- [ ] Fallback automático: si un proveedor falla, intenta el siguiente en la tabla
- [ ] Respuesta normalizada: mismo formato de output sin importar el proveedor
- [ ] Logging de cada llamada (modelo usado, tokens consumidos, latencia, costo estimado) SIN loggear contenido sensible
- [ ] Tests unitarios con mocks para cada proveedor (no depender de API keys reales para tests)
- [ ] Pasa `ruff check` y `ruff format`

#### Inputs
- Especificación de producto (sección 3.2 — LLM Gateway)
- Tabla de enrutamiento de la spec (sección 3.2)
- Estructura de repositorio (AOS-001)

#### Output esperado
- Architecture Document (Software Architect)
- API Contract (API Designer)
- Security Requirements (CISO)
- Código fuente en `gateway/` con tests
- Config YAML con tabla de enrutamiento por defecto

#### Notas
Este es el componente más crítico de la Phase 1. La tabla de enrutamiento en v1 es estática (configurada en YAML). En v2 será dinámica basada en feedback. LiteLLM maneja mucha de la abstracción de proveedores — no reinventar la rueda, pero sí wrappearla en nuestras interfaces para no acoplarnos.

---

### TICKET: AOS-003
**TITLE:** LLM Gateway — Clasificador de tareas (reglas v1)
**PHASE:** 1-brain
**SPRINT:** 1
**PRIORITY:** Alta
**ASSIGNED TO:** ML/AI Engineer
**DEPENDS ON:** AOS-001
**BLOCKED BY:** Ninguno

#### Descripción
Implementar el clasificador de tareas basado en reglas que analiza cada mensaje/tarea entrante y determina: tipo de tarea (texto, código, visión, generación), puntuación de complejidad (1–5), y tier de presupuesto (económico/estándar/premium). En v1 esto es puramente basado en reglas (regex, keywords, heurísticas). No requiere ML ni modelos externos.

#### Criterios de aceptación
- [ ] Función async `classify(task_input: TaskInput) -> TaskClassification` que retorna tipo, complejidad, y tier
- [ ] Detección de tipo de tarea por análisis de contenido:
  - Texto/chat: mensajes generales, preguntas, conversación
  - Código: presencia de syntax, keywords técnicos, pedidos de código
  - Visión: referencias a pantalla, capturas, UI
  - Generación: pedidos de crear imágenes, documentos
  - Datos: referencias a planillas, CSV, datos tabulares
- [ ] Puntuación de complejidad (1–5) basada en: longitud del mensaje, número de sub-tareas implícitas, presencia de condicionales, referencias a múltiples herramientas
- [ ] Tier de presupuesto derivado del tipo + complejidad según tabla de enrutamiento
- [ ] Zero dependencias externas (puro Python, sin modelos, sin API calls)
- [ ] Latencia de clasificación < 10ms
- [ ] Tests unitarios con al menos 30 casos de prueba cubriendo cada tipo y nivel de complejidad
- [ ] Documentado: cada regla explicada con ejemplos
- [ ] Pasa `ruff check` y `ruff format`

#### Inputs
- Especificación de producto (sección 3.2 — Task Classifier)
- Tabla de enrutamiento (sección 3.2)
- Estructura de repositorio (AOS-001)

#### Output esperado
- Código fuente en `gateway/classifier.py` con tests
- Documento de reglas: lista de todas las reglas con ejemplos de input/output

#### Notas
Este clasificador será reemplazado por un modelo ML en el futuro (v2+). Diseñar la interfaz de forma que el clasificador sea intercambiable (strategy pattern). El ML/AI Engineer puede trabajar en paralelo con AOS-002 porque solo comparten la interfaz `TaskClassification`.

---

## SPRINT 2 — EJECUCIÓN Y PERSISTENCIA (Semana 2)

**Objetivo:** Construir las manos (CLI Executor), la memoria (SQLite Store), y el lenguaje (CFP Parser) del agente.

---

### TICKET: AOS-004
**TITLE:** CLI Executor con soporte PTY
**PHASE:** 1-brain
**SPRINT:** 2
**PRIORITY:** Crítica
**ASSIGNED TO:** Software Architect (arquitectura) → CISO (seguridad) → Backend Developer (implementación)
**DEPENDS ON:** AOS-001
**BLOCKED BY:** Ninguno

#### Descripción
Implementar el motor de ejecución CLI que permite al agente ejecutar comandos de shell en la PC del usuario. Debe soportar PTY completo para programas interactivos, capturar stdout/stderr, manejar timeouts, y tener un sandbox de seguridad que bloquee comandos peligrosos.

#### Criterios de aceptación
- [ ] Función async `execute(command: str, config: ExecutorConfig) -> ExecutionResult`
- [ ] Soporte PTY completo (para programas interactivos como `ssh`, `python REPL`)
- [ ] Captura completa de stdout y stderr por separado
- [ ] Timeout configurable con terminación graceful del proceso
- [ ] Directorio de trabajo configurable por ejecución
- [ ] Variables de entorno configurables por ejecución (sin heredar secrets del host)
- [ ] **Sandbox de seguridad:**
  - Lista de comandos bloqueados (rm -rf /, format, shutdown, etc.)
  - Validación de comandos antes de ejecución
  - Límite de tiempo máximo por defecto (5 minutos)
  - Límite de output (prevenir memory overflow por output infinito)
- [ ] Logging de cada ejecución en formato estructurado (comando, duración, exit code, truncated output)
- [ ] Comando y output NUNCA incluyen API keys o secrets del environment
- [ ] Tests unitarios con comandos seguros (echo, ls, cat, etc.)
- [ ] Pasa `ruff check` y `ruff format`

#### Inputs
- Especificación de producto (sección 3.3 — Modo 2: CLI Executor)
- Modelo de seguridad (sección 8.3)
- Estructura de repositorio (AOS-001)

#### Output esperado
- Architecture Document con modelo de sandbox (Software Architect)
- Security Requirements con lista de comandos bloqueados y threat model (CISO)
- Código fuente en `executor/cli.py` con tests

#### Notas
Componente de alta sensibilidad de seguridad. El CISO debe definir el threat model y la lista de bloqueo ANTES de que el Backend Dev implemente. El executor es el componente que más riesgo tiene si se implementa mal. Priorizar seguridad sobre funcionalidad.

---

### TICKET: AOS-005
**TITLE:** Context Folder Protocol — Parser básico
**PHASE:** 1-brain
**SPRINT:** 2
**PRIORITY:** Alta
**ASSIGNED TO:** API Designer (spec del formato) → Backend Developer (implementación)
**DEPENDS ON:** AOS-001
**BLOCKED BY:** Ninguno

#### Descripción
Implementar el parser que lee y valida Context Folders (playbooks). En v1 solo necesitamos parsear `playbook.md` (instrucciones en lenguaje natural) y `config.yaml` (configuración del agente). Los demás archivos (steps/, templates/, triggers.yaml) se agregan en fases posteriores.

#### Criterios de aceptación
- [ ] Función async `parse_context_folder(path: Path) -> ContextFolder` que retorna la estructura parseada
- [ ] Parseo de `playbook.md`: extrae título, descripción, e instrucciones paso a paso
- [ ] Parseo de `config.yaml`: extrae tier LLM, permisos requeridos, límites, timeout
- [ ] Validación: verifica que los archivos requeridos existen y tienen formato válido
- [ ] Errores descriptivos si el formato es inválido (qué archivo, qué línea, qué esperaba)
- [ ] Dataclass `ContextFolder` con toda la información parseada y tipada
- [ ] Soporte para cargar múltiples context folders (directorio de playbooks)
- [ ] Tests unitarios con al menos 5 playbooks de ejemplo (válidos e inválidos)
- [ ] Playbooks de ejemplo incluidos en `examples/playbooks/`
- [ ] Pasa `ruff check` y `ruff format`

#### Inputs
- Especificación de producto (sección 4.1 — Estructura de carpeta)
- Estructura de repositorio (AOS-001)

#### Output esperado
- API Contract: formato exacto de `playbook.md` y `config.yaml` (API Designer)
- Código fuente en `context/parser.py` con tests
- 5 playbooks de ejemplo en `examples/playbooks/`

#### Notas
El API Designer debe definir el formato exacto del playbook ANTES del build. Esto es importante porque el formato será parte de la especificación abierta (CFP). Diseñar con extensibilidad en mente — v2 agrega steps/, templates/, triggers.yaml.

---

### TICKET: AOS-006
**TITLE:** SQLite Task Store — Persistencia de tareas y logging
**PHASE:** 1-brain
**SPRINT:** 2
**PRIORITY:** Alta
**ASSIGNED TO:** Database Architect (schema) → CISO (seguridad de datos) → Backend Developer (implementación)
**DEPENDS ON:** AOS-001
**BLOCKED BY:** Ninguno

#### Descripción
Implementar la capa de persistencia SQLite que almacena el historial de tareas, logs de ejecución, y métricas de uso del LLM Gateway. Esto es la memoria del agente — permite ver qué se hizo, cuánto costó, y qué falló.

#### Criterios de aceptación
- [ ] Clase async `TaskStore` con métodos CRUD para tareas
- [ ] Schema de tabla `tasks`: id, source (telegram/chat/api), input_text, classification (tipo, complejidad, tier), model_used, tokens_in, tokens_out, cost_estimate, status (pending/running/completed/failed), output_text, error_message, created_at, completed_at, duration_ms
- [ ] Schema de tabla `execution_log`: id, task_id (FK), executor_type (cli/api/screen), command_or_action, exit_code, stdout (truncado), stderr (truncado), duration_ms, created_at
- [ ] Schema de tabla `llm_usage`: id, task_id (FK), provider, model, tokens_in, tokens_out, cost_estimate, latency_ms, success (bool), created_at
- [ ] Índices en: tasks.status, tasks.created_at, tasks.source, llm_usage.provider, llm_usage.created_at
- [ ] Migración automática: la base de datos se crea/actualiza al iniciar la app
- [ ] Queries pre-construidas: tareas recientes, costo total por período, tasa de éxito por modelo
- [ ] Datos sensibles (API keys, tokens de sesión) NUNCA almacenados en estas tablas
- [ ] Tests unitarios con base de datos in-memory
- [ ] Pasa `ruff check` y `ruff format`

#### Inputs
- Especificación de producto (sección 3.1 — Context Engine con SQLite)
- Métricas de éxito (sección 9.1)
- Estructura de repositorio (AOS-001)

#### Output esperado
- Data Design Document con schema completo (Database Architect)
- Security Requirements para datos (CISO)
- Código fuente en `store/task_store.py` con tests y migraciones

#### Notas
SQLite es la elección para Phase 1-3 (single user, local). El schema debe poder evolucionar — el DBA debe diseñar pensando en migraciones futuras. El formato append-only de execution_log es intencional para audit trail.

---

### TICKET: AOS-007
**TITLE:** LLM Gateway — Rastreo de costos y medición de uso
**PHASE:** 1-brain
**SPRINT:** 2
**PRIORITY:** Media
**ASSIGNED TO:** Backend Developer
**DEPENDS ON:** AOS-002
**BLOCKED BY:** Ninguno

#### Descripción
Agregar al LLM Gateway la capacidad de rastrear tokens consumidos, calcular costos estimados por llamada, y acumular métricas de uso. Esto alimenta tanto la tabla `llm_usage` del SQLite Store como el dashboard de costos que vendrá en Phase 3.

#### Criterios de aceptación
- [ ] Conteo preciso de tokens (input + output) por cada llamada al LLM
- [ ] Cálculo de costo estimado basado en precios configurables por modelo
- [ ] Tabla de precios por defecto cargada desde config YAML (actualizable sin cambiar código)
- [ ] Métricas acumuladas en memoria: total tokens, total costo, calls por modelo, tasa de éxito
- [ ] Método `get_usage_summary(period: str) -> UsageSummary` para reportes
- [ ] Integración con `TaskStore` (AOS-006): cada llamada se registra en `llm_usage`
- [ ] Tests unitarios que verifican cálculos de costo con diferentes modelos
- [ ] Pasa `ruff check` y `ruff format`

#### Inputs
- Tabla de precios de la especificación (sección 3.2)
- Código del Gateway (AOS-002)
- Schema de llm_usage (AOS-006)

#### Output esperado
- Código fuente en `gateway/cost_tracker.py` con tests
- Config YAML con precios por modelo

#### Notas
Los precios cambian constantemente. El sistema debe leer precios de un archivo de configuración, no hardcodearlos. Este ticket es de prioridad media porque el Gateway funciona sin cost tracking — es una mejora de observabilidad.

---

## SPRINT 3 — COMUNICACIÓN E INTEGRACIÓN (Semana 3)

**Objetivo:** Conectar todo: Telegram como entrada, Agent Core como cerebro, y la demo end-to-end funcionando.

---

### TICKET: AOS-008
**TITLE:** Integración con bot de Telegram
**PHASE:** 1-brain
**SPRINT:** 3
**PRIORITY:** Crítica
**ASSIGNED TO:** Backend Developer
**DEPENDS ON:** AOS-002, AOS-004
**BLOCKED BY:** Ninguno

#### Descripción
Implementar la integración con Telegram usando `python-telegram-bot`. El bot recibe mensajes del usuario, los pasa al pipeline del agente, y devuelve los resultados. Incluye manejo de conversaciones, indicadores de "escribiendo...", y respuestas formateadas.

#### Criterios de aceptación
- [ ] Bot se conecta a Telegram usando token configurado en vault encriptado
- [ ] Recibe mensajes de texto del usuario
- [ ] Envía indicador "typing..." mientras procesa
- [ ] Formatea respuestas con markdown de Telegram (bold, code blocks, etc.)
- [ ] Maneja mensajes largos (split automático si excede el límite de Telegram)
- [ ] Comando `/start` — mensaje de bienvenida con instrucciones básicas
- [ ] Comando `/status` — muestra estado del agente y estadísticas básicas
- [ ] Comando `/history` — muestra últimas 5 tareas con estado
- [ ] Manejo de errores graceful: si algo falla, el usuario recibe un mensaje informativo
- [ ] El token de Telegram NUNCA aparece en logs
- [ ] Bot corre como servicio async (no bloquea otros componentes)
- [ ] Tests unitarios con mock del API de Telegram
- [ ] Pasa `ruff check` y `ruff format`

#### Inputs
- Especificación de producto (sección 3.1 — Capa de comunicación)
- Código del Gateway (AOS-002) y CLI Executor (AOS-004)

#### Output esperado
- Código fuente en `messaging/telegram.py` con tests

#### Notas
Telegram es el primer canal de comunicación. WhatsApp y Discord vienen después. Diseñar la interfaz de mensajería de forma genérica para que agregar canales sea plug-and-play.

---

### TICKET: AOS-009
**TITLE:** Agent Core — Pipeline de procesamiento de tareas
**PHASE:** 1-brain
**SPRINT:** 3
**PRIORITY:** Crítica
**ASSIGNED TO:** Software Architect (arquitectura) → Backend Developer (implementación)
**DEPENDS ON:** AOS-002, AOS-003, AOS-004, AOS-005, AOS-006
**BLOCKED BY:** Ninguno

#### Descripción
Implementar el pipeline central que conecta todos los componentes: recibe una tarea (desde Telegram u otra fuente), la clasifica, selecciona el modelo óptimo, genera un plan de acción consultando el playbook activo, ejecuta la acción vía CLI, y devuelve el resultado. Este es el "cerebro" que orquesta todo.

#### Criterios de aceptación
- [ ] Clase async `AgentCore` que implementa el pipeline completo
- [ ] Pipeline: `receive_task → classify → select_model → plan → execute → respond`
- [ ] Cada paso del pipeline es un componente inyectable (dependency injection)
- [ ] Integra: TaskClassifier (AOS-003), LLMGateway (AOS-002), CLIExecutor (AOS-004), ContextFolder (AOS-005), TaskStore (AOS-006)
- [ ] El agente consulta el playbook activo para entender cómo responder
- [ ] Si no hay playbook activo, usa un comportamiento por defecto (asistente general)
- [ ] Cola de tareas async: puede recibir nuevas tareas mientras procesa una
- [ ] Estado de tarea actualizado en TaskStore en cada paso (pending → running → completed/failed)
- [ ] Manejo de errores en cada paso con logging y fallback
- [ ] Retry configurable: si el LLM falla, reintenta con modelo alternativo
- [ ] Tests de integración que simulan el pipeline completo
- [ ] Pasa `ruff check` y `ruff format`

#### Inputs
- Todos los componentes anteriores
- Especificación de producto (secciones 2.1, 3.1)

#### Output esperado
- Architecture Document del pipeline (Software Architect)
- Código fuente en `core/agent.py` con tests

#### Notas
Este es el ticket más complejo del sprint. El Architect debe definir el pipeline ANTES del build. El patrón recomendado es Chain of Responsibility o Pipeline pattern. Cada paso debe ser testeable en aislamiento. En Phase 1, el agente es "single-level" (todo es Junior). La jerarquía de agentes viene en Phase 4.

---

### TICKET: AOS-010
**TITLE:** Integración end-to-end y demo funcional
**PHASE:** 1-brain
**SPRINT:** 3
**PRIORITY:** Crítica
**ASSIGNED TO:** QA Engineer (testing) → Security Auditor (auditoría) → Performance Engineer (benchmarks) → Code Reviewer (revisión final)
**DEPENDS ON:** AOS-008, AOS-009
**BLOCKED BY:** Ninguno

#### Descripción
Verificar que todo el sistema funciona end-to-end: un mensaje de Telegram dispara el pipeline completo y devuelve un resultado. Incluye testing integral, auditoría de seguridad, benchmarks de performance, y revisión de código de todo el código producido en la Phase 1.

#### Criterios de aceptación
- [ ] **Demo funcional:** enviar mensaje por Telegram → agente ejecuta comando CLI → resultado regresa por Telegram
- [ ] **QA — Tests end-to-end:**
  - Happy path: comando simple (`echo "hello"`) funciona
  - Comando que falla (exit code != 0) retorna error informativo
  - Comando bloqueado (de la lista de seguridad) es rechazado
  - Sin API keys configuradas: mensaje de error claro
  - Timeout: comando que tarda demasiado es terminado
  - Mensajes concurrentes: dos mensajes seguidos no se pierden
- [ ] **Security Audit:**
  - API keys nunca en logs ni en output
  - CLI sandbox efectivamente bloquea comandos peligrosos
  - Token de Telegram nunca expuesto
  - Vault encriptado funciona correctamente
- [ ] **Performance:**
  - Latencia total (mensaje → respuesta) documentada
  - Overhead del Gateway (sin contar latencia del LLM) < 500ms
  - Uso de memoria base < 100 MB
  - Clasificador < 10ms
- [ ] **Code Review:**
  - Todo el código sigue la arquitectura definida
  - Todos los API contracts implementados correctamente
  - Tests existen y son significativos
  - No hay secrets hardcodeados
  - No hay TODOs sin ticket asociado

#### Inputs
- Todo el código producido en Phase 1
- Documentos de arquitectura, API contracts, security requirements
- Tickets AOS-001 a AOS-009 con criterios de aceptación

#### Output esperado
- QA Report (QA Engineer)
- Security Audit Report (Security Auditor)
- Performance Report (Performance Engineer)
- Code Review (Code Reviewer)
- Lista de bugs como tickets nuevos (si aplica)

#### Notas
Este ticket es la puerta de salida de la Phase 1. No se cierra hasta que los cuatro reportes de verificación estén aprobados. Bugs críticos o de seguridad alta bloquean el cierre.

---

## Pipeline de ejecución por ticket

Para cada ticket, el flujo de agentes es:

### AOS-001 (Repo structure)
```
PM crea ticket → Software Architect (define estructura) → DevOps (implementa) → Code Reviewer → PM cierra
```

### AOS-002 (Provider Abstraction)
```
PM → Software Architect (arquitectura) → API Designer (contratos) → CISO (seguridad) → PM valida plan → Product Owner aprueba → Backend Dev (código) → QA → Security Auditor → Code Reviewer → PM cierra
```

### AOS-003 (Task Classifier)
```
PM → ML/AI Engineer (diseño + código) → QA → Code Reviewer → PM cierra
```

### AOS-004 (CLI Executor)
```
PM → Software Architect (arquitectura) → CISO (seguridad + sandbox) → PM valida → Product Owner aprueba → Backend Dev (código) → QA → Security Auditor → Code Reviewer → PM cierra
```

### AOS-005 (CFP Parser)
```
PM → API Designer (spec del formato) → Backend Dev (código) → QA → Code Reviewer → PM cierra
```

### AOS-006 (SQLite Store)
```
PM → Database Architect (schema) → CISO (seguridad datos) → Backend Dev (código) → QA → Code Reviewer → PM cierra
```

### AOS-007 (Cost Tracker)
```
PM → Backend Dev (código) → QA → Code Reviewer → PM cierra
```

### AOS-008 (Telegram Bot)
```
PM → Backend Dev (código) → QA → Security Auditor → Code Reviewer → PM cierra
```

### AOS-009 (Agent Core)
```
PM → Software Architect (pipeline) → PM valida → Product Owner aprueba → Backend Dev (código) → QA → Code Reviewer → PM cierra
```

### AOS-010 (E2E Integration)
```
PM → QA (tests) → Security Auditor (audit) → Perf Engineer (benchmarks) → Code Reviewer (review) → PM cierra Phase 1
```

---

## Riesgos identificados para Phase 1

| Riesgo | Probabilidad | Impacto | Mitigación |
|--------|-------------|---------|------------|
| LiteLLM no cubre todos los proveedores necesarios | Baja | Alto | Wrapper propio sobre LiteLLM permite agregar proveedores custom |
| CLI Executor tiene vulnerabilidad de seguridad | Media | Crítico | CISO define threat model ANTES del build; Security Auditor verifica DESPUÉS |
| Clasificador de tareas demasiado impreciso | Media | Medio | v1 es reglas — fácil de iterar y mejorar con más reglas. v2 será ML |
| Telegram API rate limits | Baja | Bajo | Implementar queue y backoff. Un solo usuario difícilmente alcanza los límites |
| Sprint 3 demasiado denso (Telegram + Core + E2E) | Media | Alto | AOS-008 y AOS-009 pueden arrancar en paralelo. Si se complica, AOS-007 se mueve a Phase 2 |

---

## Criterios de éxito de Phase 1

Referencia: Especificación de producto, sección 9.1.

| Métrica | Target | Cómo se mide |
|---------|--------|--------------|
| Task success rate (CLI mode) | > 85% | QA report en AOS-010 |
| App crash rate | < 2% | QA stress tests en AOS-010 |
| Cold start time | < 3 seconds | Performance report en AOS-010 |
| Classifier latency | < 10ms | Performance report en AOS-010 |
| Gateway overhead | < 500ms | Performance report en AOS-010 |
| Memory base | < 100 MB | Performance report en AOS-010 |

---

## Próximos pasos

1. **Product Owner revisa y aprueba este sprint plan.**
2. Si aprobado, el primer ticket a ejecutar es **AOS-001** — abrir una conversación con el Software Architect.
3. En paralelo, el CISO puede empezar a pensar en el threat model general de Phase 1 (aplica a AOS-002, AOS-004, AOS-006).
4. Sprint 1 arranca inmediatamente tras aprobación.

---

*Documento generado por el Project Manager de AgentOS. Pendiente aprobación del Product Owner.*
