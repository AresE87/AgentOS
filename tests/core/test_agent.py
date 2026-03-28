"""Tests for AgentCore pipeline."""

from __future__ import annotations

import asyncio
from pathlib import Path
from unittest.mock import AsyncMock

import pytest

from agentos.core.agent import AgentCore, extract_cli_command
from agentos.executor.cli import CommandBlockedError, CommandTimeoutError
from agentos.types import (
    ContextFolder,
    ExecutionResult,
    LLMResponse,
    LLMTier,
    PlaybookConfig,
    TaskClassification,
    TaskInput,
    TaskStatus,
    TaskType,
)

# ── Fixtures ──────────────────────────────────────────────────────────


@pytest.fixture
def mock_gateway():
    gw = AsyncMock()
    gw.complete.return_value = LLMResponse(
        content="Hello! I'm AgentOS.",
        model="gpt-4o-mini",
        provider="openai",
        tokens_in=10,
        tokens_out=20,
        cost_estimate=0.001,
        latency_ms=100.0,
    )
    return gw


@pytest.fixture
def mock_executor():
    ex = AsyncMock()
    ex.execute.return_value = ExecutionResult(
        command="echo hello",
        exit_code=0,
        stdout="hello\n",
        stderr="",
        duration_ms=50.0,
    )
    return ex


@pytest.fixture
def mock_store():
    store = AsyncMock()
    store.create_task = AsyncMock()
    store.update_task_classification = AsyncMock()
    store.update_task_status = AsyncMock()
    store.complete_task = AsyncMock()
    store.fail_task = AsyncMock()
    store.save_execution = AsyncMock()
    return store


@pytest.fixture
def mock_classifier():
    cls = AsyncMock()
    cls.classify.return_value = TaskClassification(
        task_type=TaskType.TEXT,
        complexity=1,
        tier=LLMTier.CHEAP,
        confidence=0.8,
        reasoning="Test classification",
    )
    return cls


@pytest.fixture
def task_input():
    return TaskInput(text="explain quantum computing", source="telegram", chat_id="123")


# ── extract_cli_command tests ─────────────────────────────────────────


def test_extract_cli_command_single():
    """Single bash block returns the command."""
    text = "Here is the command:\n```bash\ndf -h\n```"
    assert extract_cli_command(text) == "df -h"


def test_extract_cli_command_none():
    """No bash block returns None."""
    text = "The capital of France is Paris."
    assert extract_cli_command(text) is None


def test_extract_cli_command_multiple():
    """Multiple bash blocks returns None (ambiguous)."""
    text = "```bash\necho a\n```\n\n```bash\necho b\n```"
    assert extract_cli_command(text) is None


# ── Pipeline tests ────────────────────────────────────────────────────


@pytest.mark.asyncio
async def test_process_text_task(mock_gateway, mock_classifier, mock_store, task_input):
    """LLM returns text (no bash block) -> output is LLM text."""
    agent = AgentCore(
        gateway=mock_gateway,
        classifier=mock_classifier,
        store=mock_store,
    )

    result = await agent.process(task_input)

    assert result.status == TaskStatus.COMPLETED
    assert result.output_text == "Hello! I'm AgentOS."
    assert result.model_used == "gpt-4o-mini"
    mock_gateway.complete.assert_called_once()
    mock_store.create_task.assert_called_once()
    mock_store.update_task_classification.assert_called_once()
    mock_store.complete_task.assert_called_once()


@pytest.mark.asyncio
async def test_process_cli_task(
    mock_gateway, mock_classifier, mock_executor, mock_store, task_input
):
    """LLM returns a bash block -> executor called, output is stdout."""
    mock_gateway.complete.return_value = LLMResponse(
        content="Here you go:\n```bash\necho hello\n```",
        model="gpt-4o-mini",
        provider="openai",
        tokens_in=10,
        tokens_out=5,
        cost_estimate=0.0001,
        latency_ms=80.0,
    )
    agent = AgentCore(
        gateway=mock_gateway,
        classifier=mock_classifier,
        executor=mock_executor,
        store=mock_store,
    )

    result = await agent.process(task_input)

    assert result.status == TaskStatus.COMPLETED
    assert "hello" in result.output_text
    mock_executor.execute.assert_called_once_with("echo hello")
    mock_store.save_execution.assert_called_once()


@pytest.mark.asyncio
async def test_process_cli_failed(
    mock_gateway, mock_classifier, mock_executor, mock_store, task_input
):
    """Executor returns exit_code=1 -> output has error."""
    mock_gateway.complete.return_value = LLMResponse(
        content="```bash\nfake-cmd\n```",
        model="gpt-4o-mini",
        provider="openai",
        tokens_in=10,
        tokens_out=5,
        cost_estimate=0.0001,
        latency_ms=80.0,
    )
    mock_executor.execute.return_value = ExecutionResult(
        command="fake-cmd",
        exit_code=1,
        stdout="",
        stderr="command not found",
        duration_ms=10.0,
    )
    agent = AgentCore(
        gateway=mock_gateway,
        classifier=mock_classifier,
        executor=mock_executor,
        store=mock_store,
    )

    result = await agent.process(task_input)

    assert result.status == TaskStatus.COMPLETED
    assert "Command failed" in result.output_text
    assert "command not found" in result.output_text


