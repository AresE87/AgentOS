# Architecture: AOS-080 a AOS-088 — Analytics, Proactividad, Auto-mejora

**Fecha:** Marzo 2026

---

## Analytics Engine

```python
@dataclass
class AnalyticsReport:
    period: str                         # "2026-03-01 to 2026-03-07"
    total_tasks: int
    completed: int
    failed: int
    success_rate: float                 # 0.0-1.0
    total_tokens_in: int
    total_tokens_out: int
    total_cost: float
    avg_latency_ms: float
    tasks_by_type: dict[str, int]       # {"text": 45, "code": 23, ...}
    cost_by_provider: dict[str, float]
    cost_by_model: dict[str, float]
    top_playbooks: list[tuple[str, int]]  # [(name, count), ...]
    top_specialists: list[tuple[str, int]]
    estimated_time_saved_hours: float   # complexity-based estimate
```

## Proactive Engine

```python
class ProactiveEngine:
    """Analiza patrones y genera sugerencias."""

    async def analyze(self) -> list[Suggestion]:
        """Analiza TaskStore y retorna sugerencias.

        Detección de patrones:
        1. Recurrent: misma tarea (fuzzy match) en misma hora ± 1h, misma day-of-week, ≥ 3 veces
        2. Sequence: mismas 2-3 tareas consecutivas ≥ 3 veces
        3. Incomplete: tarea empezada pero status != completed, < 24h ago
        4. Maintenance: última ejecución de playbook "system" > 7 días
        5. Cost optimization: tasks con tier > necessary (success rate igual en tier más bajo)
        """
        ...

@dataclass
class Suggestion:
    id: str
    type: str                   # "recurrent", "sequence", "incomplete", "maintenance", "cost"
    title: str                  # "Automate your weekly disk check?"
    description: str
    action: str                 # "create_schedule", "create_playbook", "continue_task", "run_task", "change_tier"
    action_params: dict
    priority: int               # 1-5
    dismissed: bool = False
```

## Routing Optimizer

```python
class RoutingOptimizer:
    """Auto-optimiza la routing table basada en historial."""

    def optimize(self, history: list[LLMUsageRecord], current_table: dict) -> dict:
        """Genera nueva routing table optimizada.

        Para cada (task_type, tier):
            - Obtener últimas 100 ejecuciones
            - Score cada modelo: 0.5*success_rate + 0.3*(1/cost) + 0.2*(1/latency)
            - Reordenar por score descendente
            - Solo actuar si ≥ 20 data points

        Returns: nueva routing table (mismo formato que routing.yaml)
        """
        ...

    def should_optimize(self, last_optimization: datetime) -> bool:
        """True si han pasado ≥ 24h desde la última optimización."""
        ...
```

## Scheduled Tasks (triggers.yaml)

```python
class TriggerEngine:
    """Ejecuta tareas basadas en triggers."""

    async def start(self) -> None:
        """Inicia schedulers y watchers."""
        ...

    async def stop(self) -> None: ...

    async def _handle_cron(self, trigger: CronTrigger) -> None:
        """Ejecuta tarea en el cron schedule."""
        ...

    async def _handle_file_watch(self, trigger: FileWatchTrigger) -> None:
        """Ejecuta tarea cuando se detecta evento de archivo."""
        ...

    async def _handle_webhook(self, trigger: WebhookTrigger, request: dict) -> None:
        """Ejecuta tarea cuando se recibe un webhook."""
        ...
```

## Dependencias nuevas

```
apscheduler >= 3.10     # Cron scheduler
watchdog >= 4.0         # File system watcher
```
