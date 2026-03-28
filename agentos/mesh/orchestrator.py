"""Cross-node orchestrator — distributes tasks across the mesh."""

from __future__ import annotations

import asyncio
import time
from datetime import UTC, datetime
from typing import TYPE_CHECKING

from agentos.mesh.protocol import MeshMessage, MeshState, MessageType
from agentos.types import TaskInput, TaskResult, TaskStatus

if TYPE_CHECKING:
    from agentos.hierarchy.orchestrator import Orchestrator
from agentos.utils.logging import get_logger

logger = get_logger("mesh.orchestrator")


class MeshOrchestrator:
    """Extends Orchestrator to distribute work across mesh nodes."""

    def __init__(
        self,
        local_orchestrator: Orchestrator,
        mesh_state: MeshState,
        local_node_id: str,
        send_fn=None,  # noqa: ANN001
    ) -> None:
        self._local = local_orchestrator
        self._mesh = mesh_state
        self._node_id = local_node_id
        self._send = send_fn  # async (node_id, message) -> None
        self._pending_results: dict[str, asyncio.Future] = {}

    async def process(self, task_input: TaskInput) -> TaskResult:
        """Process task, potentially distributing to remote nodes."""
        start = time.monotonic()
        try:
            # Check if we should distribute
            available = self._mesh.get_available_nodes()
            if not available:
                return await self._local.process(task_input)

            # For now: simple heuristic — execute locally
            # Complex distribution logic would go here
            return await self._local.process(task_input)
        except Exception as e:
            elapsed = (time.monotonic() - start) * 1000
            return TaskResult(
                task_id=task_input.task_id,
                input_text=task_input.text,
                source=task_input.source,
                status=TaskStatus.FAILED,
                error_message=str(e),
                duration_ms=elapsed,
                created_at=task_input.created_at,
                completed_at=datetime.now(UTC),
            )

    async def assign_to_node(self, node_id: str, task_input: TaskInput) -> TaskResult | None:
        """Send task to a remote node and wait for result."""
        if not self._send:
            return None
        msg = MeshMessage(
            type=MessageType.TASK_ASSIGN,
            sender_id=self._node_id,
            target_id=node_id,
            payload={
                "task_id": task_input.task_id,
                "text": task_input.text,
                "source": task_input.source,
            },
        )
        future: asyncio.Future = asyncio.get_event_loop().create_future()
        self._pending_results[task_input.task_id] = future
        await self._send(node_id, msg)
        try:
            result = await asyncio.wait_for(future, timeout=300.0)
            return result
        except TimeoutError:
            logger.warning("Task %s timed out on node %s", task_input.task_id, node_id)
            return None
        finally:
            self._pending_results.pop(task_input.task_id, None)

    async def handle_task_result(self, message: MeshMessage) -> None:
        """Handle incoming task result from remote node."""
        task_id = message.payload.get("task_id", "")
        if task_id in self._pending_results:
            result = TaskResult(
                task_id=task_id,
                input_text=message.payload.get("input_text", ""),
                source="mesh",
                status=TaskStatus(message.payload.get("status", "completed")),
                output_text=message.payload.get("output", ""),
                cost_estimate=message.payload.get("cost", 0.0),
                duration_ms=message.payload.get("duration_ms", 0.0),
                completed_at=datetime.now(UTC),
            )
            self._pending_results[task_id].set_result(result)

    def select_best_node(self, required_skill: str | None = None) -> str | None:
        """Select the best node for a task."""
        if required_skill:
            return self._mesh.get_node_with_skill(required_skill)
        available = self._mesh.get_available_nodes()
        # Simple: pick least busy
        return available[0] if available else None
