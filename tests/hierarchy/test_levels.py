"""Tests for agent levels and profiles (AOS-032)."""

from __future__ import annotations

from pathlib import Path
from unittest.mock import AsyncMock

import pytest

from agentos.core.agent import AgentCore
from agentos.hierarchy.levels import (
    DEFAULT_PROFILES,
    AgentLevel,
    AgentProfile,
    get_profile,
    load_profiles_from_yaml,
)
from agentos.types import LLMResponse, LLMTier, TaskInput

# ── AgentLevel enum ────────────────────────────────────────────────


def test_agent_level_values() -> None:
    assert AgentLevel.JUNIOR == "junior"
    assert AgentLevel.SPECIALIST == "specialist"
    assert AgentLevel.SENIOR == "senior"
    assert AgentLevel.MANAGER == "manager"
    assert AgentLevel.ORCHESTRATOR == "orchestrator"


# ── Default profiles ───────────────────────────────────────────────


def test_default_profiles_exist() -> None:
    for level in AgentLevel:
        assert level in DEFAULT_PROFILES, f"Missing default profile for {level}"


def test_junior_profile() -> None:
    profile = DEFAULT_PROFILES[AgentLevel.JUNIOR]
    assert profile.tier == LLMTier.CHEAP
    assert profile.max_tokens == 2048


def test_senior_profile() -> None:
    profile = DEFAULT_PROFILES[AgentLevel.SENIOR]
    assert profile.tier == LLMTier.STANDARD
    assert "screen" in profile.allowed_tools


def test_orchestrator_profile() -> None:
    profile = DEFAULT_PROFILES[AgentLevel.ORCHESTRATOR]
    assert profile.temperature == pytest.approx(0.3)


# ── get_profile helper ─────────────────────────────────────────────


def test_get_profile() -> None:
    profile = get_profile(AgentLevel.JUNIOR)
    assert isinstance(profile, AgentProfile)
    assert profile.level == AgentLevel.JUNIOR
    assert profile.tier == LLMTier.CHEAP


# ── YAML loading ───────────────────────────────────────────────────


def test_load_profiles_from_yaml() -> None:
    config_path = Path(__file__).resolve().parents[2] / "config" / "levels.yaml"
    profiles = load_profiles_from_yaml(config_path)
    assert len(profiles) == 5
    assert "junior" in profiles
    assert "orchestrator" in profiles
    assert profiles["junior"].tier == LLMTier.CHEAP
    assert profiles["specialist"].tier == LLMTier.STANDARD
    assert profiles["senior"].max_tokens == 8192
    assert profiles["orchestrator"].temperature == pytest.approx(0.3)


def test_load_profiles_from_yaml_missing_file(tmp_path: Path) -> None:
    result = load_profiles_from_yaml(tmp_path / "nonexistent.yaml")
    assert result == {}


# ── Backward compatibility ─────────────────────────────────────────


@pytest.mark.asyncio
async def test_backward_compat() -> None:
    """AgentCore.process() without profile works same as before."""
    mock_gateway = AsyncMock()
    mock_gateway.complete.return_value = LLMResponse(
        content="Hello from the LLM",
        model="test-model",
        provider="test",
        tokens_in=10,
        tokens_out=5,
        cost_estimate=0.001,
        latency_ms=100.0,
    )

    agent = AgentCore(gateway=mock_gateway)
    task = TaskInput(text="What is 2+2?", source="test")
    result = await agent.process(task)

    assert result.output_text == "Hello from the LLM"
    assert result.status.value == "completed"
    # Gateway should have been called with the default system prompt
    call_args = mock_gateway.complete.call_args
    request = call_args[0][0]
    assert "AgentOS" in request.system_prompt


@pytest.mark.asyncio
async def test_process_with_profile() -> None:
    """AgentCore.process() with profile uses profile's prompt and tier."""
    mock_gateway = AsyncMock()
    mock_gateway.complete.return_value = LLMResponse(
        content="Profile response",
        model="test-model",
        provider="test",
        tokens_in=10,
        tokens_out=5,
        cost_estimate=0.001,
        latency_ms=100.0,
    )

    agent = AgentCore(gateway=mock_gateway)
    task = TaskInput(text="Do something", source="test")
    profile = get_profile(AgentLevel.JUNIOR)
    result = await agent.process(task, profile=profile)

    assert result.output_text == "Profile response"
    call_args = mock_gateway.complete.call_args
    request = call_args[0][0]
    assert request.system_prompt == profile.system_prompt
    assert request.tier == LLMTier.CHEAP


@pytest.mark.asyncio
async def test_process_with_chain_context() -> None:
    """AgentCore.process() with chain_context prepends dependency outputs."""
    mock_gateway = AsyncMock()
    mock_gateway.complete.return_value = LLMResponse(
        content="Chained response",
        model="test-model",
        provider="test",
        tokens_in=10,
        tokens_out=5,
        cost_estimate=0.001,
        latency_ms=100.0,
    )

    agent = AgentCore(gateway=mock_gateway)
    task = TaskInput(text="Summarize the above", source="test")
    result = await agent.process(task, chain_context=["Output from task A", "Output from task B"])

    assert result.output_text == "Chained response"
    call_args = mock_gateway.complete.call_args
    request = call_args[0][0]
    assert "Previous task outputs:" in request.prompt
    assert "Output from task A" in request.prompt
    assert "Output from task B" in request.prompt
