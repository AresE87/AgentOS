"""Tests for Mesh Protocol (AOS-064)."""

from __future__ import annotations

import pytest

from agentos.mesh.protocol import (
    MeshMessage,
    MeshState,
    MessageRouter,
    MessageType,
)


class TestMessageSerialization:
    def test_message_serialization(self) -> None:
        """MeshMessage -> dict -> MeshMessage roundtrip."""
        msg = MeshMessage(
            type=MessageType.TASK_ASSIGN,
            sender_id="node-a",
            payload={"task_id": "t1", "text": "hello"},
            message_id="abc123",
            timestamp="2026-01-01T00:00:00+00:00",
            target_id="node-b",
        )
        d = msg.to_dict()
        restored = MeshMessage.from_dict(d)
        assert restored.type == msg.type
        assert restored.sender_id == msg.sender_id
        assert restored.payload == msg.payload
        assert restored.message_id == msg.message_id
        assert restored.timestamp == msg.timestamp
        assert restored.target_id == msg.target_id

    def test_all_message_types(self) -> None:
        """All MessageType values are valid strings."""
        for mt in MessageType:
            assert isinstance(mt.value, str)
            assert len(mt.value) > 0
            msg = MeshMessage(type=mt, sender_id="n1", payload={})
            assert msg.type == mt


class TestMeshState:
    def test_mesh_state_update(self) -> None:
        """Update node -> get_available -> includes it."""
        state = MeshState()
        state.update_node("node-1", {"status": "online"})
        available = state.get_available_nodes()
        assert "node-1" in available

    def test_mesh_state_remove(self) -> None:
        """Remove node -> not in available."""
        state = MeshState()
        state.update_node("node-1", {"status": "online"})
        state.remove_node("node-1")
        assert "node-1" not in state.get_available_nodes()

    def test_get_node_with_skill(self) -> None:
        """Node with specialist -> found."""
        state = MeshState()
        state.update_node(
            "node-1",
            {"status": "online", "specialists": ["summarize", "translate"]},
        )
        state.update_node("node-2", {"status": "online", "specialists": ["code"]})
        assert state.get_node_with_skill("translate") == "node-1"
        assert state.get_node_with_skill("code") == "node-2"
        assert state.get_node_with_skill("unknown") is None

    def test_get_node_with_skill_ignores_offline(self) -> None:
        state = MeshState()
        state.update_node(
            "node-1",
            {"status": "offline", "specialists": ["translate"]},
        )
        assert state.get_node_with_skill("translate") is None


class TestMessageRouter:
    @pytest.mark.asyncio
    async def test_message_router_dispatch(self) -> None:
        """Register handler -> dispatch -> handler called."""
        router = MessageRouter("node-a")
        received: list[MeshMessage] = []

        async def handler(msg: MeshMessage) -> None:
            received.append(msg)

        router.on(MessageType.HEARTBEAT, handler)
        msg = MeshMessage(type=MessageType.HEARTBEAT, sender_id="node-b", payload={})
        await router.dispatch(msg)
        assert len(received) == 1
        assert received[0].sender_id == "node-b"

    @pytest.mark.asyncio
    async def test_router_skips_other_target(self) -> None:
        """Messages targeted at another node are not dispatched locally."""
        router = MessageRouter("node-a")
        received: list[MeshMessage] = []

        async def handler(msg: MeshMessage) -> None:
            received.append(msg)

        router.on(MessageType.HEARTBEAT, handler)
        msg = MeshMessage(
            type=MessageType.HEARTBEAT,
            sender_id="node-b",
            payload={},
            target_id="node-c",
        )
        await router.dispatch(msg)
        assert len(received) == 0

    @pytest.mark.asyncio
    async def test_router_delivers_to_local_target(self) -> None:
        """Messages targeted at local node are dispatched."""
        router = MessageRouter("node-a")
        received: list[MeshMessage] = []

        async def handler(msg: MeshMessage) -> None:
            received.append(msg)

        router.on(MessageType.TASK_ASSIGN, handler)
        msg = MeshMessage(
            type=MessageType.TASK_ASSIGN,
            sender_id="node-b",
            payload={},
            target_id="node-a",
        )
        await router.dispatch(msg)
        assert len(received) == 1
