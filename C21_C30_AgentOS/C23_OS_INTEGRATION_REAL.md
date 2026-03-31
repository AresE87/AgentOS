# C23 — OS INTEGRATION REAL

## Objetivo
Volver real la integración con el sistema operativo, empezando por Windows.

## Qué implementar
1. Integración de shell/context menu real para al menos 1 tipo de acción útil.
2. Envío de contexto del archivo/ruta/selección a AgentOS.
3. Apertura o disparo de flujo desde Explorer o equivalente.
4. Manejo seguro de inputs del sistema.

## Casos sugeridos
- click derecho en archivo → “Ask AgentOS”
- click derecho en carpeta → resumir/organizar
- enviar archivo a playbook específico

## Definition of Done
- el usuario dispara AgentOS desde el OS sin abrir manualmente la app
- el contexto llega correctamente
- hay flujo útil real

## Demo obligatoria
“Click derecho → Ask AgentOS → acción real sobre archivo/carpeta”.

## No hacer
- dejar solo spec/registry sin flujo útil
