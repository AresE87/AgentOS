# PROMPT PARA CLAUDE CODE — PHASE 4, SPRINT 12

## Documentos que adjuntás:

1. Phase4_Sprint_Plan.md
2. AOS-032_038_Architecture.md (secciones AOS-035, AOS-036, schema SQLite)
3. El código completo del proyecto + Sprint 11

---

## El prompt (copiá desde acá):

Sos el Backend Developer del equipo de AgentOS. Phase 4, Sprint 12. Los niveles de agente, especialistas, y el decomposer ya existen. Ahora construís el motor de cadenas de tareas y la comunicación entre agentes.

## Cómo leer los documentos

- **AOS-032_038_Architecture.md, AOS-035** → TaskChain, ChainExecutor, algoritmo de ejecución (topological sort + parallel execution), ChainStatus.
- **AOS-032_038_Architecture.md, AOS-036** → ChainContext: set/get por subtask, get_dependency_outputs() con resumen automático, serialización, límite de 50KB.
- **AOS-032_038_Architecture.md, schema SQLite** → Tablas nuevas: task_chains y chain_subtasks.

## Lo que tenés que producir

### Ticket 1: AOS-035 — Task Chain Engine
- `agentos/hierarchy/chain.py` → TaskChain, ChainStatus, ChainExecutor
- Ejecución respeta dependencias (DAG): B espera a A si depends_on=[A]
- Sub-tareas independientes se ejecutan en paralelo (asyncio.gather)
- Output de A se inyecta como contexto de B
- Timeout global de cadena (600s default)
- Estado se actualiza en TaskStore
- Migration SQLite: agregar tablas task_chains y chain_subtasks
- Tests con cadenas de 1, 3, y 5 sub-tareas con dependencias variadas

### Ticket 2: AOS-036 — Inter-Agent Communication
- `agentos/hierarchy/context.py` → ChainContext
- set(subtask_id, key, value) y get(subtask_id, key)
- get_dependency_outputs(): resume outputs largos a < 1000 chars
- Serialización a/desde dict para persistencia
- Límite de tamaño: 50KB total
- Tests de read/write/resume/serialización

## Reglas

- El ChainExecutor usa AgentCore.process() para cada sub-tarea (con el profile del nivel asignado).
- Si una sub-tarea falla, por ahora se marca la cadena como failed (recovery viene en Sprint 13).
- Los outputs de sub-tareas anteriores se pasan como contexto adicional al system prompt.
- Deadlock detection: si ninguna sub-tarea puede ejecutarse (todas esperan algo que falló), abortar.

Empezá con AOS-035.
