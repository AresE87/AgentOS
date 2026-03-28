"""Tests for the SQLite-backed TaskStore (Data Design v1)."""

from __future__ import annotations

import pytest

from agentos.store.task_store import DB_STDOUT_MAX, SCHEMA_VERSION, TaskStore
from agentos.types import (
    ExecutionResult,
    LLMResponse,
    LLMTier,
    TaskClassification,
    TaskInput,
    TaskStatus,
    TaskType,
)


@pytest.fixture
async def store() -> TaskStore:  # noqa: RUF029
    """Provide an initialised in-memory TaskStore."""
    s = TaskStore(db_path=":memory:")
    await s.initialize()
    try:
        yield s  # type: ignore[misc]
    finally:
        await s.close()


# ── helpers ──────────────────────────────────────────────────────


def _make_input(
    text: str = "list files",
    source: str = "telegram",
    chat_id: str = "chat-1",
) -> TaskInput:
    return TaskInput(text=text, source=source, chat_id=chat_id)


def _make_llm_response(
    provider: str = "openai",
    model: str = "gpt-4o-mini",
    tokens_in: int = 100,
    tokens_out: int = 200,
    cost: float = 0.005,
    latency: float = 350.0,
) -> LLMResponse:
    return LLMResponse(
        content="hello",
        model=model,
        provider=provider,
        tokens_in=tokens_in,
        tokens_out=tokens_out,
        cost_estimate=cost,
        latency_ms=latency,
    )


# ── table creation ──────────────────────────────────────────────


@pytest.mark.asyncio
async def test_initialize_creates_tables(store: TaskStore) -> None:
    assert store._db is not None
    cursor = await store._db.execute(
        "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"
    )
    tables = {row["name"] for row in await cursor.fetchall()}
    assert "tasks" in tables
    assert "execution_log" in tables
    assert "llm_usage" in tables
    assert "_schema_version" in tables


# ── schema version ──────────────────────────────────────────────


@pytest.mark.asyncio
async def test_schema_version_tracked(store: TaskStore) -> None:
    assert store._db is not None
    cursor = await store._db.execute("SELECT version FROM _schema_version")
    row = await cursor.fetchone()
    assert row is not None
    assert row["version"] == SCHEMA_VERSION


# ── create and get task ─────────────────────────────────────────


@pytest.mark.asyncio
async def test_create_and_get_task(store: TaskStore) -> None:
    task_id = await store.create_task(_make_input())
    assert task_id  # non-empty UUID string

    task = await store.get_task(task_id)
    assert task is not None
    assert task["id"] == task_id
    assert task["input_text"] == "list files"
    assert task["source"] == "telegram"
    assert task["chat_id"] == "chat-1"
    assert task["status"] == TaskStatus.PENDING.value
    assert task["created_at"] is not None


# ── update status to running ────────────────────────────────────


@pytest.mark.asyncio
async def test_update_task_status_to_running(store: TaskStore) -> None:
    task_id = await store.create_task(_make_input())
    await store.update_task_status(task_id, TaskStatus.RUNNING)

    task = await store.get_task(task_id)
    assert task is not None
    assert task["status"] == TaskStatus.RUNNING.value
    assert task["started_at"] is not None


# ── update task classification ──────────────────────────────────


@pytest.mark.asyncio
async def test_update_task_classification(store: TaskStore) -> None:
    task_id = await store.create_task(_make_input())
    classification = TaskClassification(
        task_type=TaskType.CODE,
        complexity=3,
        tier=LLMTier.STANDARD,
        confidence=0.9,
        reasoning="code task",
    )
    await store.update_task_classification(task_id, classification)

    task = await store.get_task(task_id)
    assert task is not None
    assert task["task_type"] == TaskType.CODE.value
    assert task["complexity"] == 3
    assert task["tier"] == LLMTier.STANDARD.value


# ── complete task ───────────────────────────────────────────────


@pytest.mark.asyncio
async def test_complete_task(store: TaskStore) -> None:
    task_id = await store.create_task(_make_input())
    await store.update_task_status(task_id, TaskStatus.RUNNING)

    llm = _make_llm_response()
    await store.complete_task(task_id, output="done", llm_response=llm)

    task = await store.get_task(task_id)
    assert task is not None
    assert task["status"] == TaskStatus.COMPLETED.value
    assert task["output_text"] == "done"
    assert task["completed_at"] is not None
    assert task["duration_ms"] >= 0.0
    assert task["model_used"] == "gpt-4o-mini"
    assert task["provider"] == "openai"
    assert task["tokens_in"] == 100
    assert task["tokens_out"] == 200
    assert task["cost_estimate"] == pytest.approx(0.005)


# ── fail task ───────────────────────────────────────────────────


@pytest.mark.asyncio
async def test_fail_task(store: TaskStore) -> None:
    task_id = await store.create_task(_make_input())
    await store.fail_task(task_id, error="something broke")

    task = await store.get_task(task_id)
    assert task is not None
    assert task["status"] == TaskStatus.FAILED.value
    assert task["error_message"] == "something broke"
    assert task["completed_at"] is not None


# ── recent tasks ordering ──────────────────────────────────────


@pytest.mark.asyncio
async def test_get_recent_tasks_ordering(store: TaskStore) -> None:
    ids = []
    for _ in range(5):
        tid = await store.create_task(_make_input())
        ids.append(tid)

    recent = await store.get_recent_tasks(limit=3)
    assert len(recent) == 3
    # Most recently inserted should come first
    assert recent[0]["id"] == ids[-1]


