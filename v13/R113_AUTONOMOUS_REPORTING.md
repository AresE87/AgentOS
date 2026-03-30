# FASE R113 — AUTONOMOUS REPORTING: Reportes que se generan solos

**Objetivo:** El agente genera y envía reportes periódicos sin que nadie lo pida: reporte semanal, monthly financials, daily standup, project status updates. Configurable por equipo.

---

## Tareas

### 1. Report scheduler
```rust
pub struct ReportSchedule {
    pub report_type: String,    // "weekly_summary", "monthly_financials", "daily_standup"
    pub schedule: String,       // Cron: "0 9 * * MON" (lunes 9am)
    pub recipients: Vec<String>, // Email addresses o canales
    pub template: String,       // Template ID (R58)
    pub data_sources: Vec<String>, // ["analytics", "calendar", "email", "db:sales"]
    pub format: String,         // "pdf", "email", "slack", "markdown"
}
```

### 2. Data aggregation across sources
```
Para un "Weekly Summary":
1. Analytics: tasks completed, cost, time saved (AgentOS internal)
2. Calendar: meetings attended, hours in meetings
3. Email: emails sent/received, response time avg
4. Database: sales numbers, KPIs del negocio
5. Git: commits, PRs merged, issues closed (si GitHub connected)

→ Todo se compila en un reporte coherente con charts y narrativa AI
```

### 3. Pre-built report types (5)
```
1. Weekly Team Summary — what the team accomplished, key metrics, blockers
2. Monthly Financial — revenue, costs, margins, forecast
3. Daily Standup — yesterday's activity, today's calendar, blockers
4. Project Status — tasks completed vs planned, timeline, risks
5. Client Report — work done for specific client, hours, deliverables
```

### 4. Smart narrative
```
El LLM no solo lista datos — ANALIZA y NARRA:
"This week was 15% more productive than last week, primarily due to
the new 'Invoice Processor' playbook that handled 34 invoices
automatically. However, the team's email response time increased
from 2.3h to 3.1h — worth investigating."
```

---

## Demo
1. Configure "Weekly Summary" → Monday 9am → email to team
2. Monday 9:01am → email arrives with charts, narrativa, y highlights
3. "Monthly Financial" → PDF generado con revenue chart + narrative + forecast
4. Daily standup → Slack message with yesterday + today + blockers
5. "Generate report NOW" → instant report with current data
