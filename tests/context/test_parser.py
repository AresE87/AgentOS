"""Tests for the Context Folder parser."""

from __future__ import annotations

import logging
from pathlib import Path

import pytest

from agentos.context.parser import (
    ConfigNotFoundError,
    ConfigValidationError,
    ContextFolderParser,
    PlaybookNotFoundError,
)
from agentos.types import LLMTier

EXAMPLES_DIR = Path(__file__).parent.parent.parent / "examples" / "playbooks"


@pytest.fixture
def parser() -> ContextFolderParser:
    return ContextFolderParser()


# ─── 1. Parse valid hello_world ──────────────────────────────────


@pytest.mark.asyncio
async def test_parse_hello_world(parser: ContextFolderParser) -> None:
    folder = await parser.parse(EXAMPLES_DIR / "hello_world")

    assert folder.config.name == "Hello World"
    assert folder.config.tier is LLMTier.CHEAP
    assert folder.config.timeout == 30
    assert "cli" in folder.config.permissions
    assert "echo" in folder.config.allowed_commands


# ─── 2. Parse valid system_monitor ───────────────────────────────


@pytest.mark.asyncio
async def test_parse_system_monitor(parser: ContextFolderParser) -> None:
    folder = await parser.parse(EXAMPLES_DIR / "system_monitor")

    assert folder.config.name == "System Monitor"
    assert len(folder.config.allowed_commands) >= 5


# ─── 3. Parse valid code_reviewer ────────────────────────────────


@pytest.mark.asyncio
async def test_parse_code_reviewer(parser: ContextFolderParser) -> None:
    folder = await parser.parse(EXAMPLES_DIR / "code_reviewer")

    assert folder.config.name == "Code Reviewer"
    assert folder.config.tier is LLMTier.PREMIUM
    assert folder.config.permissions == ["cli", "files"]


# ─── 4. Missing playbook.md ─────────────────────────────────────


@pytest.mark.asyncio
async def test_missing_playbook_md(parser: ContextFolderParser, tmp_path: Path) -> None:
    (tmp_path / "config.yaml").write_text("name: test\ntier: 1\n")
    with pytest.raises(PlaybookNotFoundError):
        await parser.parse(tmp_path)


# ─── 5. Missing config.yaml ─────────────────────────────────────


@pytest.mark.asyncio
async def test_missing_config_yaml(parser: ContextFolderParser, tmp_path: Path) -> None:
    (tmp_path / "playbook.md").write_text("# Test\n\nA description.\n")
    with pytest.raises(ConfigNotFoundError):
        await parser.parse(tmp_path)


# ─── 6. Missing name in config ──────────────────────────────────


@pytest.mark.asyncio
async def test_missing_name_in_config(parser: ContextFolderParser) -> None:
    with pytest.raises(ConfigValidationError) as exc_info:
        await parser.parse(EXAMPLES_DIR / "invalid_missing_name")
    assert any("name" in e for e in exc_info.value.errors)


# ─── 7. Invalid tier (7) ────────────────────────────────────────


@pytest.mark.asyncio
async def test_invalid_tier(parser: ContextFolderParser) -> None:
    with pytest.raises(ConfigValidationError) as exc_info:
        await parser.parse(EXAMPLES_DIR / "invalid_bad_tier")
    assert any("tier" in e for e in exc_info.value.errors)


# ─── 8. parse_many with mix of valid/invalid ────────────────────


@pytest.mark.asyncio
async def test_parse_many_mixed(
    parser: ContextFolderParser, caplog: pytest.LogCaptureFixture
) -> None:
    with caplog.at_level(logging.WARNING, logger="agentos.context"):
        results = await parser.parse_many(EXAMPLES_DIR)

    # 3 valid: hello_world, system_monitor, code_reviewer
    assert len(results) == 3
    names = {r.config.name for r in results}
    assert names == {"Hello World", "System Monitor", "Code Reviewer"}

    # 2 invalid folders should produce warnings
    warning_messages = [r.message for r in caplog.records if r.levelno >= logging.WARNING]
    assert len(warning_messages) >= 2


# ─── 9. Empty playbook.md uses directory name ───────────────────


@pytest.mark.asyncio
async def test_empty_playbook_uses_dir_name(parser: ContextFolderParser, tmp_path: Path) -> None:
    pb_dir = tmp_path / "my_playbook"
    pb_dir.mkdir()
    (pb_dir / "playbook.md").write_text("")
    (pb_dir / "config.yaml").write_text("name: My Playbook\ntier: 1\n")

    folder = await parser.parse(pb_dir)
    # instructions should be empty string, but parse should succeed
    assert folder.config.name == "My Playbook"
    assert folder.instructions == ""


# ─── 10. Config with unknown fields doesn't fail ────────────────


@pytest.mark.asyncio
async def test_config_ignores_unknown_fields(parser: ContextFolderParser, tmp_path: Path) -> None:
    (tmp_path / "playbook.md").write_text("# Extra Fields Test\n\nSome content.\n")
    (tmp_path / "config.yaml").write_text(
        "name: Extra\ntier: 2\nunknown_field: hello\nextra_stuff: 42\n"
    )

    folder = await parser.parse(tmp_path)
    assert folder.config.name == "Extra"
    assert folder.config.tier is LLMTier.STANDARD
