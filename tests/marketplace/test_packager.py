"""Tests for PlaybookPackager (.aosp archive creation and extraction)."""

from __future__ import annotations

import zipfile
from pathlib import Path

import pytest
import yaml

from agentos.marketplace.packager import PackagingError, PlaybookPackager

EXAMPLES_DIR = Path(__file__).resolve().parents[2] / "examples" / "playbooks"


# ── Helpers ──────────────────────────────────────────────────────────


def _make_playbook_folder(tmp_path: Path, *, extra_files: dict[str, str] | None = None) -> Path:
    """Create a minimal valid playbook folder under *tmp_path*."""
    folder = tmp_path / "my_playbook"
    folder.mkdir()
    (folder / "playbook.md").write_text("# Test Playbook\nDo something useful.\n")
    (folder / "config.yaml").write_text(
        yaml.dump(
            {
                "name": "test-playbook",
                "tier": 1,
                "timeout": 30,
                "permissions": ["cli"],
                "description": "A test playbook",
            }
        )
    )
    if extra_files:
        for rel, content in extra_files.items():
            p = folder / rel
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(content)
    return folder


# ── Tests ────────────────────────────────────────────────────────────


@pytest.fixture
def packager() -> PlaybookPackager:
    return PlaybookPackager()


async def test_pack_creates_aosp(packager: PlaybookPackager, tmp_path: Path) -> None:
    """Pack a valid playbook folder -> .aosp file exists on disk."""
    folder = _make_playbook_folder(tmp_path)
    out = await packager.pack(folder, output_path=tmp_path / "hello.aosp")
    assert out.exists()
    assert out.suffix == ".aosp"


async def test_pack_contains_required_files(packager: PlaybookPackager, tmp_path: Path) -> None:
    """The .aosp archive must contain playbook.md, config.yaml, metadata.yaml, checksum.sha256."""
    folder = _make_playbook_folder(tmp_path)
    out = await packager.pack(folder, output_path=tmp_path / "test.aosp")

    with zipfile.ZipFile(out) as zf:
        names = zf.namelist()
        assert "playbook.md" in names
        assert "config.yaml" in names
        assert "metadata.yaml" in names
        assert "checksum.sha256" in names


async def test_pack_excludes_prohibited(packager: PlaybookPackager, tmp_path: Path) -> None:
    """__pycache__/ and .env must NOT appear inside the .aosp."""
    folder = _make_playbook_folder(
        tmp_path,
        extra_files={
            "__pycache__/module.pyc": "bytecode",
            ".env": "SECRET=123",
            "scripts/helper.sh": "#!/bin/bash\necho hi",
        },
    )
    out = await packager.pack(folder, output_path=tmp_path / "test.aosp")

    with zipfile.ZipFile(out) as zf:
        names = zf.namelist()
        assert not any("__pycache__" in n for n in names)
        assert ".env" not in names
        # Allowed file should be present
        assert "scripts/helper.sh" in names


async def test_unpack_round_trip(packager: PlaybookPackager, tmp_path: Path) -> None:
    """Pack -> unpack -> files match original content."""
    folder = _make_playbook_folder(tmp_path, extra_files={"data/readme.txt": "hello"})
    aosp = await packager.pack(folder, output_path=tmp_path / "rt.aosp")
    dest = tmp_path / "unpacked"
    await packager.unpack(aosp, dest)

    # Original files should be present and identical
    assert (dest / "playbook.md").read_text() == (folder / "playbook.md").read_text()
    assert (dest / "config.yaml").read_text() == (folder / "config.yaml").read_text()
    assert (dest / "data" / "readme.txt").read_text() == "hello"
    # metadata.yaml should have been generated
    assert (dest / "metadata.yaml").exists()


async def test_unpack_verifies_checksums(packager: PlaybookPackager, tmp_path: Path) -> None:
    """Corrupting a file inside the .aosp raises PackagingError."""
    folder = _make_playbook_folder(tmp_path)
    aosp = await packager.pack(folder, output_path=tmp_path / "bad.aosp")

    # Corrupt a file inside the archive
    corrupted = tmp_path / "corrupted.aosp"
    with zipfile.ZipFile(aosp, "r") as src, zipfile.ZipFile(corrupted, "w") as dst:
        for item in src.namelist():
            data = src.read(item)
            if item == "playbook.md":
                data = b"CORRUPTED CONTENT"
            dst.writestr(item, data)

    with pytest.raises(PackagingError, match="Checksum mismatch"):
        await packager.unpack(corrupted, tmp_path / "out")


async def test_validate_metadata_valid(packager: PlaybookPackager) -> None:
    """Valid metadata returns no errors."""
    meta = {
        "name": "my-playbook",
        "version": "1.0.0",
        "author": "tester",
        "description": "A great playbook",
        "license": "free",
        "price": 0,
    }
    assert packager.validate_metadata(meta) == []


async def test_validate_metadata_missing_name(packager: PlaybookPackager) -> None:
    """Missing name field produces an error."""
    meta = {"version": "1.0.0", "author": "x", "description": "d"}
    errors = packager.validate_metadata(meta)
    assert any("name" in e for e in errors)


async def test_validate_metadata_negative_price(packager: PlaybookPackager) -> None:
    """Negative price produces an error."""
    meta = {
        "name": "x",
        "version": "1.0.0",
        "author": "x",
        "description": "d",
        "price": -1,
    }
    errors = packager.validate_metadata(meta)
    assert any("negative" in e.lower() or "price" in e.lower() for e in errors)


async def test_pack_missing_playbook(packager: PlaybookPackager, tmp_path: Path) -> None:
    """Folder without playbook.md raises PackagingError."""
    folder = tmp_path / "no_playbook"
    folder.mkdir()
    (folder / "config.yaml").write_text(yaml.dump({"name": "broken"}))

    with pytest.raises(PackagingError, match="Missing required file.*playbook.md"):
        await packager.pack(folder)


async def test_auto_generates_metadata(packager: PlaybookPackager, tmp_path: Path) -> None:
    """Folder without metadata.yaml auto-generates it from config.yaml."""
    folder = _make_playbook_folder(tmp_path)
    # Ensure no metadata.yaml in source
    assert not (folder / "metadata.yaml").exists()

    aosp = await packager.pack(folder, output_path=tmp_path / "autogen.aosp")

    with zipfile.ZipFile(aosp) as zf:
        meta = yaml.safe_load(zf.read("metadata.yaml"))
        assert meta["name"] == "test-playbook"
        assert meta["version"] == "1.0.0"
        assert meta["author"] == "local"
        assert meta["license"] == "free"
