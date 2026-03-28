# SPRINT PLAN — PHASE 4: LA JERARQUÍA

**Proyecto:** AgentOS
**Fase:** 4 — The Hierarchy (Semanas 11–14)
**Sprints:** 4 (1 por semana)
**Preparado por:** Project Manager
**Fecha:** Marzo 2026
**Estado:** PENDIENTE APROBACIÓN DEL PRODUCT OWNER

---

## Objetivo de la fase

Transformar el agente de nivel único (Phase 1-3) en un **sistema multi-agente jerárquico** con 5 niveles (Junior → Specialist → Senior → Manager → Orchestrator). El Orchestrator descompone tareas complejas en sub-tareas, las asigna al nivel apropiado, crea cadenas de dependencia, y maneja fallos — todo invisible al usuario.

---

## Entregable final de la fase

El usuario envía "Research competitor pricing, create a comparison spreadsheet, and write a summary report". El Orchestrator descompone en 3 sub-tareas, asigna un Senior para investigación, un Specialist para la planilla, y otro Senior para el reporte. Cada sub-tarea espera a sus dependencias, se ejecuta con el modelo LLM óptimo para su nivel, y el resultado final se entrega al usuario como un paquete unificado. Si una sub-tarea falla, el sistema reintenta o reasigna.

---

## Conceptos clave

### Niveles de agente

| Nivel | Complejidad | LLM Tier | Cuándo se usa |
|-------|-------------|----------|---------------|
| **Junior** | Tareas simples y repetitivas | Tier 1 (cheap) | Respuestas directas, comandos simples, data entry |
| **Specialist** | Dominio específico | Tier 1-2 | Tareas con system prompt especializado (contable, dev, marketer) |
| **Senior** | Multi-paso complejo | Tier 2 (standard) | Research, análisis, code review, documentos largos |
| **Manager** | Orquesta sub-tareas | Tier 2-3 (standard/premium) | Workflows multi-app, pipelines, coordinación |
| **Orchestrator** | Meta-planning | Tier 2 (standard) | Descomponer, asignar, monitorear, manejar fallos |

### Principio de diseño

El usuario NUNCA ve los niveles. Envía un mensaje → el sistema decide internamente qué nivel(es) necesita → ejecuta → devuelve resultado. Tareas simples = un Junior. Tareas complejas = un equipo completo. Transparente.

---

## Resumen de tickets

| Ticket | Título | Sprint | Prioridad | Asignado a | Depende de |
|--------|--------|--------|-----------|------------|------------|
| AOS-032 | Agent Levels — Sistema de niveles y perfiles de agente | S11 | Crítica | Software Architect → Backend Dev | Phase 3 completa |
| AOS-033 | Specialist Profiles — Paquetes de especialistas pre-diseñados | S11 | Alta | ML/AI Engineer | AOS-032 |
| AOS-034 | Task Decomposer — Descomposición de tareas complejas | S11 | Crítica | ML/AI Engineer → Backend Dev | AOS-032 |
| AOS-035 | Task Chain Engine — Cadenas de dependencia entre sub-tareas | S12 | Crítica | Software Architect → Backend Dev | AOS-034 |
| AOS-036 | Inter-Agent Communication — Estado compartido entre agentes | S12 | Alta | API Designer → Backend Dev | AOS-035 |
| AOS-037 | Orchestrator — El meta-agente que coordina todo | S13 | Crítica | Software Architect → ML/AI → Backend Dev | AOS-034, AOS-035, AOS-036 |
| AOS-038 | Failure Handling — Retry, reasignación, recovery | S13 | Alta | Software Architect → Backend Dev | AOS-035, AOS-037 |
| AOS-039 | Dashboard Updates — Visualización de cadenas y sub-tareas | S14 | Alta | Frontend Dev | AOS-037 |
| AOS-040 | Integración E2E Phase 4 — Demo multi-agente | S14 | Crítica | QA | Todo |

---

## Diagrama de dependencias

