"""AgentOS Python SDK -- official client library."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

import httpx

from agentos.utils.logging import get_logger

logger = get_logger("sdk")

DEFAULT_BASE_URL = "http://localhost:8080"


class AgentOSError(Exception):
    """Base SDK error."""

    def __init__(self, message: str, status_code: int | None = None):
        self.status_code = status_code
        super().__init__(message)


class AuthError(AgentOSError):
    """Authentication failed."""


class RateLimitError(AgentOSError):
    """Rate limit exceeded."""


class TaskError(AgentOSError):
    """Task execution error."""


@dataclass
class TaskResult:
    task_id: str
    status: str
    output: str = ""
    model: str | None = None
    cost: float = 0.0
    duration_ms: float = 0.0
    error: str | None = None


class AgentOS:
    """Synchronous AgentOS client."""

    def __init__(self, api_key: str, base_url: str = DEFAULT_BASE_URL) -> None:
        self._base_url = base_url.rstrip("/")
        self._client = httpx.Client(
            base_url=self._base_url,
            headers={"Authorization": f"Bearer {api_key}"},
            timeout=120.0,
        )

    def run_task(self, text: str, playbook: str | None = None) -> TaskResult:
        """Execute a task and return result."""
        body: dict[str, Any] = {"text": text, "source": "sdk"}
        if playbook:
            body["playbook"] = playbook
        resp = self._request("POST", "/api/v1/tasks", json=body)
        data = resp.get("data", {})
        return TaskResult(
            task_id=data.get("task_id", ""),
            status=data.get("status", "unknown"),
            output=data.get("output", ""),
            model=data.get("model"),
            cost=data.get("cost", 0.0),
            duration_ms=data.get("duration_ms", 0.0),
        )

    def get_task(self, task_id: str) -> TaskResult:
        resp = self._request("GET", f"/api/v1/tasks/{task_id}")
        data = resp.get("data", {})
        return TaskResult(
            task_id=data.get("task_id", task_id),
            status=data.get("status", "unknown"),
        )

    def list_tasks(self, page: int = 1, per_page: int = 20) -> list[dict]:
        resp = self._request("GET", "/api/v1/tasks", params={"page": page, "per_page": per_page})
        return resp.get("data", {}).get("tasks", [])

    def get_status(self) -> dict:
        return self._request("GET", "/api/v1/status").get("data", {})

    def get_health(self) -> dict:
        return self._request("GET", "/api/v1/health").get("data", {})

    def list_playbooks(self) -> list[dict]:
        return self._request("GET", "/api/v1/playbooks").get("data", {}).get("playbooks", [])

    def get_usage(self) -> dict:
        return self._request("GET", "/api/v1/usage").get("data", {})

    def _request(self, method: str, path: str, **kwargs: Any) -> dict:
        resp = self._client.request(method, path, **kwargs)
        if resp.status_code == 401:
            raise AuthError("Invalid API key", status_code=401)
        if resp.status_code == 429:
            raise RateLimitError("Rate limit exceeded", status_code=429)
        if resp.status_code >= 400:
            raise AgentOSError(f"API error: {resp.status_code}", status_code=resp.status_code)
        return resp.json()

    def close(self) -> None:
        self._client.close()

    def __enter__(self) -> AgentOS:
        return self

    def __exit__(self, *args: Any) -> None:
        self.close()


class AsyncAgentOS:
    """Async AgentOS client."""

    def __init__(self, api_key: str, base_url: str = DEFAULT_BASE_URL) -> None:
        self._base_url = base_url.rstrip("/")
        self._client = httpx.AsyncClient(
            base_url=self._base_url,
            headers={"Authorization": f"Bearer {api_key}"},
            timeout=120.0,
        )

    async def run_task(self, text: str, playbook: str | None = None) -> TaskResult:
        body: dict[str, Any] = {"text": text, "source": "sdk"}
        if playbook:
            body["playbook"] = playbook
        resp = await self._request("POST", "/api/v1/tasks", json=body)
        data = resp.get("data", {})
        return TaskResult(
            task_id=data.get("task_id", ""),
            status=data.get("status", "unknown"),
            output=data.get("output", ""),
            model=data.get("model"),
            cost=data.get("cost", 0.0),
            duration_ms=data.get("duration_ms", 0.0),
        )

    async def get_task(self, task_id: str) -> TaskResult:
        resp = await self._request("GET", f"/api/v1/tasks/{task_id}")
        data = resp.get("data", {})
        return TaskResult(
            task_id=data.get("task_id", task_id),
            status=data.get("status", "unknown"),
        )

    async def list_tasks(self, page: int = 1, per_page: int = 20) -> list[dict]:
        resp = await self._request(
            "GET", "/api/v1/tasks", params={"page": page, "per_page": per_page}
        )
        return resp.get("data", {}).get("tasks", [])

    async def get_status(self) -> dict:
        return (await self._request("GET", "/api/v1/status")).get("data", {})

    async def _request(self, method: str, path: str, **kwargs: Any) -> dict:
        resp = await self._client.request(method, path, **kwargs)
        if resp.status_code == 401:
            raise AuthError("Invalid API key", status_code=401)
        if resp.status_code == 429:
            raise RateLimitError("Rate limit exceeded", status_code=429)
        if resp.status_code >= 400:
            raise AgentOSError(f"API error: {resp.status_code}", status_code=resp.status_code)
        return resp.json()

    async def close(self) -> None:
        await self._client.aclose()

    async def __aenter__(self) -> AsyncAgentOS:
        return self

    async def __aexit__(self, *args: Any) -> None:
        await self.close()
