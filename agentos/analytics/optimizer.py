"""Routing optimizer — auto-improves model selection based on history."""

from __future__ import annotations

from collections import defaultdict
from dataclasses import dataclass
from typing import TYPE_CHECKING

import yaml

if TYPE_CHECKING:
    from pathlib import Path

from agentos.utils.logging import get_logger

logger = get_logger("analytics.optimizer")

MIN_SAMPLES = 20  # Minimum tasks per combination to optimize


@dataclass
class ModelScore:
    model: str
    success_rate: float
    avg_cost: float
    avg_latency_ms: float
    sample_count: int
    score: float


class RoutingOptimizer:
    """Optimizes routing table based on historical performance."""

    def __init__(self, store=None) -> None:
        self._store = store

    async def optimize(self) -> dict | None:
        """Analyze history and produce optimized routing table."""
        if not self._store:
            return None

        tasks = await self._store.get_recent_tasks(limit=5000)
        if len(tasks) < MIN_SAMPLES:
            logger.info("Not enough data to optimize (%d tasks, need %d)", len(tasks), MIN_SAMPLES)
            return None

        # Group by (task_type, tier)
        groups: dict[tuple[str, int], list[dict]] = defaultdict(list)
        for t in tasks:
            tt = t.get("task_type", "text")
            tier = t.get("tier", 1) or 1
            groups[(tt, tier)].append(t)

        optimized_routing: dict[str, dict[int, list[str]]] = {}
        for (task_type, tier), group_tasks in groups.items():
            if len(group_tasks) < MIN_SAMPLES:
                continue

            scores = self._score_models(group_tasks)
            if scores:
                if task_type not in optimized_routing:
                    optimized_routing[task_type] = {}
                optimized_routing[task_type][tier] = [s.model for s in scores]

        if not optimized_routing:
            return None

        logger.info("Routing optimized for %d combinations", len(optimized_routing))
        return optimized_routing

    def _score_models(self, tasks: list[dict]) -> list[ModelScore]:
        """Score models based on success, cost, and latency."""
        model_stats: dict[str, dict] = defaultdict(
            lambda: {
                "success": 0,
                "total": 0,
                "cost": 0.0,
                "latency": 0.0,
            }
        )

        for t in tasks:
            model = t.get("model_used", "")
            if not model:
                continue
            stats = model_stats[model]
            stats["total"] += 1
            if t.get("status") == "completed":
                stats["success"] += 1
            stats["cost"] += t.get("cost_estimate", 0.0)
            stats["latency"] += t.get("duration_ms", 0.0)

        scores = []
        for model, stats in model_stats.items():
            if stats["total"] < 3:
                continue
            success_rate = stats["success"] / stats["total"]
            avg_cost = stats["cost"] / stats["total"] if stats["total"] else 0
            avg_latency = stats["latency"] / stats["total"] if stats["total"] else 0

            # Composite score
            cost_factor = 1.0 / (avg_cost + 0.001)
            latency_factor = 1.0 / (avg_latency + 100)
            score = success_rate * 0.5 + cost_factor * 0.3 + latency_factor * 0.2

            scores.append(
                ModelScore(
                    model=model,
                    success_rate=success_rate,
                    avg_cost=avg_cost,
                    avg_latency_ms=avg_latency,
                    sample_count=stats["total"],
                    score=score,
                )
            )

        scores.sort(key=lambda s: s.score, reverse=True)
        return scores

    async def save_optimized(self, routing: dict, output_path: Path) -> None:
        """Save optimized routing to YAML."""
        output_path.parent.mkdir(parents=True, exist_ok=True)
        with open(output_path, "w") as f:
            yaml.dump({"routing": routing}, f, default_flow_style=False)
        logger.info("Saved optimized routing to %s", output_path)
