"""Task chain engine -- executes sub-tasks respecting dependencies."""

from __future__ import annotations

import asyncio
import enum
import uuid
from dataclasses import dataclass, field
from datetime import UTC, datetime
from typing import TYPE_CHECKING

from agentos.types import TaskInput, TaskResult, TaskStatus
from agentos.utils.logging import get_logger

if TYPE_CHECKING:
    from agentos.hierarchy.context import ChainContext
    from agentos.hierarchy.decomposer import SubTaskDefinition, TaskPlan

logger = get_logger("hierarchy.chain")


class ChainStatus(enum.StrEnum):
    """Status of a task chain execution."""

    PENDING = "pending"
    RUNNING = "running"
    COMPLETED = "completed"
    PARTIAL_FAILURE = "partial_failure"
    FAILED = "failed"


@dataclass
class TaskChain:
    """A chain of sub-tasks with shared context."""

    chain_id: str
    plan: TaskPlan
    context: ChainContext
    status: ChainStatus = ChainStatus.PENDING
    results: dict[str, TaskResult] = field(default_factory=dict)
    created_at: datetime = field(default_factory=lambda: datetime.now(UTC))


@dataclass
class ChainResult:
    """Aggregated result of executing a full task chain."""

    chain_id: str
    status: ChainStatus
    results: dict[str, TaskResult]
    combined_output: str
    total_cost: float
    total_duration_ms: float


class ChainExecutor:
    """Executes task chains with dependency resolution and parallel execution."""

    def __init__(self, process_fn, timeout: float = 600.0) -> None:
        """Initialize the chain executor.

        Args:
            process_fn: Async callable ``(TaskInput) -> TaskResult``.
            timeout: Global chain timeout in seconds.
        """
        self._process_fn = process_fn
        self._timeout = timeout

    async def execute(self, chain: TaskChain) -> ChainResult:
        """Execute a task chain, running independent subtasks in parallel."""
        chain.status = ChainStatus.RUNNING
        start = asyncio.get_event_loop().time()

        pending = {st.id for st in chain.plan.subtasks}
        completed: set[str] = set()
        failed: set[str] = set()

        try:
            while pending:
                elapsed = asyncio.get_event_loop().time() - start
                if elapsed > self._timeout:
                    logger.warning("Chain %s timed out after %.0fs", chain.chain_id, elapsed)
                    # Mark all remaining pending as failed
                    for sid in list(pending):
                        failed.add(sid)
                    pending.clear()
                    break

                # Find ready subtasks (all dependencies satisfied, none failed)
                subtask_map = {st.id: st for st in chain.plan.subtasks}
                ready = [
                    subtask_map[sid]
                    for sid in pending
                    if all(dep in completed for dep in subtask_map[sid].depends_on)
                    and not any(dep in failed for dep in subtask_map[sid].depends_on)
                ]

                if not ready:
                    # Mark blocked subtasks (dependencies failed)
                    blocked = [
                        sid
                        for sid in pending
                        if any(dep in failed for dep in subtask_map[sid].depends_on)
                    ]
                    for sid in blocked:
                        pending.discard(sid)
                        failed.add(sid)
                    if not ready and pending:
                        break  # Deadlock or all blocked

                # Execute ready subtasks in parallel
                tasks = [self._execute_subtask(st, chain) for st in ready]
                results = await asyncio.gather(*tasks, return_exceptions=True)

                for subtask, result in zip(ready, results, strict=True):
                    pending.discard(subtask.id)
                    if isinstance(result, Exception):
                        failed.add(subtask.id)
                        logger.warning("Subtask %s failed: %s", subtask.id, result)
                    elif result.status == TaskStatus.FAILED:
                        failed.add(subtask.id)
                    else:
                        completed.add(subtask.id)
                        chain.results[subtask.id] = result
                        chain.context.set(subtask.id, "output", result.output_text)

        except Exception:
            logger.exception("Chain execution error")

        return self._compile_result(chain, completed, failed, start)

    async def _execute_subtask(self, subtask: SubTaskDefinition, chain: TaskChain) -> TaskResult:
        """Execute a single subtask, injecting dependency outputs into the prompt."""
        dep_context = chain.context.get_dependency_outputs(subtask.id, subtask.depends_on)
        prompt = subtask.description
        if dep_context:
            prompt = f"Previous results:\n{dep_context}\n\nYour task: {subtask.description}"

        task_input = TaskInput(text=prompt, source="chain", task_id=uuid.uuid4().hex[:12])
        return await self._process_fn(task_input)

    def _compile_result(
        self,
        chain: TaskChain,
        completed: set[str],
        failed: set[str],
        start: float,
    ) -> ChainResult:
        """Compile individual subtask results into a single ChainResult."""
        elapsed = (asyncio.get_event_loop().time() - start) * 1000

        if not failed:
            status = ChainStatus.COMPLETED
        elif completed and failed:
            status = ChainStatus.PARTIAL_FAILURE
        else:
            status = ChainStatus.FAILED

        chain.status = status

        # Combine outputs in plan order
        outputs: list[str] = []
        for st in chain.plan.subtasks:
            if st.id in chain.results:
                outputs.append(f"**{st.description}:**\n{chain.results[st.id].output_text}")
            elif st.id in failed:
                outputs.append(f"**{st.description}:** Failed")

        total_cost = sum(r.cost_estimate for r in chain.results.values())

        return ChainResult(
            chain_id=chain.chain_id,
            status=status,
            results=chain.results,
            combined_output="\n\n".join(outputs),
            total_cost=total_cost,
            total_duration_ms=elapsed,
        )
