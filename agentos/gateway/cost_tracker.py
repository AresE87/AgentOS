"""Cost tracking and usage metering for the LLM Gateway.

Tracks tokens consumed, calculates costs per call, and maintains
accumulated usage metrics. Feeds data to TaskStore for persistence.
"""

from __future__ import annotations

from collections import defaultdict
from dataclasses import dataclass, field
from datetime import UTC, datetime

from agentos.types import LLMResponse, UsageSummary
from agentos.utils.logging import get_logger

logger = get_logger("gateway.cost_tracker")


@dataclass
class SessionMetrics:
    """In-memory metrics accumulated since agent start."""

    total_calls: int = 0
    successful_calls: int = 0
    failed_calls: int = 0
    total_tokens_in: int = 0
    total_tokens_out: int = 0
    total_cost: float = 0.0
    cost_by_provider: dict[str, float] = field(
        default_factory=lambda: defaultdict(float),
    )
    cost_by_model: dict[str, float] = field(
        default_factory=lambda: defaultdict(float),
    )
    calls_by_provider: dict[str, int] = field(
        default_factory=lambda: defaultdict(int),
    )
    calls_by_model: dict[str, int] = field(
        default_factory=lambda: defaultdict(int),
    )
    started_at: datetime = field(default_factory=lambda: datetime.now(UTC))


class CostTracker:
    """Tracks LLM usage costs and token consumption.

    Takes a price_table dict (loaded from routing.yaml) and optional
    TaskStore for persistence. Maintains in-memory SessionMetrics that
    can be queried for summaries.
    """

    def __init__(
        self,
        price_table: dict[str, dict[str, float]],
        task_store: object | None = None,
    ) -> None:
        # price_table: {"model_id": {"cost_per_1m_input": X, "cost_per_1m_output": Y}}
        self._prices = price_table
        self._store = task_store
        self._metrics = SessionMetrics()

    def calculate_cost(self, model: str, tokens_in: int, tokens_out: int) -> float:
        """Calculate cost for a completed call."""
        prices = self._prices.get(model)
        if not prices:
            logger.warning("Unknown model %s — cost set to $0.00", model)
            return 0.0
        input_cost = (tokens_in * prices["cost_per_1m_input"]) / 1_000_000
        output_cost = (tokens_out * prices["cost_per_1m_output"]) / 1_000_000
        return input_cost + output_cost

    async def record(
        self,
        response: LLMResponse,
        task_id: str,
        fallback_index: int = 0,
    ) -> None:
        """Record a completed LLM call.

        Updates in-memory metrics and persists to store.
        """
        cost = self.calculate_cost(response.model, response.tokens_in, response.tokens_out)

        # Update in-memory metrics
        self._metrics.total_calls += 1
        self._metrics.successful_calls += 1  # record() is called on success
        self._metrics.total_tokens_in += response.tokens_in
        self._metrics.total_tokens_out += response.tokens_out
        self._metrics.total_cost += cost
        self._metrics.cost_by_provider[response.provider] += cost
        self._metrics.cost_by_model[response.model] += cost
        self._metrics.calls_by_provider[response.provider] += 1
        self._metrics.calls_by_model[response.model] += 1

        # Persist if store available
        if self._store:
            await self._store.save_llm_usage(task_id, response, fallback_index)

        logger.info(
            "Usage: model=%s tokens=%d+%d cost=$%.6f",
            response.model,
            response.tokens_in,
            response.tokens_out,
            cost,
        )

    def estimate_cost(
        self,
        model: str,
        estimated_input_tokens: int,
        max_output_tokens: int,
    ) -> float:
        """Estimate cost BEFORE making a call (worst-case with max_output_tokens)."""
        return self.calculate_cost(model, estimated_input_tokens, max_output_tokens)

    def get_session_summary(self) -> UsageSummary:
        """Get in-memory summary (fast, no DB)."""
        m = self._metrics
        return UsageSummary(
            total_tokens_in=m.total_tokens_in,
            total_tokens_out=m.total_tokens_out,
            total_cost=m.total_cost,
            total_calls=m.total_calls,
            calls_by_provider=dict(m.calls_by_provider),
            calls_by_model=dict(m.calls_by_model),
            success_rate=m.successful_calls / m.total_calls if m.total_calls > 0 else 0.0,
        )

    async def get_period_summary(self, start: str, end: str) -> UsageSummary | None:
        """Get summary from TaskStore for a date range."""
        if not self._store:
            return None
        return await self._store.get_cost_by_period(start, end)

    def reset(self) -> None:
        """Reset all accumulated metrics."""
        self._metrics = SessionMetrics()


def load_price_table(routing_config: dict) -> dict[str, dict[str, float]]:
    """Extract price table from routing.yaml providers section."""
    table: dict[str, dict[str, float]] = {}
    for provider_data in routing_config.get("providers", {}).values():
        for model_data in provider_data.get("models", {}).values():
            table[model_data["id"]] = {
                "cost_per_1m_input": model_data["cost_per_1m_input"],
                "cost_per_1m_output": model_data["cost_per_1m_output"],
            }
    return table
