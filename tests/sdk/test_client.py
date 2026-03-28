"""Tests for AgentOS Python SDK (AOS-074)."""

from __future__ import annotations

from unittest.mock import AsyncMock, patch

import httpx
import pytest

from agentos.sdk.client import (
    AgentOS,
    AsyncAgentOS,
    AuthError,
    RateLimitError,
    TaskResult,
)


def _mock_response(status_code: int = 200, json_data: dict | None = None) -> httpx.Response:
    resp = httpx.Response(
        status_code,
        json=json_data or {},
        request=httpx.Request("GET", "http://test"),
    )
    return resp


def test_sync_client_run_task() -> None:
    json_data = {
        "data": {
            "task_id": "abc123",
            "status": "completed",
            "output": "Done!",
            "model": "gpt-4",
            "cost": 0.05,
            "duration_ms": 1200.0,
        }
    }
    mock_resp = _mock_response(200, json_data)

    with patch.object(httpx.Client, "request", return_value=mock_resp):
        client = AgentOS(api_key="test-key")
        result = client.run_task("hello world")

    assert isinstance(result, TaskResult)
    assert result.task_id == "abc123"
    assert result.status == "completed"
    assert result.output == "Done!"
    assert result.cost == 0.05


def test_sync_client_get_status() -> None:
    json_data = {"data": {"version": "0.1.0", "uptime": 3600}}
    mock_resp = _mock_response(200, json_data)

    with patch.object(httpx.Client, "request", return_value=mock_resp):
        client = AgentOS(api_key="test-key")
        status = client.get_status()

    assert status["version"] == "0.1.0"


def test_sync_client_auth_error() -> None:
    mock_resp = _mock_response(401, {"error": "unauthorized"})

    with patch.object(httpx.Client, "request", return_value=mock_resp):
        client = AgentOS(api_key="bad-key")
        with pytest.raises(AuthError) as exc_info:
            client.get_status()
        assert exc_info.value.status_code == 401


def test_sync_client_rate_limit() -> None:
    mock_resp = _mock_response(429, {"error": "rate limited"})

    with patch.object(httpx.Client, "request", return_value=mock_resp):
        client = AgentOS(api_key="test-key")
        with pytest.raises(RateLimitError) as exc_info:
            client.get_status()
        assert exc_info.value.status_code == 429


async def test_async_client_run_task() -> None:
    json_data = {
        "data": {
            "task_id": "async123",
            "status": "completed",
            "output": "Async done!",
        }
    }
    mock_resp = _mock_response(200, json_data)

    with patch.object(httpx.AsyncClient, "request", new_callable=AsyncMock, return_value=mock_resp):
        async with AsyncAgentOS(api_key="test-key") as client:
            result = await client.run_task("async hello")

    assert result.task_id == "async123"
    assert result.status == "completed"


def test_context_manager() -> None:
    json_data = {"data": {"healthy": True}}
    mock_resp = _mock_response(200, json_data)

    with patch.object(httpx.Client, "request", return_value=mock_resp):
        with AgentOS(api_key="test-key") as client:
            health = client.get_health()
        assert health["healthy"] is True
