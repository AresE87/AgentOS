# C27 — MULTI-NODE INFRA REAL

## Objetivo
Hacer real una capa mínima de infraestructura multi-nodo/relay más seria que el mesh local básico.

## Qué implementar
1. Registro básico de nodos/relays.
2. Estado de salud.
3. Reenvío simple de tareas o heartbeat.
4. Distinción local node vs relay node.
5. Manejo de desconexión / retry mínimo.

## Definition of Done
- al menos 2 nodos pueden identificarse y reportar estado
- existe un flujo básico de relay o forwarding real
- el sistema muestra salud/estado

## Demo obligatoria
“Dos nodos + un relay reportando estado y enrutando una tarea simple”.

## No hacer
- hablar de multi-región/global infra si solo hay structs
