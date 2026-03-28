"""Tests for Cross-Node Orchestrator (AOS-066)."""

from __future__ import annotations

import asyncio
from datetime import UTC, datetime
from unittest.mock import AsyncMock

import pytest

from agentos.mesh.orchestrator import MeshOrchestrator
from agentos.mesh.protocol import MeshMessage, MeshState, MessageType
from agentos.types import TaskInput, TaskResult, TaskStatus


def _ok_result(text: str = "done") -> TaskResult:
    return TaskResult(
        task_id="t1",
        input_text="x",
        source="test",
        status=TaskStatus.COMPLETED,
        output_text=text,
        cost_estimate=0.01,
        created_at=datetime.now(UTC),
        completed_at=datetime.now(UTC),
    )


def _make_task_input(text: str = "Do something") -> TaskInput:
    return TaskInput(text=text, source="test", task_id="t1")


class TestMeshOrchestrator:
    @pytest.mark.asyncio
    async def test_process_no_remote_nodes(self) -> None:
        """Empty mesh -> local execution."""
        mock_orch = AsyncMock()
        mock_orch.process.return_value = _ok_result("local result")
        mesh_state = MeshState()

        mesh_orch = MeshOrchestrator(mock_orch, mesh_state, "node-a")
        task = _make_task_input("hello")
        result = await mesh_orch.process(task)

        assert result.status == TaskStatus.COMPLETED
        assert result.output_text == "local result"
        mock_orch.process.assert_awaited_once_with(task)

    @pytest.mark.asyncio
    async def test_process_with_remote_nodes_still_local(self) -> None:
        """With remote nodes available, still executes locally (current heuristic)."""
        mock_orch = AsyncMock()
        mock_orch.process.return_value = _ok_result("local")
        mesh_state = MeshState()
        mesh_state.update_node("node-b", {"status": "online"})

        mesh_orch = MeshOrchestrator(mock_orch, mesh_state, "node-a")
        result = await mesh_orch.process(_make_task_input())

        assert result.status == TaskStatus.COMPLETED
        mock_orch.process.assert_awaited_once()

    @pytest.mark.asyncio
    async def test_process_error_returns_failed(self) -> None:
        """Exception during processing returns FAILED result."""
        mock_orch = AsyncMock()
        mock_orch.process.side_effect = RuntimeError("boom")
        mesh_state = MeshState()

        mesh_orch = MeshOrchestrator(mock_orch, mesh_state, "node-a")
        result = await mesh_orch.process(_make_task_input())

        assert result.status == TaskStatus.FAILED
        assert "boom" in result.error_message

    @pytest.mark.asyncio
    async def test_handle_task_result(self) -> None:
        """Receive result -> resolves pending future."""
        mock_orch = AsyncMock()
        mesh_state = MeshState()
        mesh_orch = MeshOrchestrator(mock_orch, mesh_state, "node-a")

        # Create a pending future
        loop = asyncio.get_event_loop()
        future: asyncio.Future = loop.create_future()
        mesh_orch._pending_results["t1"] = future

        msg = MeshMessage(
            type=MessageType.TASK_RESULT,
            sender_id="node-b",
            payload={
                "task_id": "t1",
                "input_text": "hello",
                "status": "completed",
                "output": "result from remote",
                "cost": 0.05,
                "duration_ms": 100.0,
            },
        )
        await mesh_orch.handle_task_result(msg)

        assert future.done()
        result = future.result()
        assert result.task_id == "t1"
        assert result.output_text == "result from remote"
        assert result.status == TaskStatus.COMPLETED

    def test_select_best_node_with_skill(self) -> None:
        """Node has skill -> selected."""
        mesh_state = MeshState()
        mesh_state.update_node("node-b", {"status": "online", "specialists": ["translate"]})
        mesh_state.update_node("node-c", {"status": "online", "specialists": ["code"]})
        mock_orch = AsyncMock()
        mesh_orch = MeshOrchestrator(mock_orch, mesh_state, "node-a")

        assert mesh_orch.select_best_node("translate") == "node-b"
        assert mesh_orch.select_best_node("code") == "node-c"
        assert mesh_orch.select_best_node("unknown") is None

    def test_select_best_node_no_skill(self) -> None:
        """Without required skill, picks first available."""
        mesh_state = MeshState()
        mesh_state.update_node("node-b", {"status": "online"})
        mock_orch = AsyncMock()
        mesh_orch = MeshOrchestrator(mock_orch, mesh_state, "node-a")

        result = mesh_orch.select_best_node()
        assert result == "node-b"

    def test_select_best_node_empty_mesh(self) -> None:
        """No available nodes -> None."""
        mesh_state = MeshState()
        mock_orch = AsyncMock()
        mesh_orch = MeshOrchestrator(mock_orch, mesh_state, "node-a")

        assert mesh_orch.select_best_node() is None
