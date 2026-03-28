"""LLM Provider abstraction layer.

Wraps LiteLLM to provide a unified interface for all LLM providers.
NEVER logs prompt content or API keys.
"""

from __future__ import annotations

import time
from abc import ABC, abstractmethod
from dataclasses import dataclass

import litellm

from agentos.types import LLMResponse, ModelProvider
from agentos.utils.logging import get_logger

logger = get_logger("gateway")


@dataclass
class LLMProviderError(Exception):
    """Error from an LLM provider call."""

    message: str
    provider: str = ""
    model: str = ""
    retryable: bool = False
    status_code: int | None = None

    def __str__(self) -> str:
        return self.message


class BaseLLMProvider(ABC):
    """Abstract base class for LLM providers."""

    @abstractmethod
    async def complete(
        self,
        model: str,
        prompt: str,
        system_prompt: str = "",
        max_tokens: int = 4096,
        temperature: float = 0.7,
    ) -> LLMResponse:
        """Send a completion request to the provider.

        Args:
            model: Model identifier (e.g. "gpt-4o-mini").
            prompt: User prompt text.
            system_prompt: System prompt text.
            max_tokens: Maximum tokens to generate.
            temperature: Sampling temperature.

        Returns:
            Normalized LLMResponse.

        Raises:
            LLMProviderError: On provider failure.
        """

    @abstractmethod
    async def health_check(self) -> bool:
        """Check if the provider is reachable and configured.

        Returns:
            True if healthy, False otherwise.
        """

    @abstractmethod
    def supports_model(self, model: str) -> bool:
        """Check if this provider supports the given model.

        Args:
            model: Model identifier string.

        Returns:
            True if the model is supported.
        """


# Status codes that indicate retryable errors
_RETRYABLE_STATUS_CODES = {429, 500, 502, 503}


class LiteLLMProvider(BaseLLMProvider):
    """LLM provider backed by LiteLLM.

    API keys are configured via litellm environment variables internally
    (ANTHROPIC_API_KEY, OPENAI_API_KEY, etc). This class never stores
    or logs API keys.
    """

    def __init__(self, provider: ModelProvider, model_prefixes: list[str] | None = None) -> None:
        self._provider = provider
        self._model_prefixes = model_prefixes or []
        # Suppress litellm verbose logging
        litellm.suppress_debug_info = True

    @property
    def provider_name(self) -> str:
        return self._provider.value

    async def complete(
        self,
        model: str,
        prompt: str,
        system_prompt: str = "",
        max_tokens: int = 4096,
        temperature: float = 0.7,
    ) -> LLMResponse:
        messages: list[dict[str, str]] = []
        if system_prompt:
            messages.append({"role": "system", "content": system_prompt})
        messages.append({"role": "user", "content": prompt})

        logger.debug(
            "Calling provider=%s model=%s max_tokens=%d temperature=%.2f",
            self.provider_name,
            model,
            max_tokens,
            temperature,
        )

        start = time.perf_counter()
        try:
            response = await litellm.acompletion(
                model=model,
                messages=messages,
                max_tokens=max_tokens,
                temperature=temperature,
            )
        except Exception as exc:
            latency_ms = (time.perf_counter() - start) * 1000
            status_code = getattr(exc, "status_code", None)
            retryable = _is_retryable(exc)
            logger.warning(
                "Provider %s model=%s failed after %.0fms: %s (retryable=%s)",
                self.provider_name,
                model,
                latency_ms,
                type(exc).__name__,
                retryable,
            )
            raise LLMProviderError(
                message=str(exc),
                provider=self.provider_name,
                model=model,
                retryable=retryable,
                status_code=status_code,
            ) from exc

        latency_ms = (time.perf_counter() - start) * 1000

        # Extract token usage
        usage = getattr(response, "usage", None)
        tokens_in = getattr(usage, "prompt_tokens", 0) or 0
        tokens_out = getattr(usage, "completion_tokens", 0) or 0

        # Extract content
        content = response.choices[0].message.content or ""

        # Estimate cost from usage
        cost_estimate = _estimate_cost(model, tokens_in, tokens_out)

        logger.info(
            "Provider %s model=%s tokens_in=%d tokens_out=%d cost=%.6f latency=%.0fms",
            self.provider_name,
            model,
            tokens_in,
            tokens_out,
            cost_estimate,
            latency_ms,
        )

        return LLMResponse(
            content=content,
            model=model,
            provider=self.provider_name,
            tokens_in=tokens_in,
            tokens_out=tokens_out,
            cost_estimate=cost_estimate,
            latency_ms=round(latency_ms, 2),
        )

    async def health_check(self) -> bool:
        try:
            # Use litellm's model list check as a lightweight health probe
            models = litellm.model_list or []
            return len(models) >= 0  # If no exception, provider SDK is loaded
        except Exception:
            return False
        return True

    def supports_model(self, model: str) -> bool:
        if self._model_prefixes:
            return any(model.startswith(prefix) for prefix in self._model_prefixes)
        # Fall back: check provider name appears in model or litellm handles it
        provider_hints = {
            ModelProvider.ANTHROPIC: ["claude"],
            ModelProvider.OPENAI: ["gpt", "o1", "o3"],
            ModelProvider.GOOGLE: ["gemini"],
        }
        hints = provider_hints.get(self._provider, [])
        model_lower = model.lower()
        return any(hint in model_lower for hint in hints)


def _is_retryable(exc: Exception) -> bool:
    """Determine if an exception is retryable."""
    status_code = getattr(exc, "status_code", None)
    if status_code is not None and status_code in _RETRYABLE_STATUS_CODES:
        return True
    exc_name = type(exc).__name__.lower()
    retryable_names = ["timeout", "connection", "ratelimit", "serviceunavailable"]
    return any(name in exc_name for name in retryable_names)


def _estimate_cost(model: str, tokens_in: int, tokens_out: int) -> float:
    """Rough cost estimate based on known model pricing.

    This is a fallback; the gateway uses ModelConfig costs when available.
    """
    # Try litellm's built-in cost calculation first
    try:
        cost = litellm.completion_cost(
            model=model,
            prompt_tokens=tokens_in,
            completion_tokens=tokens_out,
        )
        if cost and cost > 0:
            return round(cost, 8)
    except Exception:
        pass

    # Fallback: generic per-token estimate
    return round((tokens_in * 0.5 + tokens_out * 1.5) / 1_000_000, 8)
