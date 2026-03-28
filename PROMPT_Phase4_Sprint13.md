# PROMPT PARA CLAUDE CODE — PHASE 4, SPRINT 13

## Documentos que adjuntás:

1. Phase4_Sprint_Plan.md
2. AOS-032_038_Architecture.md (secciones AOS-037, AOS-038)
3. El código completo del proyecto + Sprint 11+12

---

## El prompt (copiá desde acá):

Sos el Backend Developer del equipo de AgentOS. Phase 4, Sprint 13. Los niveles, especialistas, decomposer, cadenas, y contexto compartido ya existen. Ahora construís el Orchestrator (el cerebro maestro) y el sistema de recuperación de fallos.

## Lo que tenés que producir

### Ticket 1: AOS-037 — Orchestrator
- `agentos/hierarchy/orchestrator.py` → Orchestrator
- process(): clasifica → decide si descomponer → ejecuta directo O arma cadena → compila resultado
- _select_level(): complexity → AgentLevel
- _select_profile(): busca especialista o usa default del nivel
- Se integra como el NUEVO punto de entrada principal (reemplaza AgentCore.process() en main.py y ipc_server.py)
- Backward compatible: tareas simples (complexity <= 2) siguen el mismo path
- GARANTÍA: nunca lanza excepciones, siempre retorna TaskResult
- Log detallado de cada decisión del orchestrator
- Tests con tareas simples (directo) y complejas (cadena)

### Ticket 2: AOS-038 — Failure Handling
- `agentos/hierarchy/recovery.py` → RecoveryStrategy
- Retry simple: hasta 2 reintentos con misma config
- Tier upgrade: si falló con Tier 1, reintentar con Tier 2
- Specialist swap: intentar otro especialista del mismo dominio
- Partial success: si algunas sub-tareas completaron, devolver resultado parcial
- Integrar en ChainExecutor.execute() (modificar para usar RecoveryStrategy)
- Tests de cada estrategia de recovery

## Reglas

- El Orchestrator es el NUEVO entry point. main.py y ipc_server.py ahora llaman a orchestrator.process() en lugar de agent_core.process().
- El usuario NUNCA ve niveles, especialistas, ni cadenas — solo ve el resultado final.
- Para tareas simples, la latencia debe ser idéntica a antes (el overhead del Orchestrator es solo una clasificación extra).
- Partial success: el TaskResult incluye qué sub-tareas completaron y cuáles fallaron, en un formato legible.

Empezá con AOS-037.
