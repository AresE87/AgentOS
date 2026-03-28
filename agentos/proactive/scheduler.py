"""Scheduled tasks — cron triggers, file watchers, webhook triggers."""

from __future__ import annotations

import asyncio
import contextlib
from dataclasses import dataclass, field
from datetime import UTC, datetime

from agentos.utils.logging import get_logger

logger = get_logger("proactive.scheduler")


@dataclass
class ScheduledTask:
    id: str
    trigger_type: str  # "cron", "file_watch", "webhook", "interval"
    schedule: str  # cron expression or interval seconds
    task_text: str
    playbook: str | None = None
    enabled: bool = True
    last_run: datetime | None = None
    run_count: int = 0


@dataclass
class TriggerEvent:
    trigger_id: str
    timestamp: datetime
    details: dict = field(default_factory=dict)


class TaskScheduler:
    """Manages scheduled and triggered tasks."""

    def __init__(self, process_fn=None) -> None:
        self._process_fn = process_fn  # async (text) -> result
        self._tasks: dict[str, ScheduledTask] = {}
        self._running = False
        self._loop_task: asyncio.Task | None = None
        self._history: list[TriggerEvent] = []

    def add_task(self, task: ScheduledTask) -> None:
        self._tasks[task.id] = task
        logger.info("Scheduled task added: %s (%s)", task.id, task.trigger_type)

    def remove_task(self, task_id: str) -> bool:
        return self._tasks.pop(task_id, None) is not None

    def enable_task(self, task_id: str) -> None:
        if task_id in self._tasks:
            self._tasks[task_id].enabled = True

    def disable_task(self, task_id: str) -> None:
        if task_id in self._tasks:
            self._tasks[task_id].enabled = False

    def list_tasks(self) -> list[ScheduledTask]:
        return list(self._tasks.values())

    async def start(self) -> None:
        self._running = True
        self._loop_task = asyncio.create_task(self._check_loop())
        logger.info("Task scheduler started with %d tasks", len(self._tasks))

    async def stop(self) -> None:
        self._running = False
        if self._loop_task:
            self._loop_task.cancel()
            with contextlib.suppress(asyncio.CancelledError):
                await self._loop_task

    async def trigger_manual(self, task_id: str) -> bool:
        """Manually trigger a scheduled task."""
        task = self._tasks.get(task_id)
        if not task:
            return False
        await self._execute_task(task)
        return True

    async def handle_webhook_trigger(self, trigger_path: str, payload: dict) -> bool:
        """Handle an incoming webhook trigger."""
        for task in self._tasks.values():
            if task.trigger_type == "webhook" and task.schedule == trigger_path and task.enabled:
                await self._execute_task(task)
                return True
        return False

    async def _check_loop(self) -> None:
        """Main loop checking for tasks to execute."""
        while self._running:
            now = datetime.now(UTC)
            for task in self._tasks.values():
                if not task.enabled:
                    continue
                if task.trigger_type == "interval":
                    interval = int(task.schedule)
                    if not task.last_run or (now - task.last_run).total_seconds() >= interval:
                        await self._execute_task(task)
            await asyncio.sleep(10)  # Check every 10s

    async def _execute_task(self, task: ScheduledTask) -> None:
        logger.info("Executing scheduled task: %s", task.id)
        task.last_run = datetime.now(UTC)
        task.run_count += 1
        self._history.append(
            TriggerEvent(
                trigger_id=task.id,
                timestamp=task.last_run,
                details={"task_text": task.task_text, "run_count": task.run_count},
            )
        )
        if self._process_fn:
            try:
                await self._process_fn(task.task_text)
            except Exception:
                logger.exception("Scheduled task %s failed", task.id)

    def get_history(self, limit: int = 50) -> list[TriggerEvent]:
        return self._history[-limit:]
