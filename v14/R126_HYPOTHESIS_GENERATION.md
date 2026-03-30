# FASE R126 — HYPOTHESIS GENERATION: Generar y testear hipótesis en paralelo

**Objetivo:** Ante un problema complejo, el agente genera 3-5 hipótesis diferentes y las testea en paralelo (usando swarm R85). La mejor hipótesis gana.

## Tareas
### 1. Hypothesis generator
- Input: "Why is the server slow?"
- Output: 5 hypotheses ranked by prior probability:
  1. High CPU usage (40%) → test: check CPU
  2. Memory leak (25%) → test: check memory trend
  3. Disk I/O bottleneck (15%) → test: check disk usage
  4. Network congestion (10%) → test: check bandwidth
  5. Bad query (10%) → test: check slow queries

### 2. Parallel testing (swarm)
- Deploy 5 agents, one per hypothesis
- Each runs its test
- Results compiled: "Hypothesis 2 confirmed: memory growing 50MB/hour = memory leak"

### 3. Bayesian updating
- Start with prior probabilities
- As evidence comes in, update: "CPU normal → H1 drops to 5%. Memory growing → H2 rises to 80%"
- Show probability evolution in real-time

### 4. Frontend: hypothesis board
```
HYPOTHESIS BOARD: "Why is the server slow?"
─────────────────────────────────────────
H1: High CPU      ░░░░░ 5%  → CPU normal (eliminated)
H2: Memory leak   █████████ 80% → Memory growing 50MB/hr ← LIKELY
H3: Disk I/O      ███░░ 8%  → Disk OK (reduced)
H4: Network        ██░░ 4%  → Bandwidth normal (reduced)
H5: Bad query      ██░░ 3%  → No slow queries (reduced)

🏆 Most likely: Memory leak. Suggested action: restart service + investigate allocation.
```

## Demo
1. "Why is the server slow?" → 5 hypotheses generated → 5 agents test in parallel
2. Probability bars update in real-time as evidence comes in
3. "Memory leak confirmed (80%)" → suggested fix → execute fix → verify
