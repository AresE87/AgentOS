# FASE R76 — AGENT ANALYTICS PRO: Funnel, cohort, predicción de costos

**Objetivo:** Analytics avanzados que demuestran el valor de AgentOS con datos concretos: funnel de tareas, retención por cohorte, predicción de costos futuros, y benchmarks contra el equipo.

---

## Tareas

### 1. Task funnel

```
TASK FUNNEL (this month)
──────────────────────
Requested:    1,247  ████████████████████████████████ 100%
Classified:   1,245  ███████████████████████████████▌  99.8%
Executed:     1,198  ██████████████████████████████    96.1%
Completed:    1,156  █████████████████████████████     92.7%
Successful:   1,089  ████████████████████████████      87.3%

Drop-offs:
  Requested → Classified: 2 failed (parse error)
  Classified → Executed: 47 skipped (plan limit reached)
  Executed → Completed: 42 timed out
  Completed → Successful: 67 got negative feedback
```

### 2. Cohort retention

```
WEEKLY RETENTION
────────────────────────────────
Users who signed up in:     W1    W2    W3    W4
  Week of Mar 1             100%  78%   62%   55%
  Week of Mar 8             100%  81%   65%   -
  Week of Mar 15            100%  74%   -     -
  Week of Mar 22            100%  -     -     -

"Users" = unique task senders per week
Retention = sent at least 1 task that week
```

### 3. Predictive cost modeling

```
COST FORECAST
─────────────
Current monthly cost: $45.67
Current trend: +12% month-over-month

Predicted next month: $51.15 (±$8.20)
Predicted next quarter: $172.40

💡 Optimization opportunities:
  - Switch 23% of Tier 3 tasks to Tier 2 → save $8.40/month
  - Use local models for simple tasks → save $5.20/month
  - Batch similar tasks → save $3.10/month via caching
  
  Total potential savings: $16.70/month (36%)
  [Apply optimizations]
```

### 4. Model comparison

```
MODEL PERFORMANCE
────────────────────────────────────────────────
              Success   Avg Cost   Avg Latency   Score
claude-sonnet   96.2%   $0.012     1.8s          94
gpt-4o          93.8%   $0.015     2.1s          88
gpt-4o-mini     89.1%   $0.001     0.6s          85
gemini-flash    87.5%   $0.0005    0.4s          82
local/llama3    78.3%   $0.000     1.2s          71

[Auto-optimize routing based on these scores]
```

### 5. Frontend: Analytics Pro page

```
ANALYTICS PRO                    [This Month ▾] [Export PDF]
────────────────────────────────────────────────────────

[Task Funnel]  [Retention]  [Cost Forecast]  [Model Performance]

Each tab shows the corresponding visualization.
Charts use Design System v2 palette.
Export generates professional PDF report.
```

### 6. IPC commands

```rust
#[tauri::command] async fn analytics_funnel(period: String) -> Result<FunnelData, String>
#[tauri::command] async fn analytics_retention(weeks: usize) -> Result<RetentionData, String>
#[tauri::command] async fn analytics_cost_forecast(months: usize) -> Result<CostForecast, String>
#[tauri::command] async fn analytics_model_comparison() -> Result<Vec<ModelScore>, String>
#[tauri::command] async fn analytics_export_pdf(period: String) -> Result<String, String>
```

---

## Demo

1. Task funnel: ver dónde se pierden tareas (classified → executed drop = plan limits)
2. Retention: ver que 55% de users siguen activos después de 4 semanas
3. Cost forecast: "Next month ~$51" con opportunities de savings
4. Model comparison: claude-sonnet tiene mejor score pero gpt-4o-mini es 12x más barato
5. Export PDF: reporte profesional para mostrar ROI a management
