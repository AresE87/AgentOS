"""Tests for Skill Replication (AOS-065)."""

from __future__ import annotations

from typing import TYPE_CHECKING
from unittest.mock import MagicMock

if TYPE_CHECKING:
    from pathlib import Path

import pytest

from agentos.mesh.protocol import MessageType
from agentos.mesh.replication import SkillReplicator


def _create_playbook(base: Path, name: str) -> Path:
    """Create a minimal playbook directory."""
    d = base / name
    d.mkdir(parents=True)
    (d / "playbook.md").write_text(f"# {name}\nInstructions here.")
    (d / "config.yaml").write_text(
        f"name: {name}\nversion: '1.0.0'\nauthor: test\ndescription: test skill\n"
    )
    return d


class TestSkillReplicator:
    def test_available_skills(self, tmp_path: Path) -> None:
        """Lists playbooks from directory."""
        _create_playbook(tmp_path, "summarize")
        _create_playbook(tmp_path, "translate")
        # Create a non-playbook dir (no playbook.md)
        (tmp_path / "not_a_skill").mkdir()
        replicator = SkillReplicator(tmp_path)
        assert replicator.available_skills == ["summarize", "translate"]

    def test_has_skill(self, tmp_path: Path) -> None:
        """Known skill -> True, unknown -> False."""
        _create_playbook(tmp_path, "summarize")
        replicator = SkillReplicator(tmp_path)
        assert replicator.has_skill("summarize") is True
        assert replicator.has_skill("unknown") is False

    @pytest.mark.asyncio
    async def test_pack_for_transfer(self, tmp_path: Path) -> None:
        """Packs existing playbook -> bytes."""
        _create_playbook(tmp_path, "summarize")
        mock_packager = MagicMock()

        async def mock_pack(folder: Path, output: Path) -> Path:
            output.write_bytes(b"FAKE_AOSP_DATA")
            return output

        mock_packager.pack = mock_pack
        replicator = SkillReplicator(tmp_path, packager=mock_packager)
        result = await replicator.pack_for_transfer("summarize")
        assert result == b"FAKE_AOSP_DATA"

    @pytest.mark.asyncio
    async def test_pack_missing_skill(self, tmp_path: Path) -> None:
        """Packing a missing skill returns None."""
        replicator = SkillReplicator(tmp_path)
        result = await replicator.pack_for_transfer("nonexistent")
        assert result is None

    @pytest.mark.asyncio
    async def test_receive_transfer(self, tmp_path: Path) -> None:
        """Receive bytes -> unpacks to directory."""
        mock_packager = MagicMock()

        async def mock_unpack(aosp_path: Path, target: Path) -> Path:
            target.mkdir(parents=True, exist_ok=True)
            (target / "playbook.md").write_text("# received")
            return target

        mock_packager.unpack = mock_unpack
        replicator = SkillReplicator(tmp_path, packager=mock_packager)
        assert not replicator.has_skill("new_skill")

        result = await replicator.receive_transfer("new_skill", b"FAKE_DATA")
        assert result == tmp_path / "new_skill"
        assert replicator.has_skill("new_skill")

    def test_create_messages(self, tmp_path: Path) -> None:
        """Creates valid request/transfer messages."""
        replicator = SkillReplicator(tmp_path)

        req = replicator.create_skill_request("node-a", "summarize")
        assert req.type == MessageType.SKILL_REQUEST
        assert req.sender_id == "node-a"
        assert req.payload["skill_name"] == "summarize"

        xfer = replicator.create_skill_transfer("node-b", "summarize", "base64data")
        assert xfer.type == MessageType.SKILL_TRANSFER
        assert xfer.sender_id == "node-b"
        assert xfer.payload["skill_name"] == "summarize"
        assert xfer.payload["data"] == "base64data"
