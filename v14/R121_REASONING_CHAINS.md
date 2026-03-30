# FASE R121 — REASONING CHAINS VISIBLE: Ver cómo piensa el agente

**Objetivo:** Cada respuesta del agente tiene un panel expandible "Show reasoning" que muestra el chain-of-thought paso a paso: qué consideró, qué descartó, y por qué eligió la respuesta final.

## Tareas
### 1. Solicitar chain-of-thought al LLM
- Agregar al system prompt: "Think step by step. Wrap your reasoning in <thinking>...</thinking> tags before your final answer."
- Parsear la respuesta: separar thinking del output final
- Guardar thinking en task_steps como metadata

### 2. Frontend: expandible "Show reasoning"
```
🤖 Agent: "Your disk is at 64%, which is healthy."

  ▼ Show reasoning (3 steps)
  ┌─────────────────────────────────────────────┐
  │ Step 1: Classified as system query (CLI)     │
  │ Step 2: Generated PowerShell: Get-PSDrive C  │
  │ Step 3: Parsed output: 320GB used / 500GB    │
  │         = 64%. Under 80% threshold = healthy  │
  │ Conclusion: Report disk usage as healthy      │
  └─────────────────────────────────────────────┘
```

### 3. Reasoning for agent selection
- "Why did I choose Code Reviewer?" → "Task contains 'review code' keywords, complexity=3, selected Senior tier"
- Visible en el debugger (R96) y como tooltip en Board cards

### 4. Reasoning for routing
- "Why claude-sonnet?" → "Tier 2 task, code type, sonnet has 96% success rate for code tasks"

## Demo
1. Enviar tarea → respuesta → click "Show reasoning" → 3 steps explicados
2. Board card → hover sobre agente → tooltip: "Selected because: keywords match + tier 2"
3. Cada decisión del pipeline es explicable y visible
