"""End-to-end integration tests for AgentOS pipeline (E1-E4).

These test the full pipeline with real components (classifier, executor,
SafetyGuard, parser, store) — only the LLM Gateway is mocked since we
cannot make real API calls in CI.
"""

from __future__ import annotations

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


# ── E1: Simple command execution ─────────────────────────────────────


@pytest.mark.asyncio
async def test_e1_command_execution(mock_gateway, executor, classifier, parser, store):
    """E1: run echo hello -> classify -> LLM -> extract command -> CLI exec -> return stdout."""
    mock_gateway.complete.return_value = LLMResponse(
        content="Here you go:\n```bash\necho hello\n```",
        model="gpt-4o-mini",
        provider="openai",
        tokens_in=20,
        tokens_out=15,
        cost_estimate=0.0001,
        latency_ms=150.0,
    )
    agent = AgentCore(
        gateway=mock_gateway,
        classifier=classifier,
        executor=executor,
        parser=parser,
        store=store,
    )
    task = TaskInput(text="run echo hello", source="test", chat_id="1")
    result = await agent.process(task)

    assert result.status == TaskStatus.COMPLETED
    assert "hello" in result.output_text


# ── E2: Text question (no CLI) ──────────────────────────────────────


@pytest.mark.asyncio
async def test_e2_text_question(mock_gateway, classifier, parser, store):
    """E2: what is 2+2? -> classify -> LLM -> no CLI -> return LLM response."""
    agent = AgentCore(
        gateway=mock_gateway,
        classifier=classifier,
        parser=parser,
        store=store,
    )
    task = TaskInput(text="what is 2+2?", source="test", chat_id="1")
    result = await agent.process(task)

    assert result.status == TaskStatus.COMPLETED
    assert "4" in result.output_text
    assert result.model_used == "gpt-4o-mini"


# ── E3: System command ───────────────────────────────────────────────


@pytest.mark.asyncio
async def test_e3_system_command(mock_gateway, executor, classifier, parser, store):
    """E3: check disk space -> LLM suggests echo disk_info -> CLI executes."""
    mock_gateway.complete.return_value = LLMResponse(
        content="```bash\necho disk_info\n```",
        model="gpt-4o-mini",
        provider="openai",
        tokens_in=25,
        tokens_out=10,
        cost_estimate=0.0001,
        latency_ms=180.0,
    )
    agent = AgentCore(
        gateway=mock_gateway,
        classifier=classifier,
        executor=executor,
        parser=parser,
        store=store,
    )
    task = TaskInput(text="check disk space", source="test", chat_id="1")
    result = await agent.process(task)

    assert result.status == TaskStatus.COMPLETED
    assert result.output_text  # has some output


# ── E4: With active playbook ────────────────────────────────────────


@pytest.mark.asyncio
async def test_e4_with_playbook(mock_gateway, executor, classifier, parser, store):
    """E4: task with active playbook -> uses playbook's system prompt."""
    playbook_path = PROJECT_ROOT / "examples" / "playbooks" / "hello_world"
    agent = AgentCore(
        gateway=mock_gateway,
        classifier=classifier,
        executor=executor,
        parser=parser,
        store=store,
        active_playbook=playbook_path,
    )
    await agent.start()

    task = TaskInput(text="say hello", source="test", chat_id="1")
    result = await agent.process(task)

    assert result.status == TaskStatus.COMPLETED
    # Verify the LLM was called with the playbook's instructions as system_prompt
    call_args = mock_gateway.complete.call_args
    request = call_args.args[0] if call_args.args else call_args.kwargs.get("request")
    assert "Hello World" in request.system_prompt or "hello" in request.system_prompt.lower()

    await agent.shutdown()
