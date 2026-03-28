# PROMPT PARA CLAUDE CODE — PHASE 2, SPRINT 6

## Documentos que adjuntás:

1. Phase2_Sprint_Plan.md
2. AOS-015_018_Architecture.md (secciones AOS-017 y AOS-018)
3. AOS-019_Verification_Plan.md
4. El código completo de Phase 1 + Sprint 4 + Sprint 5

---

## El prompt (copiá desde acá):

Sos el Backend Developer del equipo de AgentOS. Estás en Phase 2, Sprint 6 — el sprint final de "The Eyes". Todo lo anterior está completo: Phase 1 (Brain), Sprint 4 (Capture + Controller + Vision), Sprint 5 (CLIP + Recorder + CFP v2). Ahora conectás todo.

## Cómo leer los documentos

- **AOS-015_018_Architecture.md, AOS-017** → ModeSelector: ExecutorMode enum, fallback chain API > CLI > Screen, tabla de decisión, ModeDecision dataclass.
- **AOS-015_018_Architecture.md, AOS-018** → ScreenExecutor: el loop capture → analyze → act → verify, integración con AgentCore, max_iterations, timeout, uso de visual memory.
- **AOS-019_Verification_Plan.md** → Los 43 tests E2E que DEBEN pasar, security audit checklist, performance targets.

## Lo que tenés que producir

### Ticket 1: AOS-017 — Smart Mode Selection
- `executor/mode_selector.py` → ModeSelector con select()
- ExecutorMode enum: API, CLI, SCREEN
- Lógica de fallback chain basada en permisos del playbook
- Logging de cada decisión
- Tests de cada combinación de la tabla de decisión

### Ticket 2: AOS-018 — Screen Executor
- `executor/screen_executor.py` → ScreenExecutor con execute()
- Loop: capture → (visual memory search) → vision analyze → controller act → verify → repeat
- max_iterations=20, timeout=300
- Verificación post-acción: si nada cambió, retry hasta 3x
- Integración en AgentCore.process() — extender _execute_action() con ModeSelector
- Tests del loop completo con todos los componentes mockeados

### Ticket 3: AOS-019 — Integración E2E Phase 2
- Tests E2E: E1 a E43 del verification plan
- Verificar que TODOS los tests de Phase 1 siguen pasando
- Verificar security checklist (hotkeys bloqueadas, secrets protegidos, permisos enforced)
- Verificar que el fallback CLI → Screen funciona
- `main.py` actualizado para inicializar los componentes de Phase 2

## Reglas

- ScreenExecutor.execute() retorna ExecutionResult (misma interfaz que CLIExecutor).
- AgentCore.process() SIGUE sin lanzar excepciones — captura todo.
- El fallback es transparente: si CLI falla y Screen tiene éxito, el usuario recibe el resultado sin saber que hubo fallback.
- Todos los tests usan mocks — no requieren display real, API keys, ni modelo CLIP real.
- Después de todo: `make check` (lint + ALL tests from Phase 1 + Phase 2) DEBE pasar.

Empezá con AOS-017.
