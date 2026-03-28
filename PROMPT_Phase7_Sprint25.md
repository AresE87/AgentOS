# PROMPT PARA CLAUDE CODE — PHASE 7, SPRINT 25

## Documentos: Phase7_Sprint_Plan.md + AOS-061_070_Architecture.md (AOS-066, 067) + código + Sprint 23-24

## Prompt:

Sos el Backend Developer de AgentOS. Phase 7, Sprint 25. Toda la capa de red funciona (identity, discovery, channels, protocol, replication). Ahora construís la orquestación distribuida y el manejo de fallos de nodos.

### Ticket 1: AOS-066 — Cross-Node Orchestrator
- `agentos/mesh/mesh_orchestrator.py` → MeshOrchestrator (extiende Orchestrator de Phase 4)
- _select_node(): elige el mejor nodo para cada sub-tarea (specialist + carga + online)
- _execute_remote(): envía task_assign y espera task_result via SecureChannel
- El resultado remoto se integra en ChainContext como si fuera local
- El usuario NO sabe qué nodo ejecutó qué — resultado unificado
- Fallback: si nodo remoto falla → ejecutar local
- Tests con 2-3 nodos simulados

### Ticket 2: AOS-067 — Node Failure Handling
- `agentos/mesh/failure.py` → NodeFailureHandler
- Detección: 3 heartbeats perdidos → marcar offline
- Reasignación: tareas pendientes en nodo offline → otro nodo o local
- Graceful shutdown: node_goodbye con lista de tareas pendientes
- Reconexión: si nodo vuelve → sincronizar estado
- Tests de cada scenario (muerte, regreso, graceful shutdown)
