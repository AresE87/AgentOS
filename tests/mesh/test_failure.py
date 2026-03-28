"""Tests for Node Failure Handling (AOS-067)."""

from __future__ import annotations

import time

from agentos.mesh.failure import HEARTBEAT_TIMEOUT, NodeFailureDetector
from agentos.mesh.protocol import MeshState


class TestNodeFailureDetector:
    def test_record_heartbeat(self) -> None:
        """Records timestamp."""
        state = MeshState()
        detector = NodeFailureDetector(state)
        detector.record_heartbeat("node-1")
        assert "node-1" in detector._last_heartbeat
        assert abs(detector._last_heartbeat["node-1"] - time.time()) < 2.0
        assert state.get_available_nodes() == ["node-1"]

    def test_check_failures(self) -> None:
        """Old heartbeat -> detected as failed."""
        state = MeshState()
        detector = NodeFailureDetector(state)
        # Simulate an old heartbeat
        detector._last_heartbeat["node-1"] = time.time() - HEARTBEAT_TIMEOUT - 10
        state.update_node("node-1", {"status": "online"})

        failed = detector.check_failures()
        assert "node-1" in failed
        # Node should now be offline
        assert "node-1" not in state.get_available_nodes()

    def test_no_false_positive(self) -> None:
        """Recent heartbeat -> not failed."""
        state = MeshState()
        detector = NodeFailureDetector(state)
        detector.record_heartbeat("node-1")

        failed = detector.check_failures()
        assert failed == []
        assert "node-1" in state.get_available_nodes()

    def test_get_stranded_tasks(self) -> None:
        """Register tasks -> node fails -> tasks returned."""
        state = MeshState()
        detector = NodeFailureDetector(state)
        detector.register_task("node-1", "task-a")
        detector.register_task("node-1", "task-b")
        detector.register_task("node-2", "task-c")

        stranded = detector.get_stranded_tasks("node-1")
        assert stranded == ["task-a", "task-b"]
        # Second call returns empty (tasks were popped)
        assert detector.get_stranded_tasks("node-1") == []
        # node-2 tasks unaffected
        assert detector.get_stranded_tasks("node-2") == ["task-c"]

    def test_complete_task(self) -> None:
        """Completing a task removes it from pending."""
        state = MeshState()
        detector = NodeFailureDetector(state)
        detector.register_task("node-1", "task-a")
        detector.register_task("node-1", "task-b")
        detector.complete_task("node-1", "task-a")

        stranded = detector.get_stranded_tasks("node-1")
        assert stranded == ["task-b"]

    def test_complete_unknown_task(self) -> None:
        """Completing an unknown task does not raise."""
        state = MeshState()
        detector = NodeFailureDetector(state)
        detector.complete_task("node-1", "nonexistent")  # Should not raise

    def test_graceful_goodbye(self) -> None:
        """Returns pending tasks for reassignment."""
        state = MeshState()
        detector = NodeFailureDetector(state)
        detector.record_heartbeat("node-1")

        pending = detector.handle_graceful_goodbye("node-1", ["task-x", "task-y"])
        assert pending == ["task-x", "task-y"]
        # Node should be offline and heartbeat removed
        assert "node-1" not in state.get_available_nodes()
        assert "node-1" not in detector._last_heartbeat
