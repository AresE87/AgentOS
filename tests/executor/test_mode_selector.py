"""Tests for ModeSelector (AOS-017)."""

from __future__ import annotations

import pytest

from agentos.executor.mode_selector import ExecutorMode, ModeDecision, ModeSelector
from agentos.types import TaskType


@pytest.fixture
def selector() -> ModeSelector:
    return ModeSelector()


# ── Tests ─────────────────────────────────────────────────────────────


def test_vision_task_selects_screen(selector: ModeSelector) -> None:
    """VISION task with screen permission selects SCREEN mode."""
    decision = selector.select(TaskType.VISION, playbook_permissions=["screen"])
    assert decision.selected_mode == ExecutorMode.SCREEN
    assert "vision" in decision.reason.lower() or "screen" in decision.reason.lower()


def test_code_task_selects_cli(selector: ModeSelector) -> None:
    """CODE task with cli permission selects CLI mode."""
    decision = selector.select(TaskType.CODE, playbook_permissions=["cli"])
    assert decision.selected_mode == ExecutorMode.CLI


def test_forced_mode(selector: ModeSelector) -> None:
    """Forced mode overrides all priority rules."""
    decision = selector.select(
        TaskType.CODE,
        playbook_permissions=["cli", "screen"],
        forced_mode=ExecutorMode.SCREEN,
    )
    assert decision.selected_mode == ExecutorMode.SCREEN
    assert "forced" in decision.reason.lower()


def test_cli_and_screen_available(selector: ModeSelector) -> None:
    """CODE task with [cli, screen] selects CLI with SCREEN in fallback."""
    decision = selector.select(TaskType.CODE, playbook_permissions=["cli", "screen"])
    assert decision.selected_mode == ExecutorMode.CLI
    assert ExecutorMode.SCREEN in decision.fallback_chain


def test_vision_with_cli_fallback(selector: ModeSelector) -> None:
    """VISION task with [cli, screen] selects SCREEN with CLI fallback."""
    decision = selector.select(TaskType.VISION, playbook_permissions=["cli", "screen"])
    assert decision.selected_mode == ExecutorMode.SCREEN
    assert ExecutorMode.CLI in decision.fallback_chain


def test_no_permissions(selector: ModeSelector) -> None:
    """No playbook permissions defaults to CLI."""
    decision = selector.select(TaskType.CODE)
    assert decision.selected_mode == ExecutorMode.CLI
    assert isinstance(decision, ModeDecision)


def test_text_task_no_execution(selector: ModeSelector) -> None:
    """TEXT task with empty permissions falls back to CLI as default."""
    decision = selector.select(TaskType.TEXT, playbook_permissions=[])
    assert decision.selected_mode == ExecutorMode.CLI
    assert ExecutorMode.CLI in decision.available_modes
