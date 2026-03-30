# FASE R38 — ANALYTICS AVANZADOS: Dashboards ejecutivos y ROI

**Objetivo:** Analytics que demuestren el valor de AgentOS con números: ROI, tiempo ahorrado, costo vs beneficio, comparativas entre períodos, y export a PDF para presentaciones.

---

## Tareas

### 1. ROI Calculator

```rust
// Calcular tiempo ahorrado:
// Para cada tarea, estimar cuánto tomaría hacerla manualmente
// - Simple (complexity 1-2): 5 min manual
// - Medium (complexity 3): 15 min manual
// - Complex (complexity 4-5): 45 min manual
// - Chain: sum of subtask estimates

// ROI = (tiempo_manual_estimado * hourly_rate) - costo_LLM
// hourly_rate: configurable en Settings (default $30/hr)

pub struct ROIReport {
    pub period: String,
    pub tasks_completed: usize,
    pub manual_time_hours: f64,
    pub agent_time_hours: f64,
    pub time_saved_hours: f64,
    pub llm_cost: f64,
    pub manual_cost_equivalent: f64,  // time_saved * hourly_rate
    pub roi_percentage: f64,          // (manual_cost - llm_cost) / llm_cost * 100
}
```

### 2. Comparativas entre períodos

```
THIS WEEK vs LAST WEEK
──────────────────────
Tasks:    156 → 189 (+21% ▲)
Success:  92% → 96% (+4% ▲)
Cost:     $4.56 → $3.89 (-15% ▼ ✅)
Saved:    18h → 24h (+33% ▲)
```

### 3. Charts mejorados

- **Stacked bar:** Tareas por tipo por día (text, code, vision, data stacked)
- **Heatmap:** Actividad por hora del día × día de la semana (cuándo se usa más)
- **Funnel:** Tasks requested → classified → executed → completed → successful
- **Trend line:** Costo acumulado con proyección al fin de mes

### 4. Export a PDF

```rust
// Generar PDF con charts + KPIs para presentaciones
// Crate: printpdf o generar HTML y convertir con headless browser

// Contenido del PDF:
// Page 1: KPIs + ROI
// Page 2: Tasks over time (chart)
// Page 3: Cost breakdown (chart)
// Page 4: Top specialists and playbooks (table)
// Page 5: Recommendations
```

### 5. Frontend: Analytics v2

```
ANALYTICS                    [This Week ▾] [vs Last Week ▾] [Export PDF]
────────────────────────────────────────────────────────────

ROI
┌────────────────────────────────────────────────────────┐
│ 💰 You saved $720 this week                            │
│    24 hours of manual work automated                   │
│    Agent cost: $3.89 · Manual equivalent: $720          │
│    ROI: 18,408% ← this number sells the product        │
└────────────────────────────────────────────────────────┘

[charts: stacked bar, heatmap, funnel, trend line]
```

---

## Demo

1. ROI card muestra: "$720 saved, 18,408% ROI" (con datos reales de uso)
2. Comparativa: this week vs last week con arrows ▲▼
3. Heatmap muestra cuándo se usa más (ej: lunes 9am)
4. Export PDF → se genera y se descarga
