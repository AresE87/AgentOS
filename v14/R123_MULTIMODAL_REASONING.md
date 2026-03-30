# FASE R123 — MULTI-MODAL REASONING: Pensar con texto+imagen+audio+datos juntos

**Objetivo:** El agente no solo PROCESA cada modalidad por separado (R82) — ahora RAZONA combinándolas. "Mirá este gráfico [imagen] y escuchá este audio [grabación de reunión] — ¿el equipo de ventas va bien o mal según ambas fuentes?"

## Tareas
### 1. Cross-modal reasoning prompt
- Enviar TODAS las modalidades en un solo prompt al LLM vision
- "I'm giving you: 1) A sales chart [image], 2) A meeting transcript [text from audio], 3) A database query result [table]. Reason across ALL three to answer the question."

### 2. Conflict detection between modalities
- "The chart shows growth but the transcript mentions 'disappointing quarter' — there's a conflict. The chart shows revenue (up 10%) but the team discussed margins (down 5%). Both are correct — revenue grew but margins shrank."

### 3. Evidence-based responses
- Cada afirmación del agente cita la fuente: "Revenue grew 10% [from chart] but margins declined 5% [from transcript]"

### 4. Frontend: sources panel
```
🤖 "Sales revenue grew but profitability declined."

  📎 Sources:
  ├─ 📊 Chart: Revenue line shows +10% YoY [image, quadrant 2]
  ├─ 🎤 Meeting: "margins are concerning" [audio, timestamp 14:23]
  └─ 📊 Database: margin = 12% vs 17% last year [query result, row 3]
```

## Demo
1. Upload chart + audio + "analyze" → cross-modal analysis citing all sources
2. Conflicting sources → agent identifies and explains the conflict
3. Each claim has source attribution (image region, audio timestamp, data row)
