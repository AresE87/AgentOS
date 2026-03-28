"""Analytics engine — computes metrics from task history."""

from __future__ import annotations

from collections import Counter, defaultdict
from dataclasses import dataclass, field
from datetime import UTC, datetime, timedelta

from agentos.utils.logging import get_logger

logger = get_logger("analytics")


@dataclass
class AnalyticsReport:
    period_start: str
    period_end: str
    total_tasks: int = 0
    completed_tasks: int = 0
    failed_tasks: int = 0
    success_rate: float = 0.0
    total_tokens_in: int = 0
    total_tokens_out: int = 0
    total_cost: float = 0.0
    avg_latency_ms: float = 0.0
    tasks_by_type: dict[str, int] = field(default_factory=dict)
    cost_by_provider: dict[str, float] = field(default_factory=dict)
    cost_by_model: dict[str, float] = field(default_factory=dict)
    top_playbooks: list[tuple[str, int]] = field(default_factory=list)
    estimated_time_saved_minutes: float = 0.0

    def to_dict(self) -> dict:
        return {
            "period": {"start": self.period_start, "end": self.period_end},
            "tasks": {
                "total": self.total_tasks,
                "completed": self.completed_tasks,
                "failed": self.failed_tasks,
                "success_rate": self.success_rate,
            },
            "tokens": {"input": self.total_tokens_in, "output": self.total_tokens_out},
            "cost": {
                "total": self.total_cost,
                "by_provider": self.cost_by_provider,
                "by_model": self.cost_by_model,
            },
            "latency": {"avg_ms": self.avg_latency_ms},
            "distribution": {"by_type": self.tasks_by_type},
            "top_playbooks": self.top_playbooks,
            "time_saved_minutes": self.estimated_time_saved_minutes,
        }


# Estimated manual time per complexity level (minutes)
MANUAL_TIME_ESTIMATES = {1: 2, 2: 5, 3: 15, 4: 30, 5: 60}


class AnalyticsEngine:
    """Computes analytics from task store data."""

    def __init__(self, store=None) -> None:
        self._store = store
        self._cache: dict[str, tuple[AnalyticsReport, float]] = {}
        self._cache_ttl = 300.0  # 5 minutes

    async def compute(self, period: str = "today") -> AnalyticsReport:
        """Compute analytics for a period."""
        import time

        # Check cache
        cached = self._cache.get(period)
        if cached and (time.time() - cached[1]) < self._cache_ttl:
            return cached[0]

        start, end = self._resolve_period(period)
        report = await self._compute_report(start, end)
        self._cache[period] = (report, time.time())
        return report

    async def _compute_report(self, start: str, end: str) -> AnalyticsReport:
        report = AnalyticsReport(period_start=start, period_end=end)

        if not self._store:
            return report

        # Get tasks in period
        tasks = await self._store.get_recent_tasks(limit=10000)
        period_tasks = [
            t for t in tasks if t.get("created_at", "") >= start and t.get("created_at", "") < end
        ]

        if not period_tasks:
            return report

        report.total_tasks = len(period_tasks)
        type_counter: Counter = Counter()
        provider_cost: defaultdict = defaultdict(float)
        model_cost: defaultdict = defaultdict(float)
        latencies: list[float] = []
        time_saved = 0.0

        for t in period_tasks:
            status = t.get("status", "")
            if status == "completed":
                report.completed_tasks += 1
            elif status == "failed":
                report.failed_tasks += 1

            report.total_cost += t.get("cost_estimate", 0.0)
            report.total_tokens_in += t.get("tokens_in", 0)
            report.total_tokens_out += t.get("tokens_out", 0)

            if t.get("task_type"):
                type_counter[t["task_type"]] += 1
            if t.get("provider"):
                provider_cost[t["provider"]] += t.get("cost_estimate", 0.0)
            if t.get("model_used"):
                model_cost[t["model_used"]] += t.get("cost_estimate", 0.0)
            if t.get("duration_ms"):
                latencies.append(t["duration_ms"])

            complexity = t.get("complexity", 1) or 1
            time_saved += MANUAL_TIME_ESTIMATES.get(complexity, 5)

        report.success_rate = (
            report.completed_tasks / report.total_tasks if report.total_tasks else 0.0
        )
        report.avg_latency_ms = sum(latencies) / len(latencies) if latencies else 0.0
        report.tasks_by_type = dict(type_counter)
        report.cost_by_provider = dict(provider_cost)
        report.cost_by_model = dict(model_cost)
        report.estimated_time_saved_minutes = time_saved

        return report

    @staticmethod
    def _resolve_period(period: str) -> tuple[str, str]:
        now = datetime.now(UTC)
        if period == "today":
            start = now.replace(hour=0, minute=0, second=0, microsecond=0)
            end = now
        elif period == "this_week":
            start = now - timedelta(days=now.weekday())
            start = start.replace(hour=0, minute=0, second=0, microsecond=0)
            end = now
        elif period == "this_month":
            start = now.replace(day=1, hour=0, minute=0, second=0, microsecond=0)
            end = now
        elif period == "last_30_days":
            start = now - timedelta(days=30)
            end = now
        else:
            start = now - timedelta(days=7)
            end = now
        return start.isoformat(), end.isoformat()
