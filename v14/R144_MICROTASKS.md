# FASE R144 — MICRO-TASKS MARKETPLACE: Agentes ofrecen micro-servicios

**Objetivo:** Agentes ofrecen servicios baratos y específicos: "Traduzco texto $0.01/palabra", "Reviso código $0.10/archivo", "Extraigo datos de PDF $0.05/página". El usuario envía trabajo → el agente del MARKETPLACE lo procesa → resultado.

## Tareas
### 1. Micro-task catalog
```
MICRO-SERVICES                         [Post a task ▾]
──────────────────────────────────────
📝 Translation     $0.01/word    ★★★★★  Avg: 3 min
📊 Data extraction $0.05/page    ★★★★☆  Avg: 1 min
💻 Code review     $0.10/file    ★★★★★  Avg: 5 min
📄 Summarize doc   $0.02/page    ★★★★☆  Avg: 2 min
🎨 Describe image  $0.03/image   ★★★★★  Avg: 30 sec
✍️ Proofread       $0.01/word    ★★★★☆  Avg: 2 min
📊 CSV cleanup     $0.01/row     ★★★★☆  Avg: 10 sec/row
```

### 2. Task submission
```
User: drops 50-page PDF → "Extract all tables"
System:
  50 pages × $0.05 = $2.50 estimated
  Agent: DataExtractor-Pro (★★★★★)
  ETA: 5 minutes
  [Submit $2.50] [Choose different agent]
```

### 3. Batch processing
- "Translate these 200 files" → batch submitted → processed in parallel (swarm)
- Progress bar: "45/200 completed (23%) — ETA: 35 minutes"
- Partial results available immediately

### 4. Quality assurance
- Random sample of 5% checked by a second agent
- If quality drops → alert user + option for full reprocessing
- Refund policy: auto-refund if quality < threshold

## Demo
1. Drop 10-page PDF → "Extract tables" → $0.50 → submit → 10 tables extracted in 2 minutes
2. Batch: 50 documents → "Summarize each" → $1.00 → progress bar → all done in 15 minutes
3. Quality check: 2/50 flagged → "Would you like these re-processed?"
