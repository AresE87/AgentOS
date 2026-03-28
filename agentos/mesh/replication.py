"""Skill replication — transfer playbooks between nodes."""

from __future__ import annotations

import tempfile
from pathlib import Path

from agentos.marketplace.packager import PlaybookPackager
from agentos.mesh.protocol import MeshMessage, MessageType
from agentos.utils.logging import get_logger

logger = get_logger("mesh.replication")


class SkillReplicator:
    """Handles playbook transfer between mesh nodes."""

    def __init__(
        self,
        playbooks_dir: Path,
        packager: PlaybookPackager | None = None,
    ) -> None:
        self._playbooks_dir = playbooks_dir
        self._packager = packager or PlaybookPackager()
        self._local_skills: set[str] = set()
        self._refresh_skills()

    def _refresh_skills(self) -> None:
        self._local_skills.clear()
        if self._playbooks_dir.exists():
            for d in self._playbooks_dir.iterdir():
                if d.is_dir() and (d / "playbook.md").exists():
                    self._local_skills.add(d.name)

    @property
    def available_skills(self) -> list[str]:
        return sorted(self._local_skills)

    def has_skill(self, name: str) -> bool:
        return name in self._local_skills

    async def pack_for_transfer(self, skill_name: str) -> bytes | None:
        """Pack a playbook as .aosp bytes for transfer."""
        folder = self._playbooks_dir / skill_name
        if not folder.exists():
            return None
        with tempfile.NamedTemporaryFile(suffix=".aosp", delete=False) as tmp:
            tmp_path = Path(tmp.name)
        try:
            await self._packager.pack(folder, tmp_path)
            return tmp_path.read_bytes()
        finally:
            tmp_path.unlink(missing_ok=True)

    async def receive_transfer(self, skill_name: str, aosp_bytes: bytes) -> Path:
        """Receive and install a transferred playbook."""
        with tempfile.NamedTemporaryFile(suffix=".aosp", delete=False) as tmp:
            tmp.write(aosp_bytes)
            tmp_path = Path(tmp.name)
        try:
            target = self._playbooks_dir / skill_name
            await self._packager.unpack(tmp_path, target)
            self._local_skills.add(skill_name)
            logger.info("Received skill: %s", skill_name)
            return target
        finally:
            tmp_path.unlink(missing_ok=True)

    def create_skill_request(self, sender_id: str, skill_name: str) -> MeshMessage:
        return MeshMessage(
            type=MessageType.SKILL_REQUEST,
            sender_id=sender_id,
            payload={"skill_name": skill_name},
        )

    def create_skill_transfer(self, sender_id: str, skill_name: str, data_b64: str) -> MeshMessage:
        return MeshMessage(
            type=MessageType.SKILL_TRANSFER,
            sender_id=sender_id,
            payload={"skill_name": skill_name, "data": data_b64},
        )
