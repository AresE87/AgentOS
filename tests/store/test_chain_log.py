"""Tests for chain_log table and related TaskStore methods."""

from __future__ import annotations

import asyncio

import pytest

from agentos.store.task_store import TaskStore


@pytest.fixture
async def store() -> TaskStore:  # noqa: RUF029
    """Provide an initialised in-memory TaskStore."""
    s = TaskStore(db_path=":memory:")
    await s.initialize()
    try:
        yield s  # type: ignore[misc]
    finally:
        await s.close()


# ── save_chain_log / get_chain_log ──────────────────────────────


@pytest.mark.asyncio
async def test_save_and_get_chain_log(store: TaskStore) -> None:
    """Saving log entries and retrieving them returns correct data."""
    chain_id = "chain_test_001"

    id1 = await store.save_chain_log(
        chain_id=chain_id,
        agent_name="Orchestrator",
        agent_level="orchestrator",
        event_type="info",
        message="Decomposed into 3 sub-tasks",
    )
    id2 = await store.save_chain_log(
        chain_id=chain_id,
        agent_name="Worker-1",
        agent_level="senior",
        event_type="progress",
        message="Started research",
        metadata='{"progress": 0.5}',
    )

    assert id1 != id2  # unique IDs

    logs = await store.get_chain_log(chain_id)
    assert len(logs) == 2
    assert logs[0]["agent_name"] == "Orchestrator"
    assert logs[0]["event_type"] == "info"
    assert logs[0]["message"] == "Decomposed into 3 sub-tasks"
    assert logs[0]["metadata"] is None

    assert logs[1]["agent_name"] == "Worker-1"
    assert logs[1]["metadata"] == '{"progress": 0.5}'


# ── get_chain_history ────────────────────────────────────────────


@pytest.mark.asyncio
async def test_get_chain_history(store: TaskStore) -> None:
    """Chain history returns chains ordered by earliest timestamp descending."""
    # Insert logs for two different chains
    await store.save_chain_log(
        chain_id="chain_a",
        agent_name="Orchestrator",
        agent_level="orchestrator",
        event_type="info",
        message="Chain A started",
    )
    # Small delay so timestamps differ
    await asyncio.sleep(0.01)
    await store.save_chain_log(
        chain_id="chain_b",
        agent_name="Orchestrator",
        agent_level="orchestrator",
        event_type="status",
        message="completed",
    )
    await store.save_chain_log(
        chain_id="chain_b",
        agent_name="Worker",
        agent_level="junior",
        event_type="info",
        message="Did work",
    )

    history = await store.get_chain_history(limit=10)
    assert len(history) == 2
    # Most recent chain first
    assert history[0]["chain_id"] == "chain_b"
    assert history[0]["log_count"] == 2
    assert history[0]["last_status"] == "completed"

    assert history[1]["chain_id"] == "chain_a"
    assert history[1]["log_count"] == 1


# ── ordering ─────────────────────────────────────────────────────


@pytest.mark.asyncio
async def test_chain_log_ordering(store: TaskStore) -> None:
    """Log entries are returned ordered by timestamp ascending."""
    chain_id = "chain_order"

    await store.save_chain_log(
        chain_id=chain_id,
        agent_name="First",
        agent_level="orchestrator",
        event_type="info",
        message="msg1",
    )
    await asyncio.sleep(0.01)
    await store.save_chain_log(
        chain_id=chain_id,
        agent_name="Second",
        agent_level="senior",
        event_type="info",
        message="msg2",
    )
    await asyncio.sleep(0.01)
    await store.save_chain_log(
        chain_id=chain_id,
        agent_name="Third",
        agent_level="junior",
        event_type="info",
        message="msg3",
    )

    logs = await store.get_chain_log(chain_id)
    assert len(logs) == 3
    assert logs[0]["agent_name"] == "First"
    assert logs[1]["agent_name"] == "Second"
    assert logs[2]["agent_name"] == "Third"

    # Timestamps should be ascending
    assert logs[0]["timestamp"] <= logs[1]["timestamp"] <= logs[2]["timestamp"]
