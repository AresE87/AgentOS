# PROMPT PARA CLAUDE CODE — PHASE 4, SPRINT 14

## Documentos que adjuntás:

1. Phase4_Sprint_Plan.md (secciones AOS-039, AOS-040)
2. AOS-022_027_UX_Design.md (para referencia del design system)
3. El código completo del proyecto + Sprint 11+12+13

---

## El prompt (copiá desde acá):

Sos el Frontend Developer + QA Engineer del equipo de AgentOS. Phase 4, Sprint 14 — sprint final. El sistema multi-agente funciona en el backend. Ahora actualizás el dashboard para visualizar cadenas de sub-tareas y corrés la verificación E2E.

## Lo que tenés que producir

### Ticket 1: AOS-039 — Dashboard Updates
- Actualizar `frontend/src/pages/Home.tsx`:
  - Tareas complejas muestran badge "Chain" con progreso (3/5 subtasks)
  - Click en tarea → vista expandida con sub-tareas como timeline
- Actualizar `frontend/src/pages/Chat.tsx`:
  - Respuestas de cadenas muestran resultado final con botón "Show details"
  - Details: lista de sub-tareas con nivel, especialista, modelo, costo
- Nuevo componente: `frontend/src/components/ChainDetail.tsx`
  - Timeline vertical: cada sub-tarea con estado (✅/❌/⏳)
  - Dependencias visualizadas (quién espera a quién)
  - Costo total = suma de sub-tareas
- Nuevos IPC hooks para get_chain_details(chain_id)

### Ticket 2: AOS-040 — Integración E2E Phase 4
- Test: tarea simple → no descompone → Junior → respuesta directa
- Test: tarea compleja → descompone en 3 sub-tareas → cadena → resultado unificado
- Test: specialist selection → tarea de código → Security Specialist
- Test: failure → sub-tarea falla → retry con tier upgrade → éxito
- Test: partial failure → resultado parcial entregado
- Test: sub-tareas paralelas → se ejecutan en paralelo (timing)
- Test: dashboard muestra cadena con progreso
- Verificar que TODOS los tests de Phase 1, 2, 3 siguen pasando

## Reglas

- Frontend: seguir el design system existente (colores, tipografía, layout).
- Los detalles de cadena son opt-in: el usuario ve el resultado final por default.
- Los tests E2E usan mocks de LLM pero testean el flujo completo del Orchestrator.
- Performance: overhead del Orchestrator para tareas simples < 100ms.

Empezá con AOS-039.
