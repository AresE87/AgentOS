"""Tests for IPC server board/chain handlers."""

from __future__ import annotations

from unittest.mock import AsyncMock, MagicMock

import pytest

from agentos.ipc_server import IPCServer


@pytest.fixture
def server() -> IPCServer:
    """Create an IPCServer with mocked components."""
    srv = IPCServer()
    srv._agent = MagicMock()
    srv._store = MagicMock()
    srv._store.get_chain_history = AsyncMock(return_value=[])
    srv._store.get_chain_log = AsyncMock(return_value=[])
    return srv


@pytest.mark.asyncio
async def test_handle_get_active_chain(server: IPCServer) -> None:
    """get_active_chain returns idle state when no chain is active."""
    response = await server.handle_request(
        {"jsonrpc": "2.0", "method": "get_active_chain", "params": {}, "id": 1}
    )
    assert response is not None
    result = response["result"]
    assert result["status"] == "idle"
    assert response["id"] == 1


@pytest.mark.asyncio
async def test_handle_get_chain_history(server: IPCServer) -> None:
    """get_chain_history returns chains from store."""
    response = await server.handle_request(
        {"jsonrpc": "2.0", "method": "get_chain_history", "params": {}, "id": 2}
    )
    assert response is not None
    assert "chains" in response["result"]
    assert response["id"] == 2


@pytest.mark.asyncio
async def test_handle_send_chain_message(server: IPCServer) -> None:
    """send_chain_message returns ok."""
    response = await server.handle_request(
        {
            "jsonrpc": "2.0",
            "method": "send_chain_message",
            "params": {"message": "Please prioritize the report"},
            "id": 3,
        }
    )
    assert response is not None
    assert response["result"]["ok"] is True
    assert response["id"] == 3
