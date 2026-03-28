"""LLM Gateway facade: routing, fallback, cost control.

Orchestrates provider selection, cost estimation, and retry/fallback logic.
"""

from __future__ import annotations

import contextlib
from dataclasses import dataclass
from typing import TYPE_CHECKING

from agentos.gateway.provider import BaseLLMProvider, LLMProviderError
from agentos.types import (
    GatewayHealthStatus,
    LLMRequest,
    LLMResponse,
    ModelProvider,
)

if TYPE_CHECKING:
    from agentos.gateway.router import ModelRouter
    from agentos.settings import Settings
from agentos.utils.logging import get_logger

logger = get_logger("gateway")


class LLMGatewayError(Exception):
    """General gateway error."""


class LLMNoProvidersError(LLMGatewayError):
    """Raised when no providers are configured or reachable."""


@dataclass
class CostLimitExceededError(LLMGatewayError):
    """Raised when estimated cost exceeds the configured limit."""

    estimated_cost: float
    limit: float

    def __str__(self) -> str:
        return f"Estimated cost ${self.estimated_cost:.4f} exceeds limit ${self.limit:.4f}"


# Status codes that should NOT be retried
_NON_RETRYABLE_STATUS_CODES = {400, 401, 404}


class LLMGateway:
    """Facade for LLM completion with routing, fallback, and cost control.

    Args:
        settings: Application settings (contains max_cost_per_task, API keys).
        router: ModelRouter instance for selecting models.
        cost_tracker: Optional cost tracker (AOS-007). Not used yet.
        providers: Optional dict mapping provider name to BaseLLMProvider.
    """

    def __init__(
        self,
        settings: Settings,
        router: ModelRouter,
        cost_tracker: object | None = None,
        providers: dict[str, BaseLLMProvider] | None = None,
    ) -> None:
        self._settings = settings
        self._router = router
        self._cost_tracker = cost_tracker
        self._providers: dict[str, BaseLLMProvider] = providers or {}

    def register_provider(self, name: str, provider: BaseLLMProvider) -> None:
        """Register a provider instance."""
        self._providers[name] = provider

    async def complete(self, request: LLMRequest) -> LLMResponse:
        """Complete an LLM request with fallback chain.

        Selects models via the router, estimates cost before calling,
        and falls back to the next model on retryable errors.

        Args:
            request: The LLM request.

        Returns:
            LLMResponse from the first successful provider.

        Raises:
            CostLimitExceededError: If pre-call cost estimate exceeds limit.
            LLMNoProvidersError: If no providers are available.
            LLMGatewayError: If all models in the fallback chain fail.
        """
        available = list(self._providers.keys())
        if not available:
            raise LLMNoProvidersError("No providers registered with the gateway")

        models = self._router.select_models(
            task_type=request.task_type,
            tier=request.tier,
            available_providers=available,
        )

        last_error: Exception | None = None

        for model_config in models:
            # Pre-call cost estimation
            estimated_cost = _estimate_request_cost(
                model_config.cost_per_1m_input,
                model_config.cost_per_1m_output,
                request.max_tokens,
            )
            if estimated_cost > self._settings.max_cost_per_task:
                logger.warning(
                    "Skipping %s/%s: estimated cost $%.4f exceeds limit $%.4f",
                    model_config.provider.value,
                    model_config.model_id,
                    estimated_cost,
                    self._settings.max_cost_per_task,
                )
                last_error = CostLimitExceededError(
                    estimated_cost=estimated_cost,
                    limit=self._settings.max_cost_per_task,
                )
                continue

            provider = self._providers.get(model_config.provider.value)
            if provider is None:
                continue

            effective_max_tokens = min(request.max_tokens, model_config.max_tokens)

            try:
                response = await provider.complete(
                    model=model_config.model_id,
                    prompt=request.prompt,
                    system_prompt=request.system_prompt,
                    max_tokens=effective_max_tokens,
                    temperature=request.temperature,
                )
                return response
            except LLMProviderError as exc:
                last_error = exc
                if not exc.retryable and exc.status_code in _NON_RETRYABLE_STATUS_CODES:
                    logger.error(
                        "Non-retryable error from %s/%s (status=%s): %s",
                        model_config.provider.value,
                        model_config.model_id,
                        exc.status_code,
                        exc.message,
                    )
                    raise LLMGatewayError(f"Non-retryable provider error: {exc.message}") from exc
                # Retryable: log and try next model
                logger.warning(
                    "Retryable error from %s/%s, falling back: %s",
                    model_config.provider.value,
                    model_config.model_id,
                    exc.message,
                )
                continue

        # All models exhausted
        if isinstance(last_error, CostLimitExceededError):
            raise last_error

        raise LLMGatewayError(f"All models in fallback chain failed. Last error: {last_error}")

    async def health_check(self) -> GatewayHealthStatus:
        """Check health of all registered providers.

        Returns:
            GatewayHealthStatus with per-provider results.
        """
        provider_status: dict[str, bool] = {}
        for name, provider in self._providers.items():
            try:
                healthy = await provider.health_check()
            except Exception:
                healthy = False
            provider_status[name] = healthy

        all_models = self._router.all_models()
        healthy_providers = {name for name, ok in provider_status.items() if ok}
        available_count = sum(1 for m in all_models if m.provider.value in healthy_providers)

        default_provider: str | None = None
        if healthy_providers:
            # Pick the first healthy provider in a stable order
            for p in [ModelProvider.ANTHROPIC, ModelProvider.OPENAI, ModelProvider.GOOGLE]:
                if p.value in healthy_providers:
                    default_provider = p.value
                    break

        return GatewayHealthStatus(
            providers=provider_status,
            available_models=available_count,
            default_provider=default_provider,
        )

    def available_providers(self) -> list[ModelProvider]:
        """Return list of registered provider enums.

        Returns:
            List of ModelProvider values for registered providers.
        """
        result: list[ModelProvider] = []
        for name in self._providers:
            with contextlib.suppress(ValueError):
                result.append(ModelProvider(name))
        return result


def _estimate_request_cost(
    cost_per_1m_input: float,
    cost_per_1m_output: float,
    max_tokens: int,
) -> float:
    """Estimate worst-case cost for a request.

    Assumes ~500 input tokens (typical prompt) and max_tokens output.
    """
    estimated_input_tokens = 500
    input_cost = (estimated_input_tokens / 1_000_000) * cost_per_1m_input
    output_cost = (max_tokens / 1_000_000) * cost_per_1m_output
    return round(input_cost + output_cost, 6)
