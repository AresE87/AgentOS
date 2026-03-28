"""Shared test fixtures for AgentOS."""

from __future__ import annotations

from pathlib import Path

import pytest

from agentos.settings import Settings
from agentos.types import (
    LLMRequest,
    LLMResponse,
    LLMTier,
    TaskClassification,
    TaskInput,
    TaskType,
)

FIXTURES_DIR = Path(__file__).parent / "fixtures"
PROJECT_ROOT = Path(__file__).parent.parent


@pytest.fixture
def settings() -> Settings:
    """Settings with fake API keys for testing."""
    return Settings(
        anthropic_api_key="sk-ant-test-fake-key-for-testing",
        openai_api_key="sk-test-fake-openai-key-for-testing",
        google_api_key="AIzaSy-test-fake-google-key",
        telegram_bot_token="123456:ABC-TEST-TOKEN",
        log_level="DEBUG",
        max_cost_per_task=1.00,
        cli_timeout=30,
        db_path=":memory:",
        config_dir=str(PROJECT_ROOT / "config"),
        playbooks_dir=str(PROJECT_ROOT / "examples" / "playbooks"),
    )


@pytest.fixture
def sample_task_input() -> TaskInput:
    """A simple task input for testing."""
    return TaskInput(
        text="list files in the current directory",
        source="telegram",
        chat_id="12345",
    )


@pytest.fixture
def sample_classification() -> TaskClassification:
    """A sample classification result."""
    return TaskClassification(
        task_type=TaskType.CODE,
        complexity=1,
        tier=LLMTier.CHEAP,
        confidence=0.85,
        reasoning="Single CLI command",
    )


@pytest.fixture
def sample_llm_request() -> LLMRequest:
    """A sample LLM request."""
    return LLMRequest(
        prompt="Explain quantum computing",
        tier=LLMTier.CHEAP,
        task_type=TaskType.TEXT,
        system_prompt="You are a helpful assistant.",
    )


@pytest.fixture
def sample_llm_response() -> LLMResponse:
    """A sample LLM response."""
    return LLMResponse(
        content="Quantum computing uses quantum bits...",
        model="gpt-4o-mini",
        provider="openai",
        tokens_in=25,
        tokens_out=150,
        cost_estimate=0.000094,
        latency_ms=820.5,
    )


@pytest.fixture
def routing_config_path() -> Path:
    """Path to the routing config file."""
    return PROJECT_ROOT / "config" / "routing.yaml"


@pytest.fixture
def cli_safety_config_path() -> Path:
    """Path to the CLI safety config file."""
    return PROJECT_ROOT / "config" / "cli_safety.yaml"


@pytest.fixture
def playbooks_dir() -> Path:
    """Path to example playbooks."""
    return PROJECT_ROOT / "examples" / "playbooks"
