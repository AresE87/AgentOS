# PROMPT PARA CLAUDE CODE — PHASE 7, SPRINT 26 (SPRINT FINAL DEL PROYECTO)

## Documentos: Phase7_Sprint_Plan.md + AOS-061_070_Architecture.md (AOS-068, 069, 070) + código completo

## Prompt:

Sos el Frontend Developer + Security Auditor + QA de AgentOS. Phase 7, Sprint 26 — EL ÚLTIMO SPRINT DE TODO EL PROYECTO. El mesh funciona en el backend. Ahora el dashboard, la auditoría de seguridad, y la verificación final.

### Ticket 1: AOS-068 — Mesh Dashboard
- Nueva sección "Mesh" en sidebar del dashboard (5to item)
- Vista de nodos: cards con nombre, OS, estado, load %, specialists
- Mapa visual simple: nodos como círculos conectados con líneas
- En task chains distribuidas: indicar qué nodo ejecutó cada sub-tarea
- "Add Node" (manual) y "Scan Network" (refresh mDNS)
- Settings de mesh: enable/disable, port, relay URL

### Ticket 2: AOS-069 — Mesh Security Audit
- Verificar entropía de keypairs
- Verificar mutual authentication (no se puede impersonar)
- Verificar encriptación E2E (Wireshark test / traffic inspection mock)
- Verificar que credentials NUNCA se transfieren
- Verificar anti-replay (mensajes viejos rechazados)
- Verificar que relay no puede leer contenido
- Documentar findings y recomendaciones

### Ticket 3: AOS-070 — E2E Final
- Demo discovery: 2 nodos se descubren por mDNS (mock)
- Demo distribution: tarea compleja → sub-tareas en 2 nodos → resultado
- Demo replication: playbook se transfiere automáticamente
- Demo failure: nodo se cae → reasignación → completa
- Demo dashboard: nodos visibles, tareas trazables
- REGRESIÓN COMPLETA: todos los tests de Phase 1 a 7 pasan
- make check = 100% GREEN

Este es el cierre del proyecto. Todo lo que no funciona se documenta como ticket de bug futuro.
