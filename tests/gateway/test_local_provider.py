"""Tests for LocalLLMProvider (AOS-053)."""

from __future__ import annotations

from unittest.mock import AsyncMock, patch

import httpx
import pytest

from agentos.gateway.local_provider import LocalLLMProvider

_FAKE_REQUEST = httpx.Request("POST", "http://localhost")


@pytest.fixture
def ollama_provider() -> LocalLLMProvider:
    return LocalLLMProvider(base_url="http://localhost:11434", provider_type="ollama")


@pytest.fixture
def llamacpp_provider() -> LocalLLMProvider:
    return LocalLLMProvider(base_url="http://localhost:8080", provider_type="llamacpp")


# ── health_check ──────────────────────────────────────────────────────


@pytest.mark.asyncio
async def test_health_check_ollama_available(ollama_provider: LocalLLMProvider) -> None:
    mock_response = httpx.Response(200, json={"models": []})
    with patch.object(
        ollama_provider._client, "get", new_callable=AsyncMock, return_value=mock_response
    ):
        result = await ollama_provider.health_check()
    assert result is True
    assert ollama_provider.is_available is True


@pytest.mark.asyncio
async def test_health_check_unavailable(ollama_provider: LocalLLMProvider) -> None:
    with patch.object(
        ollama_provider._client,
        "get",
        new_callable=AsyncMock,
        side_effect=httpx.ConnectError("refused"),
    ):
        result = await ollama_provider.health_check()
    assert result is False
    assert ollama_provider.is_available is False


# ── complete ──────────────────────────────────────────────────────────


@pytest.mark.asyncio
async def test_complete_ollama(ollama_provider: LocalLLMProvider) -> None:
    ollama_response = httpx.Response(
        200,
        request=_FAKE_REQUEST,
        json={
            "response": "Hello from llama3",
            "prompt_eval_count": 10,
            "eval_count": 20,
        },
    )
    with patch.object(
        ollama_provider._client, "post", new_callable=AsyncMock, return_value=ollama_response
    ):
        result = await ollama_provider.complete(
            model="ollama/llama3",
            prompt="Hi",
            system_prompt="Be helpful",
        )
    assert result.content == "Hello from llama3"
    assert result.provider == "local"
    assert result.tokens_in == 10
    assert result.tokens_out == 20
    assert result.cost_estimate == 0.0


@pytest.mark.asyncio
async def test_complete_openai_compatible(llamacpp_provider: LocalLLMProvider) -> None:
    openai_response = httpx.Response(
        200,
        request=_FAKE_REQUEST,
        json={
            "choices": [{"message": {"content": "Hello from llama.cpp"}}],
            "usage": {"prompt_tokens": 5, "completion_tokens": 15},
        },
    )
    with patch.object(
        llamacpp_provider._client, "post", new_callable=AsyncMock, return_value=openai_response
    ):
        result = await llamacpp_provider.complete(
            model="local/mistral",
            prompt="Hi",
        )
    assert result.content == "Hello from llama.cpp"
    assert result.provider == "local"
    assert result.tokens_in == 5
    assert result.tokens_out == 15
    assert result.cost_estimate == 0.0


# ── supports_model ────────────────────────────────────────────────────


def test_supports_model(ollama_provider: LocalLLMProvider) -> None:
    assert ollama_provider.supports_model("ollama/llama3") is True
    assert ollama_provider.supports_model("local/mistral") is True
    assert ollama_provider.supports_model("gpt-4o") is False
    assert ollama_provider.supports_model("claude-3-haiku-20240307") is False


# ── strips prefix ─────────────────────────────────────────────────────


@pytest.mark.asyncio
async def test_strips_ollama_prefix(ollama_provider: LocalLLMProvider) -> None:
    ollama_response = httpx.Response(
        200,
        request=_FAKE_REQUEST,
        json={"response": "ok", "prompt_eval_count": 1, "eval_count": 1},
    )
    mock_post = AsyncMock(return_value=ollama_response)
    with patch.object(ollama_provider._client, "post", mock_post):
        await ollama_provider.complete(model="ollama/llama3", prompt="test")

    # Verify the payload sent to Ollama uses "llama3" not "ollama/llama3"
    call_kwargs = mock_post.call_args
    payload = call_kwargs.kwargs.get("json") or call_kwargs[1].get("json")
    assert payload["model"] == "llama3"
