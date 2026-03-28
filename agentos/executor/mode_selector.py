"""Smart mode selection for task execution (AOS-017).

Chooses between API, CLI, and Screen execution modes based on
task type, playbook permissions, and priority rules.
"""

from __future__ import annotations

import enum
from dataclasses import dataclass

from agentos.types import TaskType
from agentos.utils.logging import get_logger

logger = get_logger("executor.mode_selector")


class ExecutorMode(enum.StrEnum):
    """Available execution modes."""

    API = "api"
    CLI = "cli"
    SCREEN = "screen"


@dataclass(frozen=True)
class ModeDecision:
    """Result of mode selection with fallback chain."""

    selected_mode: ExecutorMode
    reason: str
    available_modes: list[ExecutorMode]
    fallback_chain: list[ExecutorMode]


class ModeSelector:
    """Selects optimal execution mode: API > CLI > Screen."""

    def select(
        self,
        task_type: TaskType,
        playbook_permissions: list[str] | None = None,
        forced_mode: ExecutorMode | None = None,
    ) -> ModeDecision:
        """Select the best execution mode for a task.

        Priority rules:
            1. Forced mode (user override) wins.
            2. VISION tasks require SCREEN mode.
            3. Default priority: API > CLI > SCREEN.
            4. Fall back to CLI if nothing is available.
        """
        permissions = playbook_permissions or []
        available = self._get_available_modes(permissions)

        # Rule 1: Forced mode
        if forced_mode and (forced_mode in available or not available):
            return ModeDecision(
                selected_mode=forced_mode,
                reason=f"Mode forced by user: {forced_mode}",
                available_modes=available,
                fallback_chain=[],
            )

        # Rule 2: VISION tasks -> SCREEN
        if task_type == TaskType.VISION and ExecutorMode.SCREEN in available:
            fallback = [m for m in available if m != ExecutorMode.SCREEN]
            return ModeDecision(
                selected_mode=ExecutorMode.SCREEN,
                reason="Vision task requires screen control",
                available_modes=available,
                fallback_chain=fallback,
            )

        # Rule 3: Default priority API > CLI > SCREEN
        priority = [ExecutorMode.API, ExecutorMode.CLI, ExecutorMode.SCREEN]
        for mode in priority:
            if mode in available:
                fallback = [m for m in priority if m in available and m != mode]
                return ModeDecision(
                    selected_mode=mode,
                    reason=f"Highest priority available mode: {mode}",
                    available_modes=available,
                    fallback_chain=fallback,
                )

        # No modes available -- default to CLI (text response only)
        return ModeDecision(
            selected_mode=ExecutorMode.CLI,
            reason="No execution modes available, default CLI",
            available_modes=[],
            fallback_chain=[],
        )

    def _get_available_modes(self, permissions: list[str]) -> list[ExecutorMode]:
        """Derive available modes from playbook permissions."""
        modes: list[ExecutorMode] = []
        if "cli" in permissions:
            modes.append(ExecutorMode.CLI)
        if "screen" in permissions:
            modes.append(ExecutorMode.SCREEN)
        # API mode: Phase 3+ (not available yet)
        if not modes and not permissions:
            # No playbook = default to CLI available
            modes.append(ExecutorMode.CLI)
        return modes
