# C28 — FEDERATED LEARNING BASIC REAL

## Objetivo
Hacer real una versión mínima y honesta de aprendizaje federado o compartido sin exponer datos sensibles.

## Alcance recomendado
No entrenar modelos completos.
Empezar por:
- compartir métricas agregadas
- compartir heurísticas/configs
- compartir rankings o patrones anonimizados

## Qué implementar
1. Payload agregado/anónimo.
2. Reglas de exclusión de datos sensibles.
3. Sincronización mínima entre nodos.
4. Trazabilidad de qué se comparte.

## Definition of Done
- dos instancias pueden compartir una mejora agregada no sensible
- queda claro qué no se comparte
- existe evidencia de sincronización real

## Demo obligatoria
“Dos nodos comparten señal agregada sin compartir contenido privado”.

## No hacer
- afirmar federated learning si solo mandás logs crudos
