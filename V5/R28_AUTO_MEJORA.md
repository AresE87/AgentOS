# FASE R28 — AUTO-MEJORA: El agente se vuelve más inteligente con el uso

**Objetivo:** El routing table se optimiza solo basado en historial de éxito/costo/latencia. El usuario puede dar feedback (👍/👎) y el agente aprende. Reporte semanal automático.

---

## Tareas

### 1. Routing optimizer

```rust
// En brain/router.rs:

pub fn optimize_routing(history: &[LLMCallRecord]) -> RoutingTable {
    // Para cada (task_type, tier):
    //   Agrupar por modelo
    //   Score = 0.5 * success_rate + 0.3 * (1/normalized_cost) + 0.2 * (1/normalized_latency)
    //   Reordenar modelos por score
    //   Solo si hay ≥ 20 data points
    
    // Guardar como routing_optimized.json
    // El router carga routing_optimized.json si existe, sino el default
}

// Ejecutar cada 24 horas o manualmente desde Settings
```

### 2. Feedback widget (👍/👎)

```
En cada respuesta del Chat:
  [respuesta del agente]
  claude-sonnet · $0.003 · 1.2s    👍  👎

Si 👎: dialog "What went wrong?"
  - Wrong answer
  - Too slow
  - Wrong model (should have used cheaper/better)
  - Other: [text input]

Guardar en SQLite:
CREATE TABLE IF NOT EXISTS task_feedback (
    id          TEXT PRIMARY KEY,
    task_id     TEXT NOT NULL,
    rating      INTEGER NOT NULL,  -- 1 (thumbs down) or 5 (thumbs up)
    reason      TEXT,
    created_at  TEXT NOT NULL
);
```

### 3. Weekly insight report

```
Cada lunes a las 8am (o configurable):
1. Generar resumen de la semana
2. Enviar por Telegram/Discord (canal preferido)
3. Mostrar en Home como notificación

Contenido:
  📊 Weekly AgentOS Report (Mar 22-28)
  ─────────────────────────────
  Tasks completed: 156 (↑12% vs last week)
  Success rate: 94.2% (↑2.1%)
  Total cost: $4.56 (↓8%)
  Time saved: ~18.5 hours
  
  Top specialist: Code Reviewer (45 tasks)
  Most used model: claude-sonnet (67%)
  
  💡 Tip: You use Premium tier for simple tasks 23% of the time.
  Switching to Standard could save ~$1.20/week.
```

### 4. Frontend: feedback integrated

- 👍/👎 en cada mensaje del Chat
- Analytics page: sección "Routing Changes" que muestra cómo el optimizer mejoró la tabla
- Settings: "Auto-optimize routing" toggle + "Generate weekly report" toggle

---

## Demo

1. Usar la app por 20+ tareas → routing optimizer sugiere cambios
2. Dar 👎 a una respuesta → feedback dialog → se registra
3. Reporte semanal aparece en Home como notificación
4. Enviar por Telegram si está configurado
