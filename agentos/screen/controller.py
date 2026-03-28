"""Screen controller for AgentOS.

Uses pyautogui for mouse/keyboard automation with safety validation,
a pynput-based kill switch, and structured action logging.
"""

from __future__ import annotations

import asyncio
import time
from datetime import UTC, datetime
from typing import TYPE_CHECKING

import pyautogui
from pynput import keyboard

if TYPE_CHECKING:
    from agentos.screen.safety import ScreenSafety

from agentos.types import ScreenAction, ScreenActionType
from agentos.utils.logging import get_logger

logger = get_logger("screen.controller")

pyautogui.FAILSAFE = True  # Move mouse to corner to abort


class KillSwitchError(Exception):
    """Raised when the kill switch is activated."""


class ScreenController:
    """High-level screen automation with safety and kill switch.

    All public methods validate through ScreenSafety before executing,
    check the kill switch, and return a ScreenAction record.
    """

    def __init__(
        self,
        safety: ScreenSafety,
        move_duration: float = 0.3,
        type_interval: float = 0.05,
        action_delay: float = 0.5,
        kill_switch_key: str = "f12",
    ):
        self._safety = safety
        self._move_duration = move_duration
        self._type_interval = type_interval
        self._action_delay = action_delay
        self._kill_switch_key = kill_switch_key
        self._killed = False
        self._listener: keyboard.Listener | None = None
        self._action_log: list[ScreenAction] = []

    # ── Mouse actions ────────────────────────────────────────────

    async def click(self, x: int, y: int, button: str = "left") -> ScreenAction:
        """Click at (x, y) with the given mouse button."""
        self._check_kill_switch()
        self._safety.validate_click(x, y)
        return await self._do_action(
            ScreenActionType.CLICK,
            {"x": x, "y": y, "button": button},
            lambda: (
                pyautogui.moveTo(x, y, duration=self._move_duration),
                pyautogui.click(x, y, button=button),
            ),
        )

    async def double_click(self, x: int, y: int) -> ScreenAction:
        """Double-click at (x, y)."""
        self._check_kill_switch()
        self._safety.validate_click(x, y)
        return await self._do_action(
            ScreenActionType.DOUBLE_CLICK,
            {"x": x, "y": y},
            lambda: pyautogui.doubleClick(x, y),
        )

    async def right_click(self, x: int, y: int) -> ScreenAction:
        """Right-click at (x, y)."""
        self._check_kill_switch()
        self._safety.validate_click(x, y)
        return await self._do_action(
            ScreenActionType.RIGHT_CLICK,
            {"x": x, "y": y},
            lambda: pyautogui.rightClick(x, y),
        )

    async def drag(self, from_x: int, from_y: int, to_x: int, to_y: int) -> ScreenAction:
        """Drag from one point to another."""
        self._check_kill_switch()
        self._safety.validate_click(from_x, from_y)
        self._safety.validate_click(to_x, to_y)
        return await self._do_action(
            ScreenActionType.DRAG,
            {"from_x": from_x, "from_y": from_y, "to_x": to_x, "to_y": to_y},
            lambda: (
                pyautogui.moveTo(from_x, from_y, duration=self._move_duration),
                pyautogui.drag(
                    to_x - from_x,
                    to_y - from_y,
                    duration=self._move_duration,
                ),
            ),
        )

    # ── Keyboard actions ─────────────────────────────────────────

    async def type_text(self, text: str) -> ScreenAction:
        """Type text character by character."""
        self._check_kill_switch()
        self._safety.validate_type_text(text)
        log_text = text[:20] + "..." if len(text) > 50 else text
        return await self._do_action(
            ScreenActionType.TYPE,
            {"text": log_text},
            lambda: pyautogui.typewrite(text, interval=self._type_interval),
        )

    async def hotkey(self, *keys: str) -> ScreenAction:
        """Press a hotkey combination (e.g. ctrl+c)."""
        self._check_kill_switch()
        self._safety.validate_hotkey(keys)
        return await self._do_action(
            ScreenActionType.HOTKEY,
            {"keys": list(keys)},
            lambda: pyautogui.hotkey(*keys),
        )

    async def press_key(self, key: str) -> ScreenAction:
        """Press a single key."""
        self._check_kill_switch()
        return await self._do_action(
            ScreenActionType.PRESS_KEY,
            {"key": key},
            lambda: pyautogui.press(key),
        )

    # ── Other actions ────────────────────────────────────────────

    async def scroll(self, amount: int, x: int | None = None, y: int | None = None) -> ScreenAction:
        """Scroll the mouse wheel."""
        self._check_kill_switch()
        if x is not None and y is not None:
            self._safety.validate_click(x, y)
        return await self._do_action(
            ScreenActionType.SCROLL,
            {"amount": amount, "x": x, "y": y},
            lambda: pyautogui.scroll(amount, x=x, y=y),
        )

    async def move(self, x: int, y: int) -> ScreenAction:
        """Move the mouse to (x, y) smoothly."""
        self._check_kill_switch()
        self._safety.validate_click(x, y)
        return await self._do_action(
            ScreenActionType.MOVE,
            {"x": x, "y": y},
            lambda: pyautogui.moveTo(x, y, duration=self._move_duration),
        )

    async def wait(self, seconds: float) -> ScreenAction:
        """Wait for a duration without performing any screen action."""
        start = time.monotonic()
        await asyncio.sleep(seconds)
        return ScreenAction(
            action_type=ScreenActionType.WAIT,
            params={"seconds": seconds},
            timestamp=datetime.now(UTC),
            success=True,
            duration_ms=(time.monotonic() - start) * 1000,
        )

    # ── Internal execution ───────────────────────────────────────

    async def _do_action(
        self,
        action_type: ScreenActionType,
        params: dict[str, object],
        fn: object,
    ) -> ScreenAction:
        """Execute an action in a thread, log it, and return a ScreenAction."""
        start = time.monotonic()
        try:
            await asyncio.to_thread(fn)  # type: ignore[arg-type]
            await asyncio.sleep(self._action_delay)
            action = ScreenAction(
                action_type=action_type,
                params=params,
                timestamp=datetime.now(UTC),
                success=True,
                duration_ms=(time.monotonic() - start) * 1000,
            )
        except Exception as e:
            action = ScreenAction(
                action_type=action_type,
                params=params,
                timestamp=datetime.now(UTC),
                success=False,
                duration_ms=(time.monotonic() - start) * 1000,
                error=str(e),
            )
        self._action_log.append(action)
        return action

    # ── Kill switch ──────────────────────────────────────────────

    def start_kill_switch(self) -> None:
        """Start listening for the kill switch key (default F12)."""
        self._killed = False

        def on_press(key: keyboard.Key | keyboard.KeyCode | None) -> None:
            try:
                if hasattr(key, "name") and key.name == self._kill_switch_key:
                    self._killed = True
                    logger.warning("Kill switch activated!")
            except AttributeError:
                pass

        self._listener = keyboard.Listener(on_press=on_press)
        self._listener.daemon = True
        self._listener.start()

    def stop_kill_switch(self) -> None:
        """Stop the kill switch listener."""
        if self._listener:
            self._listener.stop()
            self._listener = None

    @property
    def is_killed(self) -> bool:
        """Whether the kill switch has been activated."""
        return self._killed

    def reset_kill_switch(self) -> None:
        """Reset the kill switch so actions can resume."""
        self._killed = False

    def _check_kill_switch(self) -> None:
        """Raise KillSwitchError if kill switch is active."""
        if self._killed:
            raise KillSwitchError("Kill switch activated — all screen actions halted")

    # ── Action log ───────────────────────────────────────────────

    def get_action_log(self) -> list[ScreenAction]:
        """Return a copy of all recorded actions."""
        return list(self._action_log)
