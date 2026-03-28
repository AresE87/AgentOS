"""Screen safety validation for AgentOS.

Validates screen actions before execution to prevent dangerous
hotkey combinations, secret leakage, and runaway action loops.
"""

from __future__ import annotations

import re

from agentos.utils.logging import get_logger

logger = get_logger("screen.safety")

BLOCKED_HOTKEYS: list[set[str]] = [
    {"alt", "f4"},
    {"ctrl", "alt", "delete"},
    {"ctrl", "alt", "backspace"},
    {"super", "l"},
    {"win", "l"},
    {"ctrl", "w"},  # Close tab
    {"ctrl", "q"},  # Quit app
]

SECRET_PATTERNS: list[re.Pattern[str]] = [
    re.compile(r"sk-[a-zA-Z0-9]{20,}"),
    re.compile(r"AIza[a-zA-Z0-9\-_]{30,}"),
    re.compile(r"ghp_[a-zA-Z0-9]{36,}"),
    re.compile(r"xox[bpars]-[a-zA-Z0-9\-]+"),
]


class ScreenSafetyError(Exception):
    """Raised when a screen action is blocked by safety checks."""

    def __init__(self, action: str, reason: str):
        self.action = action
        self.reason = reason
        super().__init__(f"Screen action blocked: {action} — {reason}")


class ScreenSafety:
    """Validates screen actions before execution.

    Checks coordinates are in bounds, hotkeys are not blocked,
    typed text does not contain secrets, and action count stays
    within a configurable limit.
    """

    def __init__(
        self,
        screen_width: int = 1920,
        screen_height: int = 1080,
        max_actions: int = 200,
    ):
        self._screen_w = screen_width
        self._screen_h = screen_height
        self._max_actions = max_actions
        self._action_count = 0

    def validate_click(self, x: int, y: int) -> None:
        """Validate that click coordinates are within screen bounds."""
        self._check_action_limit()
        if x < 0 or x > self._screen_w or y < 0 or y > self._screen_h:
            raise ScreenSafetyError("click", f"Coordinates ({x}, {y}) outside screen bounds")

    def validate_hotkey(self, keys: tuple[str, ...]) -> None:
        """Validate that a hotkey combination is not blocked."""
        self._check_action_limit()
        key_set = {k.lower() for k in keys}
        for blocked in BLOCKED_HOTKEYS:
            if blocked.issubset(key_set):
                raise ScreenSafetyError(
                    "hotkey",
                    f"Blocked hotkey combination: {'+'.join(keys)}",
                )

    def validate_type_text(self, text: str) -> None:
        """Validate that typed text does not contain secrets."""
        self._check_action_limit()
        for pattern in SECRET_PATTERNS:
            if pattern.search(text):
                raise ScreenSafetyError(
                    "type_text",
                    "Text contains what appears to be a secret/API key",
                )

    def _check_action_limit(self) -> None:
        """Increment action count and raise if limit exceeded."""
        self._action_count += 1
        if self._action_count > self._max_actions:
            raise ScreenSafetyError(
                "action_limit",
                f"Exceeded max {self._max_actions} actions per session",
            )

    def reset_count(self) -> None:
        """Reset the action counter."""
        self._action_count = 0
