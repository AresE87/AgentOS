# FASE R127 — CONFIDENCE CALIBRATION: El agente sabe cuándo NO sabe

**Objetivo:** Cada respuesta tiene un confidence score calibrado. Si el agente no está seguro, lo dice: "I'm 40% confident — you should verify this." Nunca inventa datos y presenta ficción como hecho.

## Tareas
### 1. Confidence scoring
- Agregar al prompt: "Rate your confidence 0-100%. If below 60%, say 'I'm not sure about this.'"
- Parsear confidence del response
- Calibration: verificar que "80% confident" = correcto 80% del tiempo (historical accuracy)

### 2. Confidence indicators en UI
```
🤖 "The contract expires June 15, 2026."
  Confidence: ████████░░ 85% — from email dated March 10

🤖 "I think the meeting is at 3pm, but I'm not certain."
  Confidence: ████░░░░░░ 40% — based on similar past meetings
  ⚠️ You should verify this before relying on it.
```

### 3. Auto-verification for low confidence
- If confidence < 60% AND the task can be verified (file exists, API returns data):
  → Auto-verify before responding
  → "I wasn't sure, so I checked: the file IS on your desktop. Confidence: 95%."

### 4. "I don't know" is a valid answer
- If confidence < 30% and can't verify: "I don't have enough information to answer this reliably. Could you provide: [specific info needed]?"
- Track: how often the agent says "I don't know" vs making stuff up

## Demo
1. Factual question with data → "85% confident" → correct
2. Ambiguous question → "40% confident, you should verify" → honest
3. Question outside knowledge → "I don't know. I need: [specific info]" → doesn't hallucinate
4. Calibration chart: "When I say 80%, I'm right 78% of the time" ← well-calibrated
