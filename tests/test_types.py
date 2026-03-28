"""Tests for shared data types."""

from agentos.types import (
    ExecutionResult,
    LLMRequest,
    LLMResponse,
    LLMTier,
    ModelConfig,
    ModelProvider,
    PlaybookConfig,
    TaskClassification,
    TaskInput,
    TaskResult,
    TaskStatus,
    TaskType,
    UsageSummary,
)


def test_task_type_values():
    assert TaskType.TEXT == "text"
    assert TaskType.CODE == "code"
    assert TaskType.VISION == "vision"
    assert TaskType.GENERATION == "generation"
    assert TaskType.DATA == "data"


def test_llm_tier_values():
    assert LLMTier.CHEAP == 1
    assert LLMTier.STANDARD == 2
    assert LLMTier.PREMIUM == 3


def test_task_input_generates_id():
    t1 = TaskInput(text="hello")
    t2 = TaskInput(text="hello")
    assert t1.task_id != t2.task_id
    assert len(t1.task_id) == 12


def test_task_input_is_frozen():
    t = TaskInput(text="hello")
    try:
        t.text = "changed"  # type: ignore[misc]
        raise AssertionError("Should not allow mutation")
    except AttributeError:
        pass


def test_llm_request_defaults():
    r = LLMRequest(prompt="test", tier=LLMTier.CHEAP, task_type=TaskType.TEXT)
    assert r.system_prompt == ""
    assert r.max_tokens == 4096
    assert r.temperature == 0.7


def test_llm_response_is_frozen():
    r = LLMResponse(
        content="hello",
        model="gpt-4o-mini",
        provider="openai",
        tokens_in=10,
        tokens_out=20,
        cost_estimate=0.001,
        latency_ms=100.0,
    )
    assert r.content == "hello"


def test_model_config():
    mc = ModelConfig(
        provider=ModelProvider.ANTHROPIC,
        model_id="claude-3-haiku-20240307",
        display_name="Claude Haiku",
        cost_per_1m_input=0.25,
        cost_per_1m_output=1.25,
        max_tokens=4096,
    )
    assert mc.provider == ModelProvider.ANTHROPIC


def test_task_classification():
    tc = TaskClassification(
        task_type=TaskType.CODE,
        complexity=3,
        tier=LLMTier.STANDARD,
        confidence=0.85,
        reasoning="Contains code keywords",
    )
    assert tc.tier == LLMTier.STANDARD


def test_execution_result():
    er = ExecutionResult(
        command="echo hello",
        exit_code=0,
        stdout="hello\n",
        stderr="",
        duration_ms=50.0,
    )
    assert not er.timed_out


def test_task_result_defaults():
    tr = TaskResult(
        task_id="abc123",
        input_text="test",
        source="telegram",
        status=TaskStatus.PENDING,
    )
    assert tr.classification is None
    assert tr.tokens_in == 0


def test_playbook_config_defaults():
    pc = PlaybookConfig(name="Test")
    assert pc.tier == LLMTier.CHEAP
    assert pc.timeout == 300
    assert pc.name == "Test"
    assert pc.description == ""
    assert pc.blocked_commands == []


def test_usage_summary():
    us = UsageSummary(
        total_tokens_in=1000,
        total_tokens_out=500,
        total_cost=0.05,
        total_calls=10,
        calls_by_provider={"openai": 5, "anthropic": 5},
        calls_by_model={"gpt-4o-mini": 5, "haiku": 5},
        success_rate=0.9,
    )
    assert us.total_calls == 10
