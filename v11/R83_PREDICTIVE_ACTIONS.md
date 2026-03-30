# FASE R83 — PREDICTIVE ACTIONS: El agente anticipa lo que necesitás

**Objetivo:** Basado en patrones (hora, calendario, historial), el agente pre-computa tareas y las tiene listas. "Lunes 9am → briefing ya listo. 30min antes de reunión → prep listo."

---

## Tareas

### 1. Pattern detection: analizar historial → detectar tareas recurrentes (≥5 veces, mismo horario)
### 2. Pre-computation: ejecutar tareas anticipadas en background → cachear resultado
### 3. Calendar-aware: 30min antes de reunión → preparar briefing con emails y notas relacionadas
### 4. App-launch triggers: abrir VS Code → pre-computar git status y PRs pendientes
### 5. Frontend: "READY FOR YOU" section en Home con resultados pre-computados
### 6. Settings: confidence threshold (70%), max cost/day ($0.50), manage patterns

## Demo
1. Lunes 8:55am → "Monday briefing ready ✅" aparece antes de que preguntes
2. 25min antes de reunión → briefing con últimas interacciones del contacto
3. Settings muestra 5 patterns detectados con confidence %
