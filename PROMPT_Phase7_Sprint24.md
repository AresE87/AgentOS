# PROMPT PARA CLAUDE CODE — PHASE 7, SPRINT 24

## Documentos: Phase7_Sprint_Plan.md + AOS-061_070_Architecture.md (AOS-064, 065) + código + Sprint 23

## Prompt:

Sos el Backend Developer de AgentOS. Phase 7, Sprint 24. La identidad, discovery, y canales seguros ya funcionan. Ahora definís el protocolo de mensajes y la replicación de playbooks entre nodos.

### Ticket 1: AOS-064 — Mesh Protocol
- `agentos/mesh/protocol.py` → MeshMessage dataclass, message types, MeshState
- 9 tipos de mensaje: node_hello, node_status, node_goodbye, heartbeat, task_assign, task_result, task_progress, skill_request, skill_transfer
- MeshState: registry de nodos con estado, búsqueda por capability/load
- Message routing básico (si no conozco al nodo destino, envío via otro)
- Tests de cada tipo de mensaje (serialize/deserialize/validate)

### Ticket 2: AOS-065 — Skill Replication
- `agentos/mesh/replication.py` → SkillReplicator
- Inventario de skills como parte de node_hello
- skill_request → buscar qué nodo tiene el playbook → skill_transfer
- Transfer: enviar .aosp encriptado por el canal seguro (chunked si > 1MB)
- Auto-install en el nodo receptor
- CLIP embeddings incluidos en la transferencia
- Credentials NUNCA se transfieren — verificar con test
- Versionado: upgrade si el nodo ya tiene versión vieja
- Tests del flujo completo entre dos nodos mock