@pytest.mark.asyncio
async def test_process_cli_blocked(
    mock_gateway, mock_classifier, mock_executor, mock_store, task_input
):
    """Executor raises CommandBlockedError -> output says 'blocked for safety'."""
    mock_gateway.complete.return_value = LLMResponse(
        content="```bash\nrm -rf /\n```",
        model="gpt-4o-mini",
        provider="openai",
        tokens_in=10,
        tokens_out=5,
        cost_estimate=0.0001,
        latency_ms=80.0,
    )
    mock_executor.execute.side_effect = CommandBlockedError("dangerous command")
    agent = AgentCore(
        gateway=mock_gateway,
        classifier=mock_classifier,
        executor=mock_executor,
        store=mock_store,
    )

    result = await agent.process(task_input)

    assert result.status == TaskStatus.COMPLETED
    assert "blocked for safety" in result.output_text.lower()


@pytest.mark.asyncio
async def test_process_cli_timeout(
    mock_gateway, mock_classifier, mock_executor, mock_store, task_input
):
    """Executor raises CommandTimeoutError -> output says 'timed out'."""
    mock_gateway.complete.return_value = LLMResponse(
        content="```bash\nsleep 9999\n```",
        model="gpt-4o-mini",
        provider="openai",
        tokens_in=10,
        tokens_out=5,
        cost_estimate=0.0001,
        latency_ms=80.0,
    )
    mock_executor.execute.side_effect = CommandTimeoutError("exceeded 300s")
    agent = AgentCore(
        gateway=mock_gateway,
        classifier=mock_classifier,
        executor=mock_executor,
        store=mock_store,
    )

    result = await agent.process(task_input)

    assert result.status == TaskStatus.COMPLETED
    assert "timed out" in result.output_text.lower()


@pytest.mark.asyncio
async def test_process_no_gateway(mock_classifier, mock_store, task_input):
    """No gateway configured -> 'No AI providers configured'."""
    agent = AgentCore(
        classifier=mock_classifier,
        store=mock_store,
    )

    result = await agent.process(task_input)

    assert result.status == TaskStatus.COMPLETED
    assert "No AI providers configured" in result.output_text


@pytest.mark.asyncio
async def test_process_gateway_error(mock_gateway, mock_classifier, mock_store, task_input):
    """Gateway raises exception -> status=FAILED."""
    mock_gateway.complete.side_effect = RuntimeError("API Error")
    agent = AgentCore(
        gateway=mock_gateway,
        classifier=mock_classifier,
        store=mock_store,
    )

    result = await agent.process(task_input)

    assert result.status == TaskStatus.FAILED
    assert "API Error" in result.error_message
    mock_store.fail_task.assert_called_once()


@pytest.mark.asyncio
async def test_process_never_raises(mock_store, task_input):
    """Force exception in classifier -> returns FAILED, no exception raised."""
    broken_classifier = AsyncMock()
    broken_classifier.classify.side_effect = ValueError("classifier exploded")
    agent = AgentCore(
        classifier=broken_classifier,
        store=mock_store,
    )

    # Should NOT raise
    result = await agent.process(task_input)

    assert result.status == TaskStatus.FAILED
    assert "classifier exploded" in result.error_message


# ── Lifecycle tests ───────────────────────────────────────────────────


@pytest.mark.asyncio
async def test_start_initializes_store(mock_store):
    """start() calls store.initialize()."""
    agent = AgentCore(store=mock_store)
    await agent.start()
    mock_store.initialize.assert_called_once()


@pytest.mark.asyncio
async def test_shutdown_closes_store(mock_store):
    """shutdown() calls store.close()."""
    agent = AgentCore(store=mock_store)
    await agent.shutdown()
    mock_store.close.assert_called_once()


# ── Concurrency test ──────────────────────────────────────────────────


@pytest.mark.asyncio
async def test_semaphore_limits_concurrency(mock_gateway, mock_classifier):
    """Semaphore limits the number of concurrent tasks."""
    max_concurrent = 2
    running_count = 0
    max_observed = 0

    async def slow_complete(request):
        nonlocal running_count, max_observed
        running_count += 1
        if running_count > max_observed:
            max_observed = running_count
        await asyncio.sleep(0.05)
        running_count -= 1
        return LLMResponse(
            content="done",
            model="test",
            provider="test",
            tokens_in=1,
            tokens_out=1,
            cost_estimate=0.0,
            latency_ms=50.0,
        )

    mock_gateway.complete = slow_complete

    agent = AgentCore(
        gateway=mock_gateway,
        classifier=mock_classifier,
        max_concurrent_tasks=max_concurrent,
    )

    tasks = [agent.process(TaskInput(text=f"task {i}", source="test")) for i in range(6)]
    await asyncio.gather(*tasks)

    assert max_observed <= max_concurrent


# ── Playbook test ─────────────────────────────────────────────────────


@pytest.mark.asyncio
async def test_active_playbook(mock_gateway, mock_classifier, task_input):
    """set_active_playbook loads playbook and changes system_prompt."""
    custom_instructions = "You are a custom bot."
    mock_parser = AsyncMock()
    mock_parser.parse.return_value = ContextFolder(
        path="/tmp/playbook",
        config=PlaybookConfig(name="test-playbook"),
        instructions=custom_instructions,
    )

    agent = AgentCore(
        gateway=mock_gateway,
        classifier=mock_classifier,
        parser=mock_parser,
        active_playbook=Path("/tmp/playbook"),
    )
    await agent.start()

    # After start, active_context should be set
    assert agent._active_context is not None
    assert agent._active_context.instructions == custom_instructions

    result = await agent.process(task_input)

    assert result.status == TaskStatus.COMPLETED
    # Verify the LLM was called with the custom system prompt
    call_args = mock_gateway.complete.call_args
    assert call_args is not None
    request = call_args[0][0]
    assert request.system_prompt == custom_instructions
