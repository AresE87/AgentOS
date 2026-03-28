"""Tests for LiteLLMProvider with mocked litellm.acompletion."""

from __future__ import annotations

from types import SimpleNamespace
from unittest.mock import AsyncMock, patch

import pytest

from agentos.gateway.provider import LiteLLMProvider, LLMProviderError
from agentos.types import ModelProvider


@pytest.fixture
def provider() -> LiteLLMProvider:
    return LiteLLMProvider(provider=ModelProvider.OPENAI)


def _make_litellm_response(
    content: str = "Hello world",
    prompt_tokens: int = 10,
    completion_tokens: int = 20,
) -> SimpleNamespace:
    """Build a fake litellm response object."""
    message = SimpleNamespace(content=content)
    choice = SimpleNamespace(message=message)
    usage = SimpleNamespace(
        prompt_tokens=prompt_tokens,
        completion_tokens=completion_tokens,
    )
    return SimpleNamespace(choices=[choice], usage=usage)


@pytest.mark.asyncio
async def test_complete_success(provider: LiteLLMProvider) -> None:
    fake_resp = _make_litellm_response(content="Test response")

    with patch("agentos.gateway.provider.litellm") as mock_litellm:
        mock_litellm.acompletion = AsyncMock(return_value=fake_resp)
        mock_litellm.completion_cost.return_value = 0.001
        mock_litellm.suppress_debug_info = True

        response = await provider.complete(
            model="gpt-4o-mini",
            prompt="Say hello",
            system_prompt="You are helpful.",
            max_tokens=100,
            temperature=0.5,
        )

    assert response.content == "Test response"
    assert response.model == "gpt-4o-mini"
    assert response.provider == "openai"
    assert response.latency_ms > 0


@pytest.mark.asyncio
async def test_complete_extracts_tokens_and_cost(provider: LiteLLMProvider) -> None:
    fake_resp = _make_litellm_response(
        content="Result",
        prompt_tokens=50,
        completion_tokens=100,
    )

    with patch("agentos.gateway.provider.litellm") as mock_litellm:
        mock_litellm.acompletion = AsyncMock(return_value=fake_resp)
        mock_litellm.completion_cost.return_value = 0.0025
        mock_litellm.suppress_debug_info = True

        response = await provider.complete(
            model="gpt-4o-mini",
            prompt="Test",
        )

    assert response.tokens_in == 50
    assert response.tokens_out == 100
    assert response.cost_estimate > 0


@pytest.mark.asyncio
async def test_complete_handles_rate_limit(provider: LiteLLMProvider) -> None:
    exc = Exception("Rate limit exceeded")
    exc.status_code = 429  # type: ignore[attr-defined]

    with patch("agentos.gateway.provider.litellm") as mock_litellm:
        mock_litellm.acompletion = AsyncMock(side_effect=exc)
        mock_litellm.suppress_debug_info = True

        with pytest.raises(LLMProviderError) as exc_info:
            await provider.complete(model="gpt-4o-mini", prompt="Test")

    assert exc_info.value.retryable is True
    assert exc_info.value.status_code == 429
    assert exc_info.value.provider == "openai"


@pytest.mark.asyncio
async def test_complete_handles_auth_error(provider: LiteLLMProvider) -> None:
    exc = Exception("Invalid API key")
    exc.status_code = 401  # type: ignore[attr-defined]

    with patch("agentos.gateway.provider.litellm") as mock_litellm:
        mock_litellm.acompletion = AsyncMock(side_effect=exc)
        mock_litellm.suppress_debug_info = True

        with pytest.raises(LLMProviderError) as exc_info:
            await provider.complete(model="gpt-4o-mini", prompt="Test")

    assert exc_info.value.retryable is False
    assert exc_info.value.status_code == 401


@pytest.mark.asyncio
async def test_health_check(provider: LiteLLMProvider) -> None:
    with patch("agentos.gateway.provider.litellm") as mock_litellm:
        mock_litellm.model_list = []
        result = await provider.health_check()

    assert result is True


def test_supports_model(provider: LiteLLMProvider) -> None:
    assert provider.supports_model("gpt-4o-mini") is True
    assert provider.supports_model("gpt-4o") is True
    assert provider.supports_model("claude-3-haiku-20240307") is False


def test_supports_model_with_prefixes() -> None:
    prov = LiteLLMProvider(
        provider=ModelProvider.ANTHROPIC,
        model_prefixes=["claude-"],
    )
    assert prov.supports_model("claude-3-haiku-20240307") is True
    assert prov.supports_model("gpt-4o") is False
