"""Tests for Orchestrator (AOS-037)."""

from __future__ import annotations

from datetime import UTC, datetime
from unittest.mock import AsyncMock, MagicMock

import pytest

from agentos.hierarchy.chain import ChainExecutor, ChainResult, ChainStatus
from agentos.hierarchy.decomposer import SubTaskDefinition, TaskDecomposer, TaskPlan
from agentos.hierarchy.orchestrator import Orchestrator
from agentos.types import LLMTier, TaskClassification, TaskInput, TaskResult, TaskStatus, TaskType


def _ok_result(text: str = "done") -> TaskResult:
    return TaskResult(
        task_id="t1",
        input_text="x",
        source="test",
        status=TaskStatus.COMPLETED,
        output_text=text,
        cost_estimate=0.01,
        created_at=datetime.now(UTC),
        completed_at=datetime.now(UTC),
    )


def _make_classification(complexity: int) -> TaskClassification:
    return TaskClassification(
        task_type=TaskType.TEXT,
        complexity=complexity,
        tier=LLMTier.STANDARD,
        confidence=0.9,
        reasoning="test",
    )


def _make_task_input(text: str = "Do something") -> TaskInput:
    return TaskInput(text=text, source="test", task_id="t1")


# ---- simple task (direct execution) --------------------------------------


@pytest.mark.asyncio
async def test_simple_task():
    process_fn = AsyncMock(return_value=_ok_result("direct"))
    classifier = AsyncMock()
    classifier.classify = AsyncMock(return_value=_make_classification(1))

    orchestrator = Orchestrator(process_fn=process_fn, classifier=classifier)
    result = await orchestrator.process(_make_task_input())

    assert result.status == TaskStatus.COMPLETED
    assert result.output_text == "direct"
    process_fn.assert_awaited_once()


# ---- complex task (decompose -> chain) ------------------------------------


@pytest.mark.asyncio
async def test_complex_task():
    process_fn = AsyncMock(return_value=_ok_result("chain-out"))
    classifier = AsyncMock()
    classifier.classify = AsyncMock(return_value=_make_classification(4))

    # Decomposer returns 2 subtasks
    decomposer = MagicMock(spec=TaskDecomposer)
    decomposer.should_decompose.return_value = True
    decomposer.decompose = AsyncMock(
        return_value=TaskPlan(
            original_task="complex",
            subtasks=[
                SubTaskDefinition(id="s1", description="Part 1"),
                SubTaskDefinition(id="s2", description="Part 2", depends_on=["s1"]),
            ],
            reasoning="split",
        )
    )

    # Chain executor returns a completed chain result
    chain_executor = MagicMock(spec=ChainExecutor)
    chain_executor.execute = AsyncMock(
        return_value=ChainResult(
            chain_id="c1",
            status=ChainStatus.COMPLETED,
            results={"s1": _ok_result("r1"), "s2": _ok_result("r2")},
            combined_output="combined output",
            total_cost=0.02,
            total_duration_ms=100.0,
        )
    )

    orchestrator = Orchestrator(
        process_fn=process_fn,
        classifier=classifier,
        decomposer=decomposer,
        chain_executor=chain_executor,
    )
    result = await orchestrator.process(_make_task_input("complex task"))

    assert result.status == TaskStatus.COMPLETED
    assert result.output_text == "combined output"
    decomposer.decompose.assert_awaited_once()
    chain_executor.execute.assert_awaited_once()
    # process_fn should NOT be called directly for complex tasks
    process_fn.assert_not_awaited()


# ---- never raises --------------------------------------------------------


@pytest.mark.asyncio
async def test_never_raises():
    async def boom(task_input):
        msg = "catastrophic failure"
        raise RuntimeError(msg)

    orchestrator = Orchestrator(process_fn=boom)
    result = await orchestrator.process(_make_task_input())

    assert result.status == TaskStatus.FAILED
    assert "catastrophic failure" in result.error_message


# ---- no decomposer -> direct execution -----------------------------------


@pytest.mark.asyncio
async def test_no_decomposer():
    process_fn = AsyncMock(return_value=_ok_result("direct"))
    classifier = AsyncMock()
    classifier.classify = AsyncMock(return_value=_make_classification(5))

    orchestrator = Orchestrator(process_fn=process_fn, classifier=classifier)
    result = await orchestrator.process(_make_task_input())

    assert result.status == TaskStatus.COMPLETED
    process_fn.assert_awaited_once()


# ---- single subtask plan -> direct execution ------------------------------


@pytest.mark.asyncio
async def test_single_subtask_plan():
    process_fn = AsyncMock(return_value=_ok_result("direct-single"))
    classifier = AsyncMock()
    classifier.classify = AsyncMock(return_value=_make_classification(4))

    decomposer = MagicMock(spec=TaskDecomposer)
    decomposer.should_decompose.return_value = True
    decomposer.decompose = AsyncMock(
        return_value=TaskPlan(
            original_task="task",
            subtasks=[SubTaskDefinition(id="s1", description="Only one")],
            reasoning="trivial",
        )
    )

    orchestrator = Orchestrator(
        process_fn=process_fn,
        classifier=classifier,
        decomposer=decomposer,
    )
    result = await orchestrator.process(_make_task_input())

    assert result.status == TaskStatus.COMPLETED
    assert result.output_text == "direct-single"
    # Falls back to direct execution since only 1 subtask
    process_fn.assert_awaited_once()
