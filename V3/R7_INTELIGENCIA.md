# FASE R7 — INTELIGENCIA: Analytics, sugerencias, auto-mejora

**Objetivo:** El dashboard tiene analytics reales con charts. El agente sugiere tareas recurrentes. El routing table se optimiza solo basado en historial.

**Prerequisito:** R3 (frontend conectado — hay datos en SQLite para analizar)

---

## Tareas

### 1. Backend: Analytics engine

Crear queries en `memory/` que computen métricas reales desde SQLite:

```rust
#[tauri::command]
async fn get_analytics(period: String) -> Result<AnalyticsReport, String> {
    // period: "today", "this_week", "this_month"
    // Queries sobre tasks, llm_calls:
    // - total_tasks, completed, failed
    // - success_rate
    // - total_tokens_in + tokens_out
    // - total_cost (sum de llm_calls.cost)
    // - avg_latency
    // - tasks_by_type (group by task_type)
    // - cost_by_provider (group by provider)
    // - cost_by_model (group by model)
    // - top_agents (group by agent_name, count)
}
```

### 2. Frontend: Página Analytics

Sección en sidebar posición 6. Contenido:

- **Period selector:** Today | This Week | This Month
- **4 KPI cards:** Tasks totales, Success rate, Costo total, Tiempo estimado ahorrado
- **Line chart:** Tareas por día (últimos 7 o 30 días)
- **Bar chart:** Costo por proveedor (Anthropic vs OpenAI vs Google)
- **Pie chart:** Distribución por tipo de tarea
- **Tabla:** Top 5 agentes/especialistas usados con métricas

Usar `recharts` para los charts:
```bash
cd frontend && npm install recharts
```

### 3. Backend: Proactive suggestions (simple v1)

```rust
#[tauri::command]
async fn get_suggestions() -> Result<Vec<Suggestion>, String> {
    // Analizar patrones en tasks:
    // 1. Tarea recurrente: misma tarea (fuzzy match en input) 
    //    ejecutada ≥3 veces en los últimos 7 días → sugerir automatizar
    // 2. Cost optimization: si el user usa tier 3 para tareas de complexity 1-2
    //    → sugerir bajar el tier
    // 3. Unused playbook: playbook instalado pero nunca ejecutado → sugerir probarlo
}
```

### 4. Frontend: Banner de sugerencias en Home

Debajo de las stat cards, ANTES de Recent Tasks:

```
💡 You've run "check disk space" 5 times this week.
   Want me to do this automatically every morning?
   [Yes, automate] [Maybe later] [✕]
```

Máximo 2 sugerencias visibles. Dismissible.

### 5. Backend: Routing optimizer (simple v1)

```rust
// En brain/router.rs:
fn optimize_routing_table(history: &[LLMCall]) -> RoutingTable {
    // Para cada (task_type, tier):
    //   - Agrupar calls por modelo
    //   - Calcular: success_rate, avg_cost, avg_latency por modelo
    //   - Score = 0.5*success_rate + 0.3*(1/cost_normalized) + 0.2*(1/latency_normalized)
    //   - Reordenar modelos por score (mejor primero)
    //   - Solo actuar si hay ≥10 data points
    // Guardar como routing_optimized.json
}
```

Ejecutar la optimización cada 24 horas o cuando el usuario pide "optimize" en Settings.

---

## Cómo verificar

1. Usar la app por 10+ tareas → ir a Analytics → charts con datos reales
2. Repetir la misma tarea 3+ veces → sugerencia aparece en Home
3. Después de 20+ tareas → routing optimizer sugiere cambios (visible en logs)

---

## NO hacer

- No implementar scheduled tasks / cron (complejidad alta, bajo impacto ahora)
- No implementar file watchers
- No agregar ML classifier (las reglas funcionan suficiente)