```
Phase 3 completa
    │
    ├── AOS-032 (Agent Levels) ──┬── AOS-033 (Specialist Profiles)
    │                            │
    │                            └── AOS-034 (Task Decomposer)
    │                                    │
    │                                    ├── AOS-035 (Task Chain Engine)
    │                                    │       │
    │                                    │       ├── AOS-036 (Inter-Agent Comm)
    │                                    │       │
    │                                    │       └── AOS-038 (Failure Handling)
    │                                    │
    │                                    └── AOS-037 (Orchestrator)
    │                                            │
    │                                    ┌───────┘
    │                                    │
    │                            AOS-039 (Dashboard Updates)
    │                                    │
    └─────────────────────── AOS-040 (E2E Phase 4)
```

---

## SPRINT 11 — NIVELES Y DESCOMPOSICIÓN (Semana 11)

### TICKET: AOS-032
**TITLE:** Agent Levels — Sistema de niveles y perfiles de agente
**SPRINT:** 11
**PRIORITY:** Crítica
**ASSIGNED TO:** Software Architect → Backend Dev

#### Descripción
Implementar el sistema de niveles que permite crear agentes con diferentes capacidades, system prompts, y tiers de LLM. Cada nivel es una configuración que el pipeline de AgentCore usa para ajustar su comportamiento.

#### Criterios de aceptación
- [ ] Enum `AgentLevel`: JUNIOR, SPECIALIST, SENIOR, MANAGER, ORCHESTRATOR
- [ ] Dataclass `AgentProfile` con: level, system_prompt, tier, allowed_tools, max_tokens, temperature
- [ ] Registry de perfiles: cargar profiles desde YAML config
- [ ] Perfiles default incluidos: un profile por nivel
- [ ] AgentCore acepta un `AgentProfile` que modifica su comportamiento (tier, system prompt, herramientas)
- [ ] Backward compatible: sin profile explícito = Junior (comportamiento actual)
- [ ] Tests de cada nivel con mocks

### TICKET: AOS-033
**TITLE:** Specialist Profiles — Paquetes de especialistas pre-diseñados
**SPRINT:** 11
**PRIORITY:** Alta
**ASSIGNED TO:** ML/AI Engineer

#### Descripción
Crear los perfiles de especialistas pre-diseñados que se incluyen con AgentOS. Cada especialista tiene: system prompt optimizado para su dominio, tier de LLM recomendado, herramientas preferidas, y contexto de dominio.

#### Criterios de aceptación
- [ ] Al menos 8 especialistas de las categorías de la spec (1 por categoría)
- [ ] Cada especialista como archivo YAML en `config/specialists/`
- [ ] Format: name, category, system_prompt, tier, tools, description
- [ ] System prompts de al menos 200 palabras cada uno (detallados y útiles)
- [ ] El Orchestrator puede seleccionar especialistas por categoría y task_type
- [ ] Tests: cargar cada especialista y verificar format válido

### TICKET: AOS-034
**TITLE:** Task Decomposer — Descomposición de tareas complejas
**SPRINT:** 11
**PRIORITY:** Crítica
**ASSIGNED TO:** ML/AI Engineer → Backend Dev

#### Descripción
Implementar la capacidad de descomponer una tarea compleja en sub-tareas atómicas. El Decomposer usa un LLM para analizar la tarea y producir un plan de ejecución estructurado.

#### Criterios de aceptación
- [ ] `TaskDecomposer.decompose(task_input) → TaskPlan`
- [ ] `TaskPlan` contiene: sub-tareas ordenadas, dependencias entre ellas, nivel sugerido para cada una
- [ ] El Decomposer llama al LLM con un prompt específico para descomposición
- [ ] Detección automática: si complexity <= 2, no descompone (ejecuta directo)
- [ ] Si complexity >= 3, intenta descomponer
- [ ] Límite: máximo 10 sub-tareas por plan (previene over-decomposition)
- [ ] Formato de salida del LLM: JSON estructurado con schema definido
- [ ] Tests con tareas simples (no descompone) y complejas (descompone)

---

## SPRINT 12 — CADENAS Y COMUNICACIÓN (Semana 12)

