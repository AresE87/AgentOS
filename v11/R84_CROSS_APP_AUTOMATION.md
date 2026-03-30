# FASE R84 — CROSS-APP AUTOMATION: Mover datos entre apps

**Objetivo:** "Tomá los datos del Excel, mandá email con resumen, agendá reunión, y avisá en Slack" → 4 apps coordinadas automáticamente.

---

## Tareas

### 1. App connector registry: cada app conectada (R63-R66) como connector con capabilities
### 2. Cross-app workflow engine: el LLM encadena acciones de diferentes apps
### 3. Data transformation: Excel rows → HTML email table, DB result → Slack message, etc.
### 4. Natural language: el usuario describe el flujo, el agente identifica las apps e genera el plan
### 5. Board visualization: cadena cross-app con ícono de cada app en cada nodo
### 6. 5 templates pre-built: Sales pipeline, Invoice processing, Meeting prep, Bug triage, Newsletter

## Demo
1. "Excel → email → calendar → Slack" → 4 apps coordinadas → todo ejecutado en cadena
2. Board muestra cada step con ícono de la app correspondiente
3. Data transformation: tabla Excel se convierte en tabla HTML en el email automáticamente
