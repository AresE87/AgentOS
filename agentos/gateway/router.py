"""Model routing: selects models based on task type, tier, and availability.

Loads routing.yaml and provides ordered model selection with validation.
"""

from __future__ import annotations

import re
from typing import TYPE_CHECKING

import yaml

if TYPE_CHECKING:
    from pathlib import Path

from agentos.types import LLMTier, ModelConfig, ModelProvider, TaskType
from agentos.utils.logging import get_logger

logger = get_logger("gateway")


class RoutingConfigError(Exception):
    """Raised when routing.yaml is invalid or cannot be loaded."""


class NoModelsAvailableError(Exception):
    """Raised when no models match the requested task_type/tier/providers."""


class ModelNotFoundError(Exception):
    """Raised when a specific model is not found in the config."""


_URL_PATTERN = re.compile(r"https?://")
_MAX_TOKENS_CAP = 100_000


class ModelRouter:
    """Selects and orders models from routing.yaml."""

    def __init__(self, config_path: Path) -> None:
        self._config_path = config_path
        self._models: dict[tuple[str, str], ModelConfig] = {}
        self._routing: dict[str, dict[int, list[tuple[str, str]]]] = {}
        self._load_config()

    def _load_config(self) -> None:
        """Load and validate routing.yaml."""
        try:
            raw = self._config_path.read_text(encoding="utf-8")
            data = yaml.safe_load(raw)
        except FileNotFoundError as exc:
            raise RoutingConfigError(f"Routing config not found: {self._config_path}") from exc
        except yaml.YAMLError as exc:
            raise RoutingConfigError(f"Invalid YAML in routing config: {exc}") from exc

        if not isinstance(data, dict):
            raise RoutingConfigError("Routing config must be a YAML mapping")

        providers_section = data.get("providers", {})
        if not providers_section:
            raise RoutingConfigError("No providers defined in routing config")

        # Parse provider models
        for provider_name, provider_data in providers_section.items():
            try:
                provider_enum = ModelProvider(provider_name)
            except ValueError as exc:
                raise RoutingConfigError(f"Unknown provider '{provider_name}'") from exc

            models = provider_data.get("models", {})
            for model_key, model_data in models.items():
                model_id = model_data.get("id", "")
                cost_in = model_data.get("cost_per_1m_input", 0.0)
                cost_out = model_data.get("cost_per_1m_output", 0.0)
                max_tokens = model_data.get("max_tokens", 4096)

                # Validation
                if _URL_PATTERN.search(model_id):
                    raise RoutingConfigError(f"Model ID must not contain URLs: {model_id}")
                if cost_in < 0 or cost_out < 0:
                    raise RoutingConfigError(f"Negative cost for model {provider_name}/{model_key}")
                if max_tokens > _MAX_TOKENS_CAP:
                    raise RoutingConfigError(
                        f"max_tokens {max_tokens} exceeds cap {_MAX_TOKENS_CAP} "
                        f"for {provider_name}/{model_key}"
                    )

                config = ModelConfig(
                    provider=provider_enum,
                    model_id=model_id,
                    display_name=model_key,
                    cost_per_1m_input=cost_in,
                    cost_per_1m_output=cost_out,
                    max_tokens=max_tokens,
                )
                self._models[(provider_name, model_key)] = config

        # Parse routing table
        routing_section = data.get("routing", {})
        for task_type_str, tiers in routing_section.items():
            tier_map: dict[int, list[tuple[str, str]]] = {}
            for tier_val, model_refs in tiers.items():
                tier_int = int(tier_val)
                parsed_refs: list[tuple[str, str]] = []
                for ref in model_refs:
                    parts = ref.split("/", 1)
                    if len(parts) != 2:
                        raise RoutingConfigError(f"Invalid model ref '{ref}' in routing table")
                    provider_name, model_key = parts
                    if (provider_name, model_key) not in self._models:
                        raise RoutingConfigError(
                            f"Model ref '{ref}' not found in providers section"
                        )
                    parsed_refs.append((provider_name, model_key))
                tier_map[tier_int] = parsed_refs
            self._routing[task_type_str] = tier_map

        logger.info(
            "Loaded routing config: %d models, %d task types",
            len(self._models),
            len(self._routing),
        )

    def select_models(
        self,
        task_type: TaskType,
        tier: LLMTier,
        available_providers: list[str] | None = None,
    ) -> list[ModelConfig]:
        """Select ordered list of models for a task.

        Args:
            task_type: The type of task being performed.
            tier: Budget tier for model selection.
            available_providers: If given, filter to only these provider names.

        Returns:
            Ordered list of ModelConfig, best-fit first.

        Raises:
            NoModelsAvailableError: If no models match.
        """
        tier_map = self._routing.get(task_type.value, {})
        refs = tier_map.get(tier.value, [])

        results: list[ModelConfig] = []
        for provider_name, model_key in refs:
            if available_providers and provider_name not in available_providers:
                continue
            config = self._models.get((provider_name, model_key))
            if config:
                results.append(config)

        if not results:
            raise NoModelsAvailableError(
                f"No models available for task_type={task_type.value} "
                f"tier={tier.value} providers={available_providers}"
            )

        return results

    def get_model_config(self, provider: str, model_name: str) -> ModelConfig:
        """Look up a specific model config.

        Args:
            provider: Provider name string (e.g. "anthropic").
            model_name: Display name / key (e.g. "haiku").

        Returns:
            The ModelConfig for that model.

        Raises:
            ModelNotFoundError: If not found.
        """
        config = self._models.get((provider, model_name))
        if config is None:
            raise ModelNotFoundError(f"Model not found: {provider}/{model_name}")
        return config

    def all_models(self) -> list[ModelConfig]:
        """Return all configured models.

        Returns:
            List of all ModelConfig entries.
        """
        return list(self._models.values())
