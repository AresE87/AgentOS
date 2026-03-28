"""Tests for ChainExecutor and ChainContext (AOS-035 / AOS-036)."""

from __future__ import annotations

import asyncio
from datetime import UTC, datetime
from unittest.mock import AsyncMock

import pytest

from agentos.hierarchy.chain import ChainExecutor, ChainStatus, TaskChain
from agentos.hierarchy.context import ChainContext
from agentos.hierarchy.decomposer import SubTaskDefinition, TaskPlan
from agentos.types import TaskResult, TaskStatus


def _ok_result(text: str = "done") -> TaskResult:
    return TaskResult(
        task_id="t1",
        input_text="x",
        source="chain",
        status=TaskStatus.COMPLETED,
        output_text=text,
        cost_estimate=0.01,
        created_at=datetime.now(UTC),
        completed_at=datetime.now(UTC),
    )


def _fail_result() -> TaskResult:
    return TaskResult(
        task_id="t1",
        input_text="x",
        source="chain",
        status=TaskStatus.FAILED,
        error_message="boom",
        created_at=datetime.now(UTC),
        completed_at=datetime.now(UTC),
    )


def _make_chain(subtasks: list[SubTaskDefinition]) -> TaskChain:
    plan = TaskPlan(original_task="test", subtasks=subtasks)
    return TaskChain(
        chain_id="chain1",
        plan=plan,
        context=ChainContext(chain_id="chain1"),
    )


# ---- single subtask ------------------------------------------------------


@pytest.mark.asyncio
async def test_execute_single_subtask():
    process_fn = AsyncMock(return_value=_ok_result("output1"))
    executor = ChainExecutor(process_fn)
    chain = _make_chain([SubTaskDefinition(id="s1", description="Do thing")])

    result = await executor.execute(chain)

    assert result.status == ChainStatus.COMPLETED
    assert "s1" in result.results
    process_fn.assert_awaited_once()


# ---- sequential chain (A -> B -> C) --------------------------------------


@pytest.mark.asyncio
async def test_execute_sequential():
    call_order: list[str] = []

    async def process_fn(task_input):
        call_order.append(task_input.text)
        return _ok_result(f"result-{len(call_order)}")

    executor = ChainExecutor(process_fn)
    chain = _make_chain(
        [
            SubTaskDefinition(id="a", description="Step A"),
            SubTaskDefinition(id="b", description="Step B", depends_on=["a"]),
            SubTaskDefinition(id="c", description="Step C", depends_on=["b"]),
        ]
    )

    result = await executor.execute(chain)

    assert result.status == ChainStatus.COMPLETED
    assert len(result.results) == 3
    # A must execute before B, B before C
    assert call_order.index("Step A") < call_order.index(
        "Previous results:\n[a]: result-1\n\nYour task: Step B"
    )


# ---- parallel chain (A, B independent) -----------------------------------


@pytest.mark.asyncio
async def test_execute_parallel():
    call_count = 0

    async def process_fn(task_input):
        nonlocal call_count
        call_count += 1
        return _ok_result(f"out-{call_count}")

    executor = ChainExecutor(process_fn)
    chain = _make_chain(
        [
            SubTaskDefinition(id="a", description="Task A"),
            SubTaskDefinition(id="b", description="Task B"),
        ]
    )

    result = await executor.execute(chain)

    assert result.status == ChainStatus.COMPLETED
    assert len(result.results) == 2


# ---- dependency output passing -------------------------------------------


@pytest.mark.asyncio
async def test_execute_dependency_output():
    prompts: list[str] = []

    async def process_fn(task_input):
        prompts.append(task_input.text)
        return _ok_result("Hello from A")

    executor = ChainExecutor(process_fn)
    chain = _make_chain(
        [
            SubTaskDefinition(id="a", description="Produce data"),
            SubTaskDefinition(id="b", description="Consume data", depends_on=["a"]),
        ]
    )

    result = await executor.execute(chain)

    assert result.status == ChainStatus.COMPLETED
    # B's prompt should contain A's output
    assert any("Hello from A" in p for p in prompts)


# ---- failure handling ----------------------------------------------------


@pytest.mark.asyncio
async def test_execute_failure():
    call_count = 0

    async def process_fn(task_input):
        nonlocal call_count
        call_count += 1
        if call_count == 1:
            return _fail_result()
        return _ok_result("ok")

    executor = ChainExecutor(process_fn)
    chain = _make_chain(
        [
            SubTaskDefinition(id="a", description="Fail"),
            SubTaskDefinition(id="b", description="OK"),
        ]
    )

    result = await executor.execute(chain)

    # One succeeded, one failed -> partial failure
    assert result.status == ChainStatus.PARTIAL_FAILURE


# ---- timeout -------------------------------------------------------------


@pytest.mark.asyncio
async def test_execute_timeout():
    async def slow_fn(task_input):
        await asyncio.sleep(10)
        return _ok_result()

    executor = ChainExecutor(slow_fn, timeout=0.1)
    chain = _make_chain(
        [
            SubTaskDefinition(id="a", description="Slow A"),
            SubTaskDefinition(id="b", description="Slow B", depends_on=["a"]),
        ]
    )

    result = await executor.execute(chain)

    # Should not complete all subtasks within timeout
    assert result.status in (ChainStatus.FAILED, ChainStatus.PARTIAL_FAILURE)


# ---- ChainContext --------------------------------------------------------


def test_chain_context():
    ctx = ChainContext(chain_id="c1")

    ctx.set("s1", "output", "Result from s1")
    ctx.set("s2", "output", "Result from s2")
    ctx.set("s1", "extra", "metadata")

    assert ctx.get("s1", "output") == "Result from s1"
    assert ctx.get_output("s2") == "Result from s2"
    assert ctx.get("s1", "extra") == "metadata"
    assert ctx.get("s3", "output") is None

    dep_outputs = ctx.get_dependency_outputs("s3", ["s1", "s2"])
    assert "[s1]: Result from s1" in dep_outputs
    assert "[s2]: Result from s2" in dep_outputs

    # Serialization roundtrip
    data = ctx.to_dict()
    ctx2 = ChainContext.from_dict(data)
    assert ctx2.get_output("s1") == "Result from s1"
    assert ctx2.chain_id == "c1"
