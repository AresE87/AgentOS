"""Local LLM provider — connects to Ollama or llama.cpp."""

from __future__ import annotations

import time

import httpx

from agentos.types import LLMResponse
from agentos.utils.logging import get_logger

logger = get_logger("gateway.local_provider")

DEFAULT_OLLAMA_URL = "http://localhost:11434"
DEFAULT_LLAMACPP_URL = "http://localhost:8080"


class LocalLLMProvider:
    """Provider for local LLMs via Ollama or llama.cpp HTTP APIs."""

    def __init__(
        self,
        base_url: str = DEFAULT_OLLAMA_URL,
        provider_type: str = "ollama",
    ) -> None:
        self._base_url = base_url.rstrip("/")
        self._type = provider_type
        self._client = httpx.AsyncClient(base_url=self._base_url, timeout=120.0)
        self._available = False

    async def health_check(self) -> bool:
        """Check if local LLM server is running."""
        try:
            if self._type == "ollama":
                resp = await self._client.get("/api/tags")
            else:
                resp = await self._client.get("/health")
            self._available = resp.status_code == 200
            return self._available
        except httpx.HTTPError:
            self._available = False
            return False

    @property
    def is_available(self) -> bool:
        return self._available

    async def complete(
        self,
        model: str,
        prompt: str,
        system_prompt: str = "",
        max_tokens: int = 4096,
        temperature: float = 0.7,
    ) -> LLMResponse:
        """Send completion request to local LLM."""
        start = time.monotonic()

        if self._type == "ollama":
            response = await self._ollama_complete(
                model, prompt, system_prompt, max_tokens, temperature
            )
        else:
            response = await self._openai_compatible_complete(
                model, prompt, system_prompt, max_tokens, temperature
            )

        latency = (time.monotonic() - start) * 1000
        return LLMResponse(
            content=response["content"],
            model=model,
            provider="local",
            tokens_in=response.get("tokens_in", 0),
            tokens_out=response.get("tokens_out", 0),
            cost_estimate=0.0,  # Local = free
            latency_ms=latency,
        )

    async def _ollama_complete(
        self,
        model: str,
        prompt: str,
        system_prompt: str,
        max_tokens: int,
        temperature: float,
    ) -> dict:
        """Ollama API format."""
        # Strip "ollama/" prefix if present
        model_name = model.replace("ollama/", "")

        payload = {
            "model": model_name,
            "prompt": prompt,
            "system": system_prompt,
            "stream": False,
            "options": {"temperature": temperature, "num_predict": max_tokens},
        }
        resp = await self._client.post("/api/generate", json=payload)
        resp.raise_for_status()
        data = resp.json()
        return {
            "content": data.get("response", ""),
            "tokens_in": data.get("prompt_eval_count", 0),
            "tokens_out": data.get("eval_count", 0),
        }

    async def _openai_compatible_complete(
        self,
        model: str,
        prompt: str,
        system_prompt: str,
        max_tokens: int,
        temperature: float,
    ) -> dict:
        """OpenAI-compatible API (llama.cpp server)."""
        messages: list[dict[str, str]] = []
        if system_prompt:
            messages.append({"role": "system", "content": system_prompt})
        messages.append({"role": "user", "content": prompt})

        payload = {
            "model": model,
            "messages": messages,
            "max_tokens": max_tokens,
            "temperature": temperature,
            "stream": False,
        }
        resp = await self._client.post("/v1/chat/completions", json=payload)
        resp.raise_for_status()
        data = resp.json()
        choice = data.get("choices", [{}])[0]
        usage = data.get("usage", {})
        return {
            "content": choice.get("message", {}).get("content", ""),
            "tokens_in": usage.get("prompt_tokens", 0),
            "tokens_out": usage.get("completion_tokens", 0),
        }

    def supports_model(self, model: str) -> bool:
        return model.startswith("ollama/") or model.startswith("local/")

    async def close(self) -> None:
        await self._client.aclose()
