"""Mesh protocol — message types and routing."""

from __future__ import annotations

import enum
import uuid
from dataclasses import dataclass, field
from datetime import UTC, datetime

from agentos.utils.logging import get_logger

logger = get_logger("mesh.protocol")


class MessageType(enum.StrEnum):
    NODE_HELLO = "node_hello"
    NODE_STATUS = "node_status"
    NODE_GOODBYE = "node_goodbye"
    TASK_ASSIGN = "task_assign"
    TASK_RESULT = "task_result"
    TASK_PROGRESS = "task_progress"
    SKILL_REQUEST = "skill_request"
    SKILL_TRANSFER = "skill_transfer"
    HEARTBEAT = "heartbeat"


@dataclass
class MeshMessage:
    type: MessageType
    sender_id: str
    payload: dict
    message_id: str = field(default_factory=lambda: uuid.uuid4().hex[:12])
    timestamp: str = field(default_factory=lambda: datetime.now(UTC).isoformat())
    target_id: str = ""  # Empty = broadcast

    def to_dict(self) -> dict:
        return {
            "type": self.type.value,
            "sender_id": self.sender_id,
            "payload": self.payload,
            "message_id": self.message_id,
            "timestamp": self.timestamp,
            "target_id": self.target_id,
        }

    @classmethod
    def from_dict(cls, data: dict) -> MeshMessage:
        return cls(
            type=MessageType(data["type"]),
            sender_id=data["sender_id"],
            payload=data.get("payload", {}),
            message_id=data.get("message_id", ""),
            timestamp=data.get("timestamp", ""),
            target_id=data.get("target_id", ""),
        )


@dataclass
class MeshState:
    """Current state of the mesh as seen by this node."""

    nodes: dict[str, dict] = field(default_factory=dict)  # node_id -> status info

    def update_node(self, node_id: str, status: dict) -> None:
        self.nodes[node_id] = {**status, "last_seen": datetime.now(UTC).isoformat()}

    def remove_node(self, node_id: str) -> None:
        self.nodes.pop(node_id, None)

    def get_available_nodes(self) -> list[str]:
        return [nid for nid, info in self.nodes.items() if info.get("status") != "offline"]

    def get_node_with_skill(self, skill_name: str) -> str | None:
        for nid, info in self.nodes.items():
            if skill_name in info.get("specialists", []) and info.get("status") != "offline":
                return nid
        return None


class MessageRouter:
    """Routes messages between nodes."""

    def __init__(self, local_node_id: str) -> None:
        self._local_id = local_node_id
        self._handlers: dict[MessageType, list] = {mt: [] for mt in MessageType}

    def on(self, msg_type: MessageType, handler) -> None:  # noqa: ANN001
        self._handlers[msg_type].append(handler)

    async def dispatch(self, message: MeshMessage) -> None:
        if message.target_id and message.target_id != self._local_id:
            logger.debug("Relaying message to %s", message.target_id)
            return  # TODO(AOS-064): relay to target
        for handler in self._handlers.get(message.type, []):
            try:
                await handler(message)
            except Exception:
                logger.exception("Handler error for %s", message.type)
