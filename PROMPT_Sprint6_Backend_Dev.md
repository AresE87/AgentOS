# PROMPT PARA CLAUDE CODE — SPRINT 6 (Phase 2)

Copiá todo lo que está debajo de la línea y pegalo como primer mensaje.
Después adjuntá los documentos indicados.

---

## Documentos que tenés que adjuntar:

1. AgentOS_Sprint_Plan_Phase2.md
2. AOS-017_018_Architecture.md

IMPORTANTE: También adjuntá el código completo de Phase 1 + Sprint 4 + Sprint 5.

---

## El prompt (copiá desde acá):

Sos el Backend Developer de AgentOS. Sprint 4 y 5 están completos (todos los componentes de screen). Ahora Sprint 6: conectar todo con Smart Mode Selection y verificación E2E.

## Cómo leer los documentos

- **AOS-017_018_Architecture.md** → ModeSelector con reglas de selección y fallback, integración con Agent Core (modifica AOS-009), y el Verification Plan completo con 16 tests E2E, security audit, y benchmarks de performance.

## Lo que tenés que producir

### Ticket 1: AOS-017 — Smart Mode Selection
- executor/mode_selector.py → ModeSelector + ModeDecision + ExecutionMode
- Reglas de selección: VISION→SCREEN, bash blocks→CLI, playbook permissions, default→CLI
- execute_with_fallback(): intenta modo principal, si falla → siguiente en cadena
- Tabla de fallback definida en el documento
- Forzar modo: el usuario puede decir "usa screen control"
- Modificar core/agent.py (AOS-009) para usar ModeSelector en Step 5
- Tests de cada regla de selección + tests de fallback

### Ticket 2: AOS-018 — E2E Phase 2
- tests/test_phase2_e2e.py → Tests V1 a V6 (happy paths) con mocks
- tests/test_phase2_errors.py → Tests V7 a V12 (error handling)
- tests/test_visual_memory.py → Tests V13 a V16
- Verificar que TODOS los tests de Phase 1 siguen pasando (regression)
- main.py actualizado para inicializar los componentes de screen

## Reglas críticas

- ModeSelector NO cambia la interfaz pública de AgentCore.process() — solo la implementación interna
- El fallback es transparente: el caller recibe ExecutionResult sin saber si hubo fallback
- El ExecutionResult incluye qué modo se usó efectivamente
- Kill switch SIEMPRE activo cuando hay screen control
- Todos los tests E2E usan mocks completos — no dependen de display real

Empezá con AOS-017.