### TICKET: AOS-035
**TITLE:** Task Chain Engine — Cadenas de dependencia entre sub-tareas
**SPRINT:** 12
**PRIORITY:** Crítica
**ASSIGNED TO:** Software Architect → Backend Dev

#### Descripción
Implementar el motor que ejecuta cadenas de sub-tareas respetando dependencias. La Tarea B espera a que la Tarea A complete antes de ejecutarse. El output de A se pasa como input a B.

#### Criterios de aceptación
- [ ] `TaskChain` contiene sub-tareas con dependencias (DAG — directed acyclic graph)
- [ ] `ChainExecutor.execute(chain) → ChainResult`
- [ ] Ejecución respeta dependencias: B espera a A si B depende de A
- [ ] Sub-tareas independientes se ejecutan en paralelo
- [ ] El output de una sub-tarea se inyecta como contexto de la siguiente
- [ ] Estado de la cadena se actualiza en TaskStore (cada sub-tarea tiene su propio registro)
- [ ] Timeout global de la cadena (configurable, default 600s)
- [ ] Si una sub-tarea falla, la cadena se marca como failed (por ahora — retry en AOS-038)
- [ ] Tests con cadenas de 1, 3, y 5 sub-tareas con dependencias variadas

### TICKET: AOS-036
**TITLE:** Inter-Agent Communication — Estado compartido entre agentes
**SPRINT:** 12
**PRIORITY:** Alta
**ASSIGNED TO:** API Designer → Backend Dev

#### Descripción
Implementar el mecanismo de comunicación entre agentes en una cadena. Cada agente puede leer el output de los agentes anteriores y aportar al estado compartido de la cadena.

#### Criterios de aceptación
- [ ] `ChainContext`: diccionario tipado compartido entre todos los agentes de una cadena
- [ ] Cada agente recibe: su input + context de la cadena + outputs de dependencias
- [ ] `context.set(key, value)` y `context.get(key)` con namespacing por sub-tarea
- [ ] El context se serializa en TaskStore para persistencia
- [ ] Un agente puede pedir al contexto "qué hizo el agente anterior" (resumen automático)
- [ ] Límite de tamaño del contexto: 50KB (previene explosión de memoria)
- [ ] Tests de lectura/escritura de contexto entre sub-tareas

---

## SPRINT 13 — ORCHESTRATOR (Semana 13)

### TICKET: AOS-037
**TITLE:** Orchestrator — El meta-agente que coordina todo
**SPRINT:** 13
**PRIORITY:** Crítica
**ASSIGNED TO:** Software Architect → ML/AI → Backend Dev

#### Descripción
Implementar el Orchestrator: el agente de nivel más alto que recibe tareas del usuario, decide si descomponer o ejecutar directo, ensambla el equipo de agentes, lanza la cadena, y devuelve el resultado final.

#### Criterios de aceptación
- [ ] `Orchestrator.process(task_input) → TaskResult`
- [ ] Flujo:
  1. Clasificar tarea (complexity, type)
  2. Si simple (complexity <= 2) → ejecutar directo con Junior/Specialist
  3. Si compleja (complexity >= 3) → descomponer con TaskDecomposer
  4. Para cada sub-tarea: seleccionar nivel de agente + especialista (si aplica)
  5. Armar TaskChain con dependencias
  6. Ejecutar cadena con ChainExecutor
  7. Compilar resultado final de todos los outputs
  8. Retornar al usuario
- [ ] El usuario NUNCA ve los niveles internos — solo ve el resultado final
- [ ] El Orchestrator se integra como replacement de AgentCore.process() para tareas complejas
- [ ] Backward compatible: tareas simples siguen el mismo path que antes
- [ ] Log detallado: qué nivel eligió, por qué, qué especialista asignó
- [ ] Tests del flujo completo con tareas simples (no descompone) y complejas (sí descompone)

### TICKET: AOS-038
**TITLE:** Failure Handling — Retry, reasignación, recovery
**SPRINT:** 13
**PRIORITY:** Alta
**ASSIGNED TO:** Software Architect → Backend Dev

#### Descripción
Implementar estrategias de recuperación cuando una sub-tarea falla dentro de una cadena.

