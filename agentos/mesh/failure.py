"""Node failure detection and task reassignment."""

from __future__ import annotations

import contextlib
import time
from typing import TYPE_CHECKING

from agentos.utils.logging import get_logger

if TYPE_CHECKING:
    from agentos.mesh.protocol import MeshState

logger = get_logger("mesh.failure")

HEARTBEAT_TIMEOUT = 90.0  # 3 missed heartbeats at 30s interval


class NodeFailureDetector:
    """Detects node failures and triggers reassignment."""

    def __init__(self, mesh_state: MeshState) -> None:
        self._mesh = mesh_state
        self._last_heartbeat: dict[str, float] = {}
        self._pending_tasks: dict[str, list[str]] = {}  # node_id -> [task_ids]

    def record_heartbeat(self, node_id: str) -> None:
        self._last_heartbeat[node_id] = time.time()
        self._mesh.update_node(node_id, {"status": "online"})

    def check_failures(self) -> list[str]:
        """Check for failed nodes. Returns list of newly failed node IDs."""
        now = time.time()
        failed = []
        for node_id, last in list(self._last_heartbeat.items()):
            if now - last > HEARTBEAT_TIMEOUT:
                logger.warning(
                    "Node %s failed (no heartbeat for %.0fs)",
                    node_id,
                    now - last,
                )
                self._mesh.update_node(node_id, {"status": "offline"})
                failed.append(node_id)
        return failed

    def register_task(self, node_id: str, task_id: str) -> None:
        if node_id not in self._pending_tasks:
            self._pending_tasks[node_id] = []
        self._pending_tasks[node_id].append(task_id)

    def complete_task(self, node_id: str, task_id: str) -> None:
        if node_id in self._pending_tasks:
            with contextlib.suppress(ValueError):
                self._pending_tasks[node_id].remove(task_id)

    def get_stranded_tasks(self, node_id: str) -> list[str]:
        """Get tasks that were pending on a failed node."""
        return self._pending_tasks.pop(node_id, [])

    def handle_graceful_goodbye(self, node_id: str, pending_task_ids: list[str]) -> list[str]:
        """Handle a node's graceful shutdown."""
        logger.info(
            "Node %s disconnecting gracefully with %d pending tasks",
            node_id,
            len(pending_task_ids),
        )
        self._mesh.update_node(node_id, {"status": "offline"})
        self._last_heartbeat.pop(node_id, None)
        return pending_task_ids  # Return for reassignment
