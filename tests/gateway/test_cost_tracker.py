"""Tests for cost tracker."""

from __future__ import annotations

import pytest

from agentos.gateway.cost_tracker import CostTracker, SessionMetrics, load_price_table
from agentos.types import LLMResponse

# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

# Price table matching config/routing.yaml
PRICE_TABLE: dict[str, dict[str, float]] = {
    "claude-3-haiku-20240307": {"cost_per_1m_input": 0.25, "cost_per_1m_output": 1.25},
    "claude-3-5-sonnet-20241022": {"cost_per_1m_input": 3.00, "cost_per_1m_output": 15.00},
    "claude-3-opus-20240229": {"cost_per_1m_input": 15.00, "cost_per_1m_output": 75.00},
    "gpt-4o-mini": {"cost_per_1m_input": 0.15, "cost_per_1m_output": 0.60},
    "gpt-4o": {"cost_per_1m_input": 2.50, "cost_per_1m_output": 10.00},
    "gemini/gemini-1.5-flash": {"cost_per_1m_input": 0.10, "cost_per_1m_output": 0.40},
    "gemini/gemini-1.5-pro": {"cost_per_1m_input": 1.25, "cost_per_1m_output": 5.00},
}


def _make_response(
    model: str = "gpt-4o-mini",
    provider: str = "openai",
    tokens_in: int = 100,
    tokens_out: int = 50,
) -> LLMResponse:
    return LLMResponse(
        content="test",
        model=model,
        provider=provider,
        tokens_in=tokens_in,
        tokens_out=tokens_out,
        cost_estimate=0.0,  # cost_estimate is on the response but CostTracker calculates its own
        latency_ms=100.0,
    )


@pytest.fixture()
def tracker() -> CostTracker:
    return CostTracker(price_table=PRICE_TABLE)


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


@pytest.mark.asyncio()
async def test_record_haiku(tracker: CostTracker) -> None:
    """Record response with haiku model: cost = (100*0.25 + 200*1.25) / 1M."""
    resp = _make_response(
        model="claude-3-haiku-20240307",
        provider="anthropic",
        tokens_in=100,
        tokens_out=200,
    )
    await tracker.record(resp, task_id="task-1")

    expected = (100 * 0.25 + 200 * 1.25) / 1_000_000  # 0.000275
    summary = tracker.get_session_summary()
    assert summary.total_calls == 1
    assert summary.total_tokens_in == 100
    assert summary.total_tokens_out == 200
    assert abs(summary.total_cost - expected) < 1e-10


@pytest.mark.asyncio()
async def test_record_two_providers(tracker: CostTracker) -> None:
    """Record responses from two different providers — cost_by_provider has 2 entries."""
    resp_a = _make_response(
        model="claude-3-haiku-20240307",
        provider="anthropic",
        tokens_in=100,
        tokens_out=100,
    )
    resp_b = _make_response(
        model="gpt-4o-mini",
        provider="openai",
        tokens_in=200,
        tokens_out=200,
    )
    await tracker.record(resp_a, task_id="t1")
    await tracker.record(resp_b, task_id="t2")

    summary = tracker.get_session_summary()
    assert summary.total_calls == 2
    assert summary.calls_by_provider["anthropic"] == 1
    assert summary.calls_by_provider["openai"] == 1
    assert len(summary.calls_by_provider) == 2


def test_estimate_cost_gpt4o_mini(tracker: CostTracker) -> None:
    """Estimate for gpt-4o-mini with 1000 in, 4096 out."""
    cost = tracker.estimate_cost("gpt-4o-mini", 1000, 4096)
    expected = (1000 * 0.15 + 4096 * 0.60) / 1_000_000
    assert abs(cost - expected) < 1e-10


@pytest.mark.asyncio()
async def test_get_session_summary_after_records(tracker: CostTracker) -> None:
    """After 5 records, totals are correct."""
    for i in range(5):
        resp = _make_response(
            model="gpt-4o-mini",
            provider="openai",
            tokens_in=100,
            tokens_out=50,
        )
        await tracker.record(resp, task_id=f"t{i}")

    summary = tracker.get_session_summary()
    assert summary.total_calls == 5
    assert summary.total_tokens_in == 500
    assert summary.total_tokens_out == 250
    assert summary.success_rate == 1.0

    single_cost = (100 * 0.15 + 50 * 0.60) / 1_000_000
    assert abs(summary.total_cost - single_cost * 5) < 1e-10


def test_unknown_model(tracker: CostTracker) -> None:
    """Model not in price_table yields cost=0.0 without error."""
    cost = tracker.calculate_cost("nonexistent-model", 500, 500)
    assert cost == 0.0


@pytest.mark.asyncio()
async def test_no_store() -> None:
    """task_store=None works without persistence errors."""
    ct = CostTracker(price_table=PRICE_TABLE, task_store=None)
    resp = _make_response()
    await ct.record(resp, task_id="t1")
    summary = ct.get_session_summary()
    assert summary.total_calls == 1


def test_load_price_table() -> None:
    """Build price table from routing.yaml format."""
    routing_config = {
        "providers": {
            "anthropic": {
                "models": {
                    "haiku": {
                        "id": "claude-3-haiku-20240307",
                        "cost_per_1m_input": 0.25,
                        "cost_per_1m_output": 1.25,
                        "max_tokens": 4096,
                    },
                },
            },
            "openai": {
                "models": {
                    "gpt4o-mini": {
                        "id": "gpt-4o-mini",
                        "cost_per_1m_input": 0.15,
                        "cost_per_1m_output": 0.60,
                        "max_tokens": 4096,
                    },
                },
            },
        },
    }
    table = load_price_table(routing_config)
    assert "claude-3-haiku-20240307" in table
    assert "gpt-4o-mini" in table
    assert table["gpt-4o-mini"]["cost_per_1m_input"] == 0.15
    assert table["gpt-4o-mini"]["cost_per_1m_output"] == 0.60


@pytest.mark.asyncio()
async def test_reset(tracker: CostTracker) -> None:
    """After reset, metrics are zeroed."""
    resp = _make_response()
    await tracker.record(resp, task_id="t1")
    assert tracker.get_session_summary().total_calls == 1

    tracker.reset()
    summary = tracker.get_session_summary()
    assert summary.total_calls == 0
    assert summary.total_cost == 0.0
    assert summary.total_tokens_in == 0
    assert summary.total_tokens_out == 0
    assert summary.success_rate == 0.0


def test_session_metrics_defaults() -> None:
    """SessionMetrics initializes with expected defaults."""
    m = SessionMetrics()
    assert m.total_calls == 0
    assert m.total_cost == 0.0
    assert isinstance(m.cost_by_provider, dict)
    assert isinstance(m.calls_by_model, dict)
