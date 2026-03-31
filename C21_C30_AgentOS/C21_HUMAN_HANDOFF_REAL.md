# C21 — HUMAN HANDOFF REAL

## Objetivo
Hacer real el escalamiento de una tarea desde un agente hacia un humano cuando:
- el agente no puede continuar
- la confianza cae por debajo de un umbral
- ocurre error parcial o bloqueo externo
- la tarea requiere aprobación/decisión humana

## Qué implementar
1. Estado explícito de handoff:
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
3. Interfaz para que un humano:
   - vea el caso
   - agregue nota/decisión
   - reanude o cierre
4. Persistencia de historial de handoff.
5. Reanudación controlada del flujo del agente.

## Definition of Done
- un caso puede escalarse a humano con contexto completo
- el humano puede tomar control o devolver al agente
- queda audit trail claro

## Demo obligatoria
“Agente falla / duda → handoff → humano revisa → reanuda o cierra”.

## No hacer
- solo un campo string “needs_human”
- handoff sin contexto utilizable
