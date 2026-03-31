# C22 — AGENT DEBUGGER REAL

## Objetivo
Construir un debugger real para seguir la ejecución del agente paso a paso.

## Qué implementar
1. Traza de ejecución por step:
   - timestamp
   - acción planeada
   - modelo/agente usado
   - input resumido
   - output resumido
   - error si aplica
2. Vista ordenada de steps.
3. Filtros por task / agente / estado.
4. Posibilidad de inspeccionar evidencia asociada.
5. Persistencia mínima de traces.

## Definition of Done
- una ejecución compleja puede inspeccionarse paso a paso
- se entiende qué hizo el agente y por qué falló
- sirve para debugging real, no solo logging plano

## Demo obligatoria
“Abrir una ejecución y ver 5+ steps con estados reales”.

## No hacer
- volcar solo logs crudos sin estructura
- exponer pensamiento inventado; mostrar trazas operativas reales