#### Criterios de aceptación
- [ ] Retry: si una sub-tarea falla, reintentar hasta 2 veces (configurable)
- [ ] Retry con upgrade: si falla con Tier 1, reintentar con Tier 2
- [ ] Reasignación: si un especialista falla, intentar con otro del mismo dominio
- [ ] Partial success: si 3 de 5 sub-tareas completaron, devolver resultado parcial al usuario con indicación de qué falló
- [ ] Timeout handling: si una sub-tarea excede su timeout, marcarla como failed y evaluar si la cadena puede continuar
- [ ] Log de cada retry/reasignación para debugging
- [ ] Tests de cada estrategia de recovery

---

## SPRINT 14 — DASHBOARD Y E2E (Semana 14)

### TICKET: AOS-039
**TITLE:** Dashboard Updates — Visualización de cadenas y sub-tareas
**SPRINT:** 14
**PRIORITY:** Alta
**ASSIGNED TO:** Frontend Dev

#### Descripción
Actualizar el dashboard para mostrar cadenas de tareas, sub-tareas con dependencias, estado de cada una, y progreso general.

#### Criterios de aceptación
- [ ] En Home: las tareas complejas muestran un indicador "chain" con progreso (3/5 sub-tasks)
- [ ] Detalle de tarea: vista expandida con sub-tareas como lista/timeline
- [ ] Cada sub-tarea muestra: input, output, nivel, especialista, modelo, costo, estado
- [ ] Vista de dependencias: cuáles esperan, cuáles ejecutando, cuáles completadas
- [ ] Costo total de la cadena = suma de costos de sub-tareas
- [ ] En Chat: respuestas de cadenas muestran el resultado final con opción de expandir detalles

### TICKET: AOS-040
**TITLE:** Integración E2E Phase 4 — Demo multi-agente
**SPRINT:** 14
**PRIORITY:** Crítica
**ASSIGNED TO:** QA

#### Criterios de aceptación
- [ ] **Demo principal:** "Research X, create spreadsheet, write report" → 3 sub-tareas → resultado unificado
- [ ] **Tarea simple:** "what time is it?" → no descompone → Junior directo
- [ ] **Specialist:** "review this code for security issues" → Senior con profile de security specialist
- [ ] **Failure recovery:** sub-tarea falla → retry con tier upgrade → éxito
- [ ] **Partial failure:** 2 de 3 sub-tareas ok, 1 falla → resultado parcial entregado
- [ ] **Concurrencia:** sub-tareas independientes se ejecutan en paralelo
- [ ] **Dashboard:** cadena visible con progreso en tiempo real
- [ ] Todos los tests de Phase 1, 2, y 3 siguen pasando
- [ ] Performance: overhead del Orchestrator < 2s (sin contar LLM latency)

---

## Riesgos

| Riesgo | Probabilidad | Impacto | Mitigación |
|--------|-------------|---------|------------|
| LLM produce descomposiciones malas | Alta | Alto | Prompt iterado + validación de schema + fallback a ejecución directa |
| Cadenas largas exceden costo máximo | Media | Alto | Budget check antes de ejecutar cadena: sum(estimated_costs) < limit |
| Over-decomposition (10 sub-tareas para algo simple) | Media | Medio | Threshold de complexity: solo descompone si >= 3. Max 10 sub-tareas. |
| Context sharing entre agentes es confuso | Media | Medio | Resumen automático del output anterior (no pasa raw output de 5000 tokens) |
| Latencia total de cadenas es alta | Alta | Medio | Paralelismo de sub-tareas independientes. Tier 1 para sub-tareas simples. |

---

## Criterios de éxito de Phase 4

| Métrica | Target |
|---------|--------|
| Decomposition accuracy (plan hace sentido) | > 80% |
| Chain completion rate | > 75% |
| Retry success rate (falla → retry → éxito) | > 60% |
| Orchestrator overhead | < 2 seconds |
| Parallel sub-task speedup (3 independientes) | > 2x vs secuencial |
| User satisfaction (resultado final useful) | > 70% |
