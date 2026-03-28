# Implementation Spec: AOS-007 — LLM Gateway — Rastreo de costos y medición de uso

**Ticket:** AOS-007
**Rol:** PM → Backend Developer (ticket directo, sin fase de planning)
**Input:** AOS-002 API Contract (LLMResponse, ModelConfig), AOS-006 Data Design (llm_usage table)
**Fecha:** Marzo 2026

---

## Objetivo

Agregar al LLM Gateway la capacidad de rastrear tokens consumidos, calcular costos estimados, y acumular métricas. Este módulo es el puente entre el Gateway (que genera datos de uso) y el TaskStore (que los persiste).

---

## Interface: CostTracker

```python
class CostTracker:
    """Rastrea costos y uso de LLM en tiempo real.

    Mantiene métricas acumuladas en memoria (para dashboards rápidos)
    y persiste cada llamada en TaskStore (para reportes históricos).

    Uso:
        tracker = CostTracker(price_table, task_store)
        await tracker.record(response, task_id)
        summary = tracker.get_session_summary()
        detailed = await tracker.get_period_summary("2026-03-01", "2026-03-31")
    """

    def __init__(self, price_table: dict, task_store: TaskStore | None = None) -> None:
        """
        Args:
            price_table: Dict cargado de routing.yaml con precios por modelo.
            task_store: Store para persistencia (None = solo in-memory).
        """
        ...

    async def record(self, response: LLMResponse, task_id: str, fallback_index: int = 0) -> None:
        """Registra una llamada completada al LLM.

        1. Calcula costo basado en tokens y precios del modelo.
        2. Actualiza contadores en memoria.
        3. Persiste en TaskStore si disponible.
        """
        ...

    def estimate_cost(self, model: str, estimated_input_tokens: int, max_output_tokens: int) -> float:
        """Estima el costo ANTES de hacer la llamada.

        Usado por el Gateway para verificar el límite de costo por tarea.
        Usa max_output_tokens como worst-case para el cálculo.
        """
        ...

    def get_session_summary(self) -> UsageSummary:
        """Resumen acumulado desde que se inició el agente (in-memory).

        Rápido — no toca la DB.
        """
        ...

    async def get_period_summary(self, start: str, end: str) -> UsageSummary:
        """Resumen de un período (consulta TaskStore).

        Args:
            start: ISO 8601 date string.
            end: ISO 8601 date string.
        """
        ...
```

---

## Tabla de precios

Se carga del campo `providers.*.models.*.cost_per_1m_input/output` de `config/routing.yaml`.

```python
# Formato interno del price_table:
{
    "claude-3-haiku-20240307": {
        "cost_per_1m_input": 0.25,
        "cost_per_1m_output": 1.25,
    },
    "gpt-4o-mini": {
        "cost_per_1m_input": 0.15,
        "cost_per_1m_output": 0.60,
    },
    # ...
}
```

### Fórmula de costo

```python
cost = (tokens_in * cost_per_1m_input / 1_000_000) + (tokens_out * cost_per_1m_output / 1_000_000)
```

---

## Métricas in-memory

El CostTracker mantiene contadores que se resetean al reiniciar el agente:

```python
@dataclass
class SessionMetrics:
    total_calls: int = 0
    successful_calls: int = 0
    failed_calls: int = 0
    total_tokens_in: int = 0
    total_tokens_out: int = 0
    total_cost: float = 0.0
    cost_by_provider: dict[str, float]   # defaultdict(float)
    cost_by_model: dict[str, float]      # defaultdict(float)
    calls_by_provider: dict[str, int]    # defaultdict(int)
    started_at: datetime                  # Cuando se creó el tracker
```

---

## Test cases

| # | Test | Expected |
|---|------|----------|
| 1 | Record una respuesta de haiku (100 in, 200 out) | cost = (100*0.25 + 200*1.25) / 1M = 0.000275 |
| 2 | Record dos respuestas de proveedores diferentes | cost_by_provider tiene 2 entries |
| 3 | estimate_cost para gpt-4o-mini con 1000 in, 4096 out | Calcula correctamente |
| 4 | get_session_summary después de 5 records | Totales correctos |
| 5 | Modelo desconocido (no está en price_table) | cost_estimate = 0.0, log warning |
| 6 | task_store=None — funciona sin persistencia | Solo in-memory, sin error |
