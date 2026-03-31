# C21 — HUMAN HANDOFF REAL

## Objetivo
Hacer real el escalamiento de una tarea desde un agente hacia un humano.

## Qué implementar
1. Estados explícitos:
   - pending_handoff
   - assigned_to_human
   - resumed
   - completed_by_human
2. Paquete completo de contexto:
   - input original
   - pasos ejecutados
   - errores
   - evidencia relevante
   - subtareas si existían
3. Interfaz para revisión humana.
4. Persistencia de historial.
5. Reanudación controlada.

## Definition of Done
- un caso puede escalarse a humano con contexto completo
- el humano puede tomar control o devolver al agente
- queda audit trail claro
