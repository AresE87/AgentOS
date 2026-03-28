"""Tests for the routing optimizer (AOS-084)."""

from __future__ import annotations

from unittest.mock import AsyncMock

import pytest

from agentos.analytics.optimizer import RoutingOptimizer


@pytest.fixture
def mock_store():
    store = AsyncMock()
    store.get_recent_tasks = AsyncMock(return_value=[])
    return store


def _make_task(
    task_type="text",
    tier=1,
    model="gpt-4o-mini",
    status="completed",
    cost=0.01,
    duration_ms=500.0,
):
    return {
        "task_type": task_type,
        "tier": tier,
        "model_used": model,
        "status": status,
        "cost_estimate": cost,
        "duration_ms": duration_ms,
    }


@pytest.mark.asyncio
async def test_no_data(mock_store):
    """Not enough tasks returns None."""
    mock_store.get_recent_tasks.return_value = [_make_task() for _ in range(5)]
    optimizer = RoutingOptimizer(store=mock_store)
    result = await optimizer.optimize()
    assert result is None


@pytest.mark.asyncio
async def test_optimize_with_data(mock_store):
    """Sufficient tasks in one group produce optimized routing."""
    tasks = [_make_task(task_type="text", tier=1, model="gpt-4o-mini") for _ in range(25)]
    mock_store.get_recent_tasks.return_value = tasks

    optimizer = RoutingOptimizer(store=mock_store)
    result = await optimizer.optimize()

    assert result is not None
    assert "text" in result
    assert 1 in result["text"]
    assert "gpt-4o-mini" in result["text"][1]


@pytest.mark.asyncio
async def test_score_models(mock_store):
    """Models are scored correctly by success/cost/latency."""
    tasks = [
        _make_task(model="model-a", status="completed", cost=0.01, duration_ms=200),
        _make_task(model="model-a", status="completed", cost=0.01, duration_ms=200),
        _make_task(model="model-a", status="completed", cost=0.01, duration_ms=200),
        _make_task(model="model-b", status="completed", cost=0.10, duration_ms=1000),
        _make_task(model="model-b", status="failed", cost=0.10, duration_ms=1000),
        _make_task(model="model-b", status="failed", cost=0.10, duration_ms=1000),
    ]

    optimizer = RoutingOptimizer(store=mock_store)
    scores = optimizer._score_models(tasks)

    assert len(scores) == 2
    # model-a should score higher (100% success, lower cost, lower latency)
    assert scores[0].model == "model-a"
    assert scores[0].success_rate == pytest.approx(1.0)
    assert scores[1].model == "model-b"
    assert scores[1].success_rate == pytest.approx(1 / 3)


@pytest.mark.asyncio
async def test_min_samples(mock_store):
    """Below threshold per group is skipped even if total is enough."""
    # 25 tasks total but split across 5 different types = 5 each, below MIN_SAMPLES
    tasks = []
    for tt in ["text", "code", "vision", "generation", "data"]:
        tasks.extend([_make_task(task_type=tt, tier=1) for _ in range(5)])
    mock_store.get_recent_tasks.return_value = tasks

    optimizer = RoutingOptimizer(store=mock_store)
    result = await optimizer.optimize()

    assert result is None