# ── nonexistent task ────────────────────────────────────────────


@pytest.mark.asyncio
async def test_get_nonexistent_task_returns_none(store: TaskStore) -> None:
    result = await store.get_task("does-not-exist")
    assert result is None


# ── execution log ───────────────────────────────────────────────


@pytest.mark.asyncio
async def test_save_execution(store: TaskStore) -> None:
    task_id = await store.create_task(_make_input())

    result = ExecutionResult(
        command="ls -la",
        exit_code=0,
        stdout="total 8\n",
        stderr="",
        duration_ms=42.5,
    )
    await store.save_execution(task_id, result)

    rows = await store.get_task_executions(task_id)
    assert len(rows) == 1
    row = rows[0]
    assert row["command"] == "ls -la"
    assert row["exit_code"] == 0
    assert row["success"] == 1
    assert row["stdout"] == "total 8\n"
    assert row["duration_ms"] == pytest.approx(42.5)
    # ID should be a UUID string
    assert len(row["id"]) == 36


@pytest.mark.asyncio
async def test_save_execution_truncates_stdout(store: TaskStore) -> None:
    task_id = await store.create_task(_make_input())

    big_output = "x" * (DB_STDOUT_MAX + 5000)
    result = ExecutionResult(
        command="cat bigfile",
        exit_code=0,
        stdout=big_output,
        stderr=big_output,
        duration_ms=10.0,
    )
    await store.save_execution(task_id, result)

    rows = await store.get_task_executions(task_id)
    assert len(rows) == 1
    assert len(rows[0]["stdout"]) == DB_STDOUT_MAX
    assert len(rows[0]["stderr"]) == DB_STDOUT_MAX


# ── LLM usage ──────────────────────────────────────────────────


@pytest.mark.asyncio
async def test_save_llm_usage(store: TaskStore) -> None:
    task_id = await store.create_task(_make_input())
    llm = _make_llm_response()

    await store.save_llm_usage(task_id, llm, fallback_index=0)

    rows = await store.get_task_llm_usage(task_id)
    assert len(rows) == 1
    row = rows[0]
    assert row["provider"] == "openai"
    assert row["model"] == "gpt-4o-mini"
    assert row["tokens_in"] == 100
    assert row["tokens_out"] == 200
    assert row["success"] == 1
    assert row["fallback_index"] == 0
    assert len(row["id"]) == 36


@pytest.mark.asyncio
async def test_save_llm_usage_with_fallback(store: TaskStore) -> None:
    task_id = await store.create_task(_make_input())

    llm1 = _make_llm_response(provider="openai", model="gpt-4o")
    llm2 = _make_llm_response(provider="anthropic", model="claude-3-haiku")

    await store.save_llm_usage(task_id, llm1, fallback_index=0, error_type="rate_limit")
    await store.save_llm_usage(task_id, llm2, fallback_index=1)

    rows = await store.get_task_llm_usage(task_id)
    assert len(rows) == 2
    assert rows[0]["fallback_index"] == 0
    assert rows[0]["error_type"] == "rate_limit"
    assert rows[0]["success"] == 0
    assert rows[1]["fallback_index"] == 1
    assert rows[1]["success"] == 1


# ── cost by period ──────────────────────────────────────────────


@pytest.mark.asyncio
async def test_get_cost_by_period(store: TaskStore) -> None:
    task_id = await store.create_task(_make_input())

    await store.save_llm_usage(
        task_id,
        _make_llm_response(cost=0.005, tokens_in=100, tokens_out=200),
    )
    await store.save_llm_usage(
        task_id,
        _make_llm_response(
            provider="anthropic", model="claude-3-haiku", cost=0.002, tokens_in=50, tokens_out=100
        ),
    )

    result = await store.get_cost_by_period("2000-01-01", "2099-12-31")
    assert result["total_calls"] == 2
    assert result["total_cost"] == pytest.approx(0.007)
    assert result["total_tokens_in"] == 150
    assert result["total_tokens_out"] == 300


# ── cost by provider ────────────────────────────────────────────


@pytest.mark.asyncio
async def test_get_cost_by_provider(store: TaskStore) -> None:
    task_id = await store.create_task(_make_input())

    await store.save_llm_usage(
        task_id, _make_llm_response(provider="openai", cost=0.005, latency=300.0)
    )
    await store.save_llm_usage(
        task_id,
        _make_llm_response(provider="anthropic", model="claude-3-haiku", cost=0.002, latency=200.0),
    )

    rows = await store.get_cost_by_provider("2000-01-01", "2099-12-31")
    providers = {r["provider"]: r for r in rows}
    assert "openai" in providers
    assert "anthropic" in providers
    assert providers["openai"]["call_count"] == 1
    assert providers["openai"]["total_cost"] == pytest.approx(0.005)


# ── success rate ────────────────────────────────────────────────


@pytest.mark.asyncio
async def test_get_success_rate(store: TaskStore) -> None:
    t1 = await store.create_task(_make_input())
    t2 = await store.create_task(_make_input())
    t3 = await store.create_task(_make_input())

    await store.complete_task(t1, output="ok")
    await store.complete_task(t2, output="ok")
    await store.fail_task(t3, error="boom")

    rate = await store.get_success_rate("2000-01-01", "2099-12-31")
    assert rate["total"] == 3
    assert rate["completed"] == 2
    assert rate["failed"] == 1
