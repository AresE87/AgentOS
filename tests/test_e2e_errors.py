"""End-to-end error handling tests (E5-E12).

These test the full pipeline with real components (classifier, executor,
SafetyGuard, parser, store) — only the LLM Gateway is mocked since we
cannot make real API calls in CI.
"""

from __future__ import annotations

import asyncio
from pathlib import Path
from unittest.mock import AsyncMock

import pytest

from agentos.context.parser import ContextFolderParser
from agentos.core.agent import AgentCore
from agentos.executor.cli import CLIExecutor
from agentos.executor.safety import SafetyGuard
from agentos.gateway.classifier import RuleBasedClassifier
from agentos.store.task_store import TaskStore
from agentos.types import (
    LLMResponse,
    TaskInput,
    TaskStatus,
)

PROJECT_ROOT = Path(__file__).parent.parent


# ── Fixtures ─────────────────────────────────────────────────────────


@pytest.fixture
async def store():
    s = TaskStore(db_path=":memory:")
    await s.initialize()
    yield s
    await s.close()


@pytest.fixture
def mock_gateway():
    gw = AsyncMock()
    gw.complete.return_value = LLMResponse(
        content="The answer is 4.",
        model="gpt-4o-mini",
        provider="openai",
        tokens_in=20,
        tokens_out=10,
        cost_estimate=0.0001,
        latency_ms=200.0,
    )
    return gw


@pytest.fixture
def executor():
    safety_config = PROJECT_ROOT / "config" / "cli_safety.yaml"
    guard = SafetyGuard(config_path=safety_config)
    return CLIExecutor(safety=guard, default_timeout=30)


@pytest.fixture
def classifier():
    return RuleBasedClassifier()


@pytest.fixture
def parser():
    return ContextFolderParser()


# ── E5: Command that fails (exit code != 0) ─────────────────────────


@pytest.mark.asyncio
async def test_e5_command_fails(mock_gateway, executor, classifier, parser, store):
    """LLM suggests a command that fails -> user gets error with stderr."""
    mock_gateway.complete.return_value = LLMResponse(
        content="```bash\nls /nonexistent_dir_12345\n```",
        model="gpt-4o-mini",
        provider="openai",
        tokens_in=20,
        tokens_out=10,
        cost_estimate=0.0001,
        latency_ms=100.0,
    )
    agent = AgentCore(
        gateway=mock_gateway,
        classifier=classifier,
        executor=executor,
        parser=parser,
        store=store,
    )
    result = await agent.process(TaskInput(text="list nonexistent", source="test", chat_id="1"))
    assert result.status == TaskStatus.COMPLETED
    # Output should contain an error indication
    lower = result.output_text.lower()
    assert "fail" in lower or "no such" in lower or "not found" in lower or "cannot" in lower


# ── E6: Blocked command ──────────────────────────────────────────────


@pytest.mark.asyncio
async def test_e6_blocked_command(mock_gateway, executor, classifier, parser, store):
    """LLM suggests 'sudo rm -rf /' -> SafetyGuard blocks."""
    mock_gateway.complete.return_value = LLMResponse(
        content="```bash\nsudo rm -rf /\n```",
        model="gpt-4o-mini",
        provider="openai",
        tokens_in=20,
        tokens_out=10,
        cost_estimate=0.0001,
        latency_ms=100.0,
    )
    agent = AgentCore(
        gateway=mock_gateway,
        classifier=classifier,
        executor=executor,
        parser=parser,
        store=store,
    )
    result = await agent.process(TaskInput(text="delete everything", source="test", chat_id="1"))
    assert result.status == TaskStatus.COMPLETED
    lower = result.output_text.lower()
    assert "blocked" in lower or "safety" in lower


# ── E7: No API keys ─────────────────────────────────────────────────


@pytest.mark.asyncio
async def test_e7_no_api_keys(classifier, parser, store):
    """No gateway configured -> helpful error message."""
    agent = AgentCore(
        gateway=None,
        classifier=classifier,
        parser=parser,
        store=store,
    )
    result = await agent.process(TaskInput(text="hello", source="test", chat_id="1"))
    assert result.status == TaskStatus.COMPLETED
    lower = result.output_text.lower()
    assert "no ai provider" in lower or "api key" in lower


# ── E9: CLI timeout ──────────────────────────────────────────────────


@pytest.mark.asyncio
async def test_e9_cli_timeout(mock_gateway, classifier, parser, store):
    """Command times out -> user gets timeout message."""
    mock_gateway.complete.return_value = LLMResponse(
        content="```bash\nping -n 100 127.0.0.1\n```",
        model="gpt-4o-mini",
        provider="openai",
        tokens_in=20,
        tokens_out=10,
        cost_estimate=0.0001,
        latency_ms=100.0,
    )
    safety_config = PROJECT_ROOT / "config" / "cli_safety.yaml"
    guard = SafetyGuard(config_path=safety_config)
    timeout_executor = CLIExecutor(safety=guard, default_timeout=1)

    agent = AgentCore(
        gateway=mock_gateway,
        classifier=classifier,
        executor=timeout_executor,
        parser=parser,
        store=store,
    )
    result = await agent.process(TaskInput(text="ping forever", source="test", chat_id="1"))
    assert result.status == TaskStatus.COMPLETED
    lower = result.output_text.lower()
    assert "timeout" in lower or "timed out" in lower


# ── E10: Concurrent messages ─────────────────────────────────────────


@pytest.mark.asyncio
async def test_e10_concurrent_messages(mock_gateway, executor, classifier, parser, store):
    """3 messages sent simultaneously -> all 3 processed."""
    agent = AgentCore(
        gateway=mock_gateway,
        classifier=classifier,
        executor=executor,
        parser=parser,
        store=store,
    )
    tasks = [
        agent.process(TaskInput(text=f"task {i}", source="test", chat_id="1")) for i in range(3)
    ]
    results = await asyncio.gather(*tasks)
    assert len(results) == 3
    assert all(r.status == TaskStatus.COMPLETED for r in results)


# ── E11: Empty message ───────────────────────────────────────────────


@pytest.mark.asyncio
async def test_e11_empty_message(mock_gateway, classifier, parser, store):
    """Empty message -> still processes without crash."""
    agent = AgentCore(
        gateway=mock_gateway,
        classifier=classifier,
        parser=parser,
        store=store,
    )
    result = await agent.process(TaskInput(text="", source="test", chat_id="1"))
    assert result.status == TaskStatus.COMPLETED


# ── E12: Very long message ───────────────────────────────────────────


@pytest.mark.asyncio
async def test_e12_long_message(mock_gateway, classifier, parser, store):
    """10,000 char message -> processes without error."""
    long_text = "a" * 10000
    agent = AgentCore(
        gateway=mock_gateway,
        classifier=classifier,
        parser=parser,
        store=store,
    )
    result = await agent.process(TaskInput(text=long_text, source="test", chat_id="1"))
    assert result.status == TaskStatus.COMPLETED
