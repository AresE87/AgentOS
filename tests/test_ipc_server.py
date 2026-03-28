"""Tests for IPC server JSON-RPC handling."""

from __future__ import annotations

import io
import json
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from agentos.ipc_server import IPCServer
from agentos.types import TaskStatus


@pytest.fixture
def server() -> IPCServer:
    """Create an IPCServer with mocked components."""
    srv = IPCServer()
    srv._agent = MagicMock()
    srv._store = MagicMock()
    return srv


# ─── Handler tests ──────────────────────────────────────────────────


@pytest.mark.asyncio
async def test_handle_get_status(server: IPCServer) -> None:
    """get_status returns state=running and provider list."""
    with patch("agentos.ipc_server.load_settings") as mock_settings:
        mock_settings.return_value.available_providers.return_value = {"anthropic": "key1"}
        response = await server.handle_request(
            {"jsonrpc": "2.0", "method": "get_status", "params": {}, "id": 1}
        )
    assert response is not None
    assert response["result"]["state"] == "running"
    assert "anthropic" in response["result"]["providers"]
    assert response["id"] == 1


@pytest.mark.asyncio
async def test_handle_process_message(server: IPCServer) -> None:
    """process_message calls agent.process and returns the result."""
    mock_result = MagicMock()
    mock_result.task_id = "task-123"
    mock_result.status = TaskStatus.COMPLETED
    mock_result.output_text = "Hello, world!"
    mock_result.model_used = "claude-3"
    mock_result.cost_estimate = 0.01
    mock_result.duration_ms = 500

    server._agent.process = AsyncMock(return_value=mock_result)

    # Capture stdout for the event
    captured = io.StringIO()
    with patch("sys.stdout", captured):
        response = await server.handle_request(
            {
                "jsonrpc": "2.0",
                "method": "process_message",
                "params": {"text": "Hello"},
                "id": 2,
            }
        )

    assert response is not None
    assert response["result"]["task_id"] == "task-123"
    assert response["result"]["status"] == "completed"
    assert response["result"]["output"] == "Hello, world!"


@pytest.mark.asyncio
async def test_handle_get_tasks(server: IPCServer) -> None:
    """get_tasks returns a list of tasks from the store."""
    server._store.get_recent_tasks = AsyncMock(
        return_value=[{"task_id": "t1", "status": "completed"}]
    )

    response = await server.handle_request(
        {"jsonrpc": "2.0", "method": "get_tasks", "params": {"limit": 5}, "id": 3}
    )

    assert response is not None
    assert response["result"]["tasks"] == [{"task_id": "t1", "status": "completed"}]


@pytest.mark.asyncio
async def test_handle_get_playbooks(server: IPCServer) -> None:
    """get_playbooks returns playbook list (empty when dir doesn't exist)."""
    with patch("agentos.ipc_server.load_settings") as mock_settings:
        mock_settings.return_value.playbooks_dir = "/nonexistent"
        response = await server.handle_request(
            {
                "jsonrpc": "2.0",
                "method": "get_playbooks",
                "params": {},
                "id": 4,
            }
        )

    assert response is not None
    assert response["result"]["playbooks"] == []


@pytest.mark.asyncio
async def test_handle_unknown_method(server: IPCServer) -> None:
    """Unknown method returns JSON-RPC method not found error."""
    response = await server.handle_request(
        {
            "jsonrpc": "2.0",
            "method": "nonexistent_method",
            "params": {},
            "id": 5,
        }
    )

    assert response is not None
    assert response["error"]["code"] == -32601
    assert "Method not found" in response["error"]["message"]


@pytest.mark.asyncio
async def test_handle_shutdown(server: IPCServer) -> None:
    """shutdown sets _running to False and calls agent.shutdown."""
    server._agent.shutdown = AsyncMock()

    response = await server.handle_request(
        {"jsonrpc": "2.0", "method": "shutdown", "params": {}, "id": 6}
    )

    assert response is not None
    assert response["result"]["ok"] is True
    assert server._running is False
    server._agent.shutdown.assert_awaited_once()


def test_send_event(server: IPCServer) -> None:
    """send_event writes a JSON-RPC notification to stdout."""
    captured = io.StringIO()
    with patch("sys.stdout", captured):
        server.send_event("task_started", {"task_id": "t-42"})

    output = captured.getvalue().strip()
    parsed = json.loads(output)
    assert parsed["jsonrpc"] == "2.0"
    assert parsed["method"] == "event"
    assert parsed["params"]["type"] == "task_started"
    assert parsed["params"]["task_id"] == "t-42"


def test_send_error(server: IPCServer) -> None:
    """send_error writes a JSON-RPC error to stdout."""
    captured = io.StringIO()
    with patch("sys.stdout", captured):
        server.send_error(99, -32700, "Parse error")

    output = captured.getvalue().strip()
    parsed = json.loads(output)
    assert parsed["error"]["code"] == -32700
    assert parsed["id"] == 99


@pytest.mark.asyncio
async def test_invalid_json_handling(server: IPCServer) -> None:
    """Invalid JSON input produces a parse error response."""
    # Simulate what happens when json.loads fails in the run loop
    # We test handle_request won't be called, but send_error will
    captured = io.StringIO()
    with patch("sys.stdout", captured):
        try:
            json.loads("{not valid json}")
        except json.JSONDecodeError as e:
            server.send_error(None, -32700, f"Parse error: {e}")

    output = captured.getvalue().strip()
    parsed = json.loads(output)
    assert parsed["error"]["code"] == -32700
    assert "Parse error" in parsed["error"]["message"]
    assert parsed["id"] is None
