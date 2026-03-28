"""Tests for agentos.screen.controller.

All pyautogui calls are mocked — no actual mouse/keyboard movement.
"""

from __future__ import annotations

import time
from unittest.mock import MagicMock, patch

import pytest

from agentos.screen.controller import KillSwitchError, ScreenController
from agentos.screen.safety import ScreenSafety, ScreenSafetyError
from agentos.types import ScreenActionType


@pytest.fixture()
def safety() -> ScreenSafety:
    return ScreenSafety(screen_width=1920, screen_height=1080)


@pytest.fixture()
def controller(safety: ScreenSafety) -> ScreenController:
    return ScreenController(
        safety=safety,
        move_duration=0.0,
        type_interval=0.0,
        action_delay=0.0,
    )


# ── Click ────────────────────────────────────────────────────────


@patch("agentos.screen.controller.pyautogui")
async def test_click_calls_pyautogui(
    mock_pyautogui: MagicMock, controller: ScreenController
) -> None:
    action = await controller.click(100, 200)
    assert action.success is True
    assert action.action_type == ScreenActionType.CLICK
    mock_pyautogui.moveTo.assert_called_once()
    mock_pyautogui.click.assert_called_once_with(100, 200, button="left")


async def test_click_out_of_bounds(controller: ScreenController) -> None:
    with pytest.raises(ScreenSafetyError, match="outside screen bounds"):
        await controller.click(99999, 100)


# ── Hotkey ───────────────────────────────────────────────────────


async def test_hotkey_blocked_alt_f4(controller: ScreenController) -> None:
    with pytest.raises(ScreenSafetyError, match="Blocked hotkey"):
        await controller.hotkey("alt", "f4")


@patch("agentos.screen.controller.pyautogui")
async def test_hotkey_allowed(mock_pyautogui: MagicMock, controller: ScreenController) -> None:
    action = await controller.hotkey("ctrl", "c")
    assert action.success is True
    assert action.action_type == ScreenActionType.HOTKEY
    mock_pyautogui.hotkey.assert_called_once_with("ctrl", "c")


# ── Type text ────────────────────────────────────────────────────


async def test_type_text_blocks_api_key(controller: ScreenController) -> None:
    with pytest.raises(ScreenSafetyError, match="secret/API key"):
        await controller.type_text("sk-abcdefghijklmnopqrstuvwxyz1234567890")


@patch("agentos.screen.controller.pyautogui")
async def test_type_text_allowed(mock_pyautogui: MagicMock, controller: ScreenController) -> None:
    action = await controller.type_text("hello")
    assert action.success is True
    assert action.action_type == ScreenActionType.TYPE
    mock_pyautogui.typewrite.assert_called_once_with("hello", interval=0.0)


# ── Kill switch ──────────────────────────────────────────────────


async def test_kill_switch_blocks_action(controller: ScreenController) -> None:
    controller._killed = True
    with pytest.raises(KillSwitchError, match="Kill switch activated"):
        await controller.click(100, 100)


async def test_kill_switch_reset(controller: ScreenController) -> None:
    controller._killed = True
    assert controller.is_killed is True
    controller.reset_kill_switch()
    assert controller.is_killed is False


# ── Action limit ─────────────────────────────────────────────────


@patch("agentos.screen.controller.pyautogui")
async def test_action_limit(
    mock_pyautogui: MagicMock,
) -> None:
    safety = ScreenSafety(screen_width=1920, screen_height=1080, max_actions=5)
    ctrl = ScreenController(safety=safety, move_duration=0.0, type_interval=0.0, action_delay=0.0)
    for _ in range(5):
        await ctrl.click(100, 100)
    with pytest.raises(ScreenSafetyError, match="Exceeded max"):
        await ctrl.click(100, 100)


# ── Action log ───────────────────────────────────────────────────


@patch("agentos.screen.controller.pyautogui")
async def test_action_log(mock_pyautogui: MagicMock, controller: ScreenController) -> None:
    await controller.click(100, 100)
    await controller.type_text("hi")
    await controller.press_key("enter")
    log = controller.get_action_log()
    assert len(log) == 3
    assert log[0].action_type == ScreenActionType.CLICK
    assert log[1].action_type == ScreenActionType.TYPE
    assert log[2].action_type == ScreenActionType.PRESS_KEY


# ── Wait ─────────────────────────────────────────────────────────


async def test_wait() -> None:
    safety = ScreenSafety()
    ctrl = ScreenController(safety=safety, move_duration=0.0, type_interval=0.0, action_delay=0.0)
    start = time.monotonic()
    action = await ctrl.wait(0.1)
    elapsed = time.monotonic() - start
    assert action.success is True
    assert action.action_type == ScreenActionType.WAIT
    assert elapsed >= 0.08  # allow small timing tolerance


# ── Double click / right click / drag / scroll / move ────────────


@patch("agentos.screen.controller.pyautogui")
async def test_double_click(mock_pyautogui: MagicMock, controller: ScreenController) -> None:
    action = await controller.double_click(200, 300)
    assert action.success is True
    mock_pyautogui.doubleClick.assert_called_once_with(200, 300)


@patch("agentos.screen.controller.pyautogui")
async def test_right_click(mock_pyautogui: MagicMock, controller: ScreenController) -> None:
    action = await controller.right_click(200, 300)
    assert action.success is True
    mock_pyautogui.rightClick.assert_called_once_with(200, 300)


@patch("agentos.screen.controller.pyautogui")
async def test_scroll(mock_pyautogui: MagicMock, controller: ScreenController) -> None:
    action = await controller.scroll(5, x=100, y=100)
    assert action.success is True
    mock_pyautogui.scroll.assert_called_once_with(5, x=100, y=100)


@patch("agentos.screen.controller.pyautogui")
async def test_move(mock_pyautogui: MagicMock, controller: ScreenController) -> None:
    action = await controller.move(400, 500)
    assert action.success is True
    mock_pyautogui.moveTo.assert_called_once_with(400, 500, duration=0.0)
