"""Tests for the analytics engine (AOS-080)."""

from __future__ import annotations

from datetime import UTC, datetime
from unittest.mock import AsyncMock

import pytest

from agentos.analytics.engine import AnalyticsEngine, AnalyticsReport


@pytest.fixture
def mock_store():
    store = AsyncMock()
    store.get_recent_tasks = AsyncMock(return_value=[])
    return store


def _make_task(
    status="completed",
    cost=0.01,
    tokens_in=100,
    tokens_out=50,
    task_type="text",
    provider="openai",
    model="gpt-4o-mini",
    complexity=1,
    tier=1,
    duration_ms=500.0,
    created_at=None,
):
    if created_at is None:
        created_at = datetime.now(UTC).isoformat()
    return {
        "status": status,
        "cost_estimate": cost,
        "tokens_in": tokens_in,
        "tokens_out": tokens_out,
        "task_type": task_type,
        "provider": provider,
        "model_used": model,
        "complexity": complexity,
        "tier": tier,
        "duration_ms": duration_ms,
        "created_at": created_at,
    }


@pytest.mark.asyncio
async def test_compute_empty():
    """No store returns empty report."""
    engine = AnalyticsEngine(store=None)
    report = await engine.compute("today")
    assert isinstance(report, AnalyticsReport)
    assert report.total_tasks == 0


@pytest.mark.asyncio
async def test_compute_with_tasks(mock_store):
    """Insert mock tasks and verify correct totals."""
    now = datetime.now(UTC).isoformat()
    tasks = [_make_task(created_at=now) for _ in range(5)]
    mock_store.get_recent_tasks.return_value = tasks

    engine = AnalyticsEngine(store=mock_store)
    report = await engine.compute("today")

    assert report.total_tasks == 5
    assert report.completed_tasks == 5
    assert report.total_cost == pytest.approx(0.05)
    assert report.total_tokens_in == 500
    assert report.total_tokens_out == 250


@pytest.mark.asyncio
async def test_success_rate(mock_store):
    """3 completed + 1 failed = 75% success rate."""
    now = datetime.now(UTC).isoformat()
    tasks = [
        _make_task(status="completed", created_at=now),
        _make_task(status="completed", created_at=now),
        _make_task(status="completed", created_at=now),
        _make_task(status="failed", created_at=now),
    ]
    mock_store.get_recent_tasks.return_value = tasks

    engine = AnalyticsEngine(store=mock_store)
    report = await engine.compute("today")

    assert report.success_rate == pytest.approx(0.75)
    assert report.completed_tasks == 3
    assert report.failed_tasks == 1


@pytest.mark.asyncio
async def test_cost_by_provider(mock_store):
    """Two providers have costs split correctly."""
    now = datetime.now(UTC).isoformat()
    tasks = [
        _make_task(provider="openai", cost=0.10, created_at=now),
        _make_task(provider="openai", cost=0.20, created_at=now),
        _make_task(provider="anthropic", cost=0.05, created_at=now),
    ]
    mock_store.get_recent_tasks.return_value = tasks

    engine = AnalyticsEngine(store=mock_store)
    report = await engine.compute("today")

    assert report.cost_by_provider["openai"] == pytest.approx(0.30)
    assert report.cost_by_provider["anthropic"] == pytest.approx(0.05)


@pytest.mark.asyncio
async def test_time_saved(mock_store):
    """Tasks with complexity map to estimated time saved."""
    now = datetime.now(UTC).isoformat()
    tasks = [
        _make_task(complexity=1, created_at=now),  # 2 min
        _make_task(complexity=3, created_at=now),  # 15 min
        _make_task(complexity=5, created_at=now),  # 60 min
    ]
    mock_store.get_recent_tasks.return_value = tasks

    engine = AnalyticsEngine(store=mock_store)
    report = await engine.compute("today")

    assert report.estimated_time_saved_minutes == pytest.approx(77.0)


def test_resolve_period_today():
    """_resolve_period('today') returns start-of-day and now."""
    start, end = AnalyticsEngine._resolve_period("today")
    start_dt = datetime.fromisoformat(start)
    end_dt = datetime.fromisoformat(end)
    assert start_dt.hour == 0
    assert start_dt.minute == 0
    assert end_dt > start_dt


@pytest.mark.asyncio
async def test_cache_hit(mock_store):
    """Second call within TTL returns cached report."""
    now = datetime.now(UTC).isoformat()
    mock_store.get_recent_tasks.return_value = [_make_task(created_at=now)]

    engine = AnalyticsEngine(store=mock_store)
    report1 = await engine.compute("today")
    report2 = await engine.compute("today")

    assert report1 is report2
    # get_recent_tasks should only be called once due to caching
    assert mock_store.get_recent_tasks.call_count == 1


@pytest.mark.asyncio
async def test_to_dict(mock_store):
    """to_dict produces expected structure."""
    now = datetime.now(UTC).isoformat()
    mock_store.get_recent_tasks.return_value = [_make_task(created_at=now)]

    engine = AnalyticsEngine(store=mock_store)
    report = await engine.compute("today")
    d = report.to_dict()

    assert "period" in d
    assert "tasks" in d
    assert "cost" in d
    assert d["tasks"]["total"] == 1
