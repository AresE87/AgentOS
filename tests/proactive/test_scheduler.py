"""Tests for the task scheduler (AOS-083)."""

from __future__ import annotations

from unittest.mock import AsyncMock

import pytest

from agentos.proactive.scheduler import ScheduledTask, TaskScheduler


def _make_scheduled_task(
    task_id="test-task",
    trigger_type="interval",
    schedule="60",
    task_text="do something",
    enabled=True,
):
    return ScheduledTask(
        id=task_id,
        trigger_type=trigger_type,
        schedule=schedule,
        task_text=task_text,
        enabled=enabled,
    )


@pytest.mark.asyncio
async def test_add_and_list():
    """Add task and verify it is listed."""
    scheduler = TaskScheduler()
    task = _make_scheduled_task()
    scheduler.add_task(task)

    listed = scheduler.list_tasks()
    assert len(listed) == 1
    assert listed[0].id == "test-task"


@pytest.mark.asyncio
async def test_remove_task():
    """Remove task and verify it is no longer listed."""
    scheduler = TaskScheduler()
    scheduler.add_task(_make_scheduled_task())

    removed = scheduler.remove_task("test-task")
    assert removed is True
    assert len(scheduler.list_tasks()) == 0

    # Removing non-existent returns False
    assert scheduler.remove_task("nonexistent") is False


@pytest.mark.asyncio
async def test_enable_disable():
    """Disable sets enabled=False, enable sets enabled=True."""
    scheduler = TaskScheduler()
    task = _make_scheduled_task(enabled=True)
    scheduler.add_task(task)

    scheduler.disable_task("test-task")
    assert scheduler.list_tasks()[0].enabled is False

    scheduler.enable_task("test-task")
    assert scheduler.list_tasks()[0].enabled is True


@pytest.mark.asyncio
async def test_trigger_manual():
    """Manual trigger calls process_fn with task text."""
    process_fn = AsyncMock()
    scheduler = TaskScheduler(process_fn=process_fn)
    scheduler.add_task(_make_scheduled_task(task_text="run backup"))

    result = await scheduler.trigger_manual("test-task")
    assert result is True
    process_fn.assert_awaited_once_with("run backup")


@pytest.mark.asyncio
async def test_trigger_manual_nonexistent():
    """Manual trigger on non-existent task returns False."""
    scheduler = TaskScheduler()
    result = await scheduler.trigger_manual("nonexistent")
    assert result is False


@pytest.mark.asyncio
async def test_webhook_trigger():
    """Matching webhook path executes the task."""
    process_fn = AsyncMock()
    scheduler = TaskScheduler(process_fn=process_fn)
    scheduler.add_task(
        _make_scheduled_task(
            task_id="webhook-task",
            trigger_type="webhook",
            schedule="/hooks/deploy",
            task_text="deploy app",
        )
    )

    result = await scheduler.handle_webhook_trigger("/hooks/deploy", {"event": "push"})
    assert result is True
    process_fn.assert_awaited_once_with("deploy app")


@pytest.mark.asyncio
async def test_webhook_no_match():
    """Non-matching webhook path returns False."""
    scheduler = TaskScheduler()
    scheduler.add_task(
        _make_scheduled_task(
            task_id="webhook-task",
            trigger_type="webhook",
            schedule="/hooks/deploy",
        )
    )

    result = await scheduler.handle_webhook_trigger("/hooks/other", {})
    assert result is False


@pytest.mark.asyncio
async def test_get_history():
    """After trigger, history has an entry."""
    process_fn = AsyncMock()
    scheduler = TaskScheduler(process_fn=process_fn)
    scheduler.add_task(_make_scheduled_task(task_text="backup db"))

    await scheduler.trigger_manual("test-task")
    history = scheduler.get_history()

    assert len(history) == 1
    assert history[0].trigger_id == "test-task"
    assert history[0].details["task_text"] == "backup db"
    assert history[0].details["run_count"] == 1
