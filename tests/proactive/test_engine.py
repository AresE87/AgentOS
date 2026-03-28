"""Tests for the proactive suggestions engine (AOS-082)."""

from __future__ import annotations

from unittest.mock import AsyncMock

import pytest

from agentos.proactive.engine import ProactiveEngine


@pytest.fixture
def mock_store():
    store = AsyncMock()
    store.get_recent_tasks = AsyncMock(return_value=[])
    return store


def _make_task(input_text="do something useful", tier=1, complexity=1):
    return {
        "input_text": input_text,
        "tier": tier,
        "complexity": complexity,
    }


@pytest.mark.asyncio
async def test_detect_recurring(mock_store):
    """3+ identical tasks generate a recurring suggestion."""
    tasks = [_make_task(input_text="check server status please") for _ in range(5)]
    # Need at least 5 total tasks
    mock_store.get_recent_tasks.return_value = tasks

    engine = ProactiveEngine(store=mock_store)
    suggestions = await engine.analyze()

    recurring = [s for s in suggestions if s.type == "recurring_task"]
    assert len(recurring) >= 1
    assert "recurring" in recurring[0].id


@pytest.mark.asyncio
async def test_detect_sequence(mock_store):
    """A->B repeated produces a sequence suggestion."""
    tasks = [
        _make_task(input_text="compile the project code"),
        _make_task(input_text="run the test suite now"),
        _make_task(input_text="compile the project code"),
        _make_task(input_text="run the test suite now"),
        _make_task(input_text="compile the project code"),
        _make_task(input_text="run the test suite now"),
    ]
    mock_store.get_recent_tasks.return_value = tasks

    engine = ProactiveEngine(store=mock_store)
    suggestions = await engine.analyze()

    sequence = [s for s in suggestions if s.type == "sequence"]
    assert len(sequence) >= 1


@pytest.mark.asyncio
async def test_detect_optimization(mock_store):
    """Tier 3 for simple tasks suggests cost optimization."""
    tasks = [_make_task(tier=3, complexity=1) for _ in range(6)]
    mock_store.get_recent_tasks.return_value = tasks

    engine = ProactiveEngine(store=mock_store)
    suggestions = await engine.analyze()

    opt = [s for s in suggestions if s.type == "optimization"]
    assert len(opt) == 1
    assert opt[0].id == "optimize_tier"


@pytest.mark.asyncio
async def test_max_suggestions(mock_store):
    """Many patterns are limited to max_suggestions."""
    # Create many recurring patterns
    tasks = []
    for i in range(4):
        for _ in range(5):
            tasks.append(_make_task(input_text=f"repeated task number {i} please"))
    # Also add optimization triggers
    tasks.extend([_make_task(tier=3, complexity=1) for _ in range(5)])
    mock_store.get_recent_tasks.return_value = tasks

    engine = ProactiveEngine(store=mock_store, max_suggestions=3)
    suggestions = await engine.analyze()

    assert len(suggestions) <= 3


@pytest.mark.asyncio
async def test_dismiss(mock_store):
    """Dismissed suggestion is not returned."""
    tasks = [_make_task(tier=3, complexity=1) for _ in range(6)]
    mock_store.get_recent_tasks.return_value = tasks

    engine = ProactiveEngine(store=mock_store)
    suggestions = await engine.analyze()
    assert len(suggestions) > 0

    engine.dismiss("optimize_tier")
    suggestions = await engine.analyze()
    opt = [s for s in suggestions if s.id == "optimize_tier"]
    assert len(opt) == 0


@pytest.mark.asyncio
async def test_no_data(mock_store):
    """Few tasks produce no suggestions."""
    mock_store.get_recent_tasks.return_value = [_make_task() for _ in range(3)]

    engine = ProactiveEngine(store=mock_store)
    suggestions = await engine.analyze()

    assert suggestions == []
