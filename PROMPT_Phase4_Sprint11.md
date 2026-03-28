# PROMPT PARA CLAUDE CODE — PHASE 4, SPRINT 11

## Documentos que adjuntás:

1. Phase4_Sprint_Plan.md
2. AOS-032_038_Architecture.md (secciones AOS-032, AOS-033, AOS-034)
3. El código Python completo del proyecto (Phase 1+2+3)

---

## El prompt (copiá desde acá):

Sos el Backend Developer + ML/AI Engineer del equipo de AgentOS. Estás en Phase 4 (The Hierarchy) — transformar el agente de nivel único en un sistema multi-agente jerárquico. Sprint 11: crear los niveles de agente, los perfiles de especialistas, y el descomponedor de tareas.

## Cómo leer los documentos

- **Phase4_Sprint_Plan.md** → Contexto general, niveles de agente, principio de diseño (usuario nunca ve los niveles).
- **AOS-032_038_Architecture.md, AOS-032** → AgentLevel enum, AgentProfile dataclass, DEFAULT_PROFILES, integración con AgentCore (agregar parámetro profile).
- **AOS-032_038_Architecture.md, AOS-033** → Formato YAML de especialistas, los 8 especialistas iniciales, SpecialistRegistry.
- **AOS-032_038_Architecture.md, AOS-034** → TaskDecomposer, SubTaskDefinition, TaskPlan, prompt de descomposición, should_decompose().

## Lo que tenés que producir

### Ticket 1: AOS-032 — Agent Levels
- `agentos/hierarchy/levels.py` → AgentLevel enum + AgentProfile dataclass + DEFAULT_PROFILES
- Modificar `core/agent.py` → AgentCore.process() acepta profile y chain_context opcionales
- `config/levels.yaml` → Configuración de perfiles por nivel (system prompts, tiers)
- Backward compatible: sin profile = Junior (comportamiento actual idéntico)
- Tests de cada nivel

### Ticket 2: AOS-033 — Specialist Profiles
- `agentos/hierarchy/specialists.py` → SpecialistRegistry
- `config/specialists/` → 8 archivos YAML (uno por categoría)
- System prompts detallados (200+ palabras cada uno)
- select_best() con heurística de keywords
- Tests: cargar cada especialista, seleccionar por categoría

### Ticket 3: AOS-034 — Task Decomposer
- `agentos/hierarchy/decomposer.py` → TaskDecomposer
- should_decompose(): True si complexity >= 3
- decompose(): llama al LLM con prompt de descomposición, parsea JSON
- Máximo 10 sub-tareas, validación de schema
- Tests con mocks del LLM (respuestas JSON pre-definidas)

## Reglas

- El directorio `agentos/hierarchy/` es NUEVO — crearlo con __init__.py.
- AgentCore SIGUE funcionando exactamente igual si no se pasa profile.
- Los system prompts de especialistas deben ser genuinamente útiles (no genéricos).
- TaskDecomposer usa Tier 2 (STANDARD) para la llamada de descomposición.
- Todos los tests de fases anteriores deben seguir pasando.

Empezá con AOS-032.
