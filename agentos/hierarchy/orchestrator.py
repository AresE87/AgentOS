"""Orchestrator -- the meta-agent that coordinates everything."""

from __future__ import annotations

import time
import uuid
from datetime import UTC, datetime
from typing import TYPE_CHECKING

from agentos.hierarchy.chain import ChainExecutor, TaskChain
from agentos.hierarchy.context import ChainContext
from agentos.hierarchy.levels import AgentLevel
from agentos.types import TaskClassification, TaskInput, TaskResult, TaskStatus
from agentos.utils.logging import get_logger

if TYPE_CHECKING:
    from agentos.hierarchy.decomposer import TaskDecomposer
    from agentos.hierarchy.specialists import SpecialistRegistry

logger = get_logger("hierarchy.orchestrator")


class Orchestrator:
    """Meta-agent that coordinates the multi-agent system.

    Receives a task, classifies it, and either handles it directly (simple)
    or decomposes it into sub-tasks and runs them through a chain (complex).
    """

    def __init__(
        self,
        process_fn,  # async (TaskInput) -> TaskResult
        classifier=None,
        decomposer: TaskDecomposer | None = None,
        specialist_registry: SpecialistRegistry | None = None,
        chain_executor: ChainExecutor | None = None,
    ) -> None:
        self._process_fn = process_fn
        self._classifier = classifier
        self._decomposer = decomposer
        self._registry = specialist_registry
        self._chain_executor = chain_executor

    async def process(self, task_input: TaskInput) -> TaskResult:
        """Main entry point. Never raises exceptions."""
        start = time.monotonic()
        try:
            # 1. Classify
            classification = None
            if self._classifier:
                classification = await self._classifier.classify(task_input)
                logger.info(
                    "Orchestrator: task classified as %s/%d",
                    classification.task_type.value,
                    classification.complexity,
                )

            # 2. Decide: direct or decompose?
            if (
                classification
                and self._decomposer
                and self._decomposer.should_decompose(classification)
            ):
                return await self._handle_complex(task_input, classification, start)
            return await self._handle_simple(task_input, classification, start)

        except Exception as exc:
            elapsed = (time.monotonic() - start) * 1000
            logger.exception("Orchestrator error")
            return TaskResult(
                task_id=task_input.task_id,
                input_text=task_input.text,
                source=task_input.source,
                status=TaskStatus.FAILED,
                error_message=str(exc),
                duration_ms=elapsed,
                created_at=task_input.created_at,
                completed_at=datetime.now(UTC),
            )

    async def _handle_simple(
        self,
        task_input: TaskInput,
        classification: TaskClassification | None,
        start: float,
    ) -> TaskResult:
        """Direct execution for simple tasks."""
        level = self._select_level(classification) if classification else AgentLevel.JUNIOR
        logger.info("Simple task -> %s agent", level.value)
        return await self._process_fn(task_input)

    async def _handle_complex(
        self,
        task_input: TaskInput,
        classification: TaskClassification,
        start: float,
    ) -> TaskResult:
        """Decompose and execute as chain."""
        logger.info("Complex task -> decomposing")
        plan = await self._decomposer.decompose(task_input, classification)
        logger.info("Decomposed into %d subtasks", len(plan.subtasks))

        if len(plan.subtasks) <= 1:
            return await self._process_fn(task_input)

        if not self._chain_executor:
            return await self._process_fn(task_input)

        # Build chain
        chain = TaskChain(
            chain_id=uuid.uuid4().hex[:12],
            plan=plan,
            context=ChainContext(chain_id=uuid.uuid4().hex[:12]),
        )

        # Execute chain
        chain_result = await self._chain_executor.execute(chain)

        elapsed = (time.monotonic() - start) * 1000
        status = (
            TaskStatus.COMPLETED if chain_result.status.value == "completed" else TaskStatus.FAILED
        )

        return TaskResult(
            task_id=task_input.task_id,
            input_text=task_input.text,
            source=task_input.source,
            status=status,
            output_text=chain_result.combined_output,
            cost_estimate=chain_result.total_cost,
            duration_ms=elapsed,
            created_at=task_input.created_at,
            completed_at=datetime.now(UTC),
        )

    def _select_level(self, classification: TaskClassification) -> AgentLevel:
        """Pick the cheapest agent level capable of handling the complexity."""
        if classification.complexity <= 2:
            return AgentLevel.JUNIOR
        if classification.complexity == 3:
            return AgentLevel.SENIOR
        return AgentLevel.MANAGER
