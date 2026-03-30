# FASE R124 — CAUSAL INFERENCE: El agente entiende causa y efecto

**Objetivo:** "¿Por qué bajaron las ventas?" → el agente no solo correlaciona, INFIERE causas. Analiza datos históricos, identifica cambios, y propone explicaciones causales rankeadas por probabilidad.

## Tareas
### 1. Causal analysis prompt engineering
- "Analyze this time series. Identify changes. For each change, propose causal explanations."
- Use: "Before X happened, the metric was Y. After X, it became Z. Possible causes: ..."

### 2. Counterfactual reasoning
- "If we hadn't raised prices in February, sales would likely be ~15% higher based on the trend"
- "If we had launched the marketing campaign 2 weeks earlier, we would have captured the holiday demand"

### 3. Intervention suggestions
- "To reverse the sales decline, I suggest: 1) Roll back the price increase (high confidence), 2) Increase marketing spend in the underperforming region (medium confidence), 3) Investigate competitor pricing (needs more data)"

### 4. Causal graph visualization
```
Price increase (Feb 15) ──causes──→ Sales decline (Feb 20+)
                                          │
Competitor launch (Feb 10) ──contributes──┘
                                          │
Seasonal pattern (Q1 dip) ──contributes───┘

Confidence: Price increase = 70%, Competitor = 20%, Seasonal = 10%
```

## Demo
1. "Why did sales drop in March?" → causal analysis with 3 ranked explanations
2. Counterfactual: "Without the price increase, sales would be X"
3. Visual causal graph showing relationships between events and outcomes
