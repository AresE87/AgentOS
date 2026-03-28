"""Tests for agentos.screen.safety."""

from __future__ import annotations

import pytest

from agentos.screen.safety import ScreenSafety, ScreenSafetyError


class TestValidateClick:
    def test_validate_click_in_bounds(self) -> None:
        safety = ScreenSafety(screen_width=1920, screen_height=1080)
        safety.validate_click(500, 500)  # should not raise

    def test_validate_click_out_of_bounds_negative_x(self) -> None:
        safety = ScreenSafety(screen_width=1920, screen_height=1080)
        with pytest.raises(ScreenSafetyError, match="outside screen bounds"):
            safety.validate_click(-1, 0)

    def test_validate_click_out_of_bounds_large_x(self) -> None:
        safety = ScreenSafety(screen_width=1920, screen_height=1080)
        with pytest.raises(ScreenSafetyError, match="outside screen bounds"):
            safety.validate_click(99999, 500)

    def test_validate_click_out_of_bounds_negative_y(self) -> None:
        safety = ScreenSafety(screen_width=1920, screen_height=1080)
        with pytest.raises(ScreenSafetyError, match="outside screen bounds"):
            safety.validate_click(500, -1)


class TestValidateHotkey:
    def test_validate_hotkey_blocked_alt_f4(self) -> None:
        safety = ScreenSafety()
        with pytest.raises(ScreenSafetyError, match="Blocked hotkey"):
            safety.validate_hotkey(("alt", "f4"))

    def test_validate_hotkey_blocked_ctrl_alt_delete(self) -> None:
        safety = ScreenSafety()
        with pytest.raises(ScreenSafetyError, match="Blocked hotkey"):
            safety.validate_hotkey(("ctrl", "alt", "delete"))

    def test_validate_hotkey_blocked_super_l(self) -> None:
        safety = ScreenSafety()
        with pytest.raises(ScreenSafetyError, match="Blocked hotkey"):
            safety.validate_hotkey(("super", "l"))

    def test_validate_hotkey_allowed_ctrl_c(self) -> None:
        safety = ScreenSafety()
        safety.validate_hotkey(("ctrl", "c"))  # should not raise

    def test_validate_hotkey_allowed_ctrl_v(self) -> None:
        safety = ScreenSafety()
        safety.validate_hotkey(("ctrl", "v"))  # should not raise


class TestValidateTypeText:
    def test_validate_type_api_key_openai(self) -> None:
        safety = ScreenSafety()
        with pytest.raises(ScreenSafetyError, match="secret/API key"):
            safety.validate_type_text("here is sk-abcdefghijklmnopqrstuvwxyz1234567890")

    def test_validate_type_api_key_google(self) -> None:
        safety = ScreenSafety()
        with pytest.raises(ScreenSafetyError, match="secret/API key"):
            safety.validate_type_text("AIzaSyB1234567890abcdefghijklmnopqrstuv")

    def test_validate_type_api_key_github(self) -> None:
        safety = ScreenSafety()
        with pytest.raises(ScreenSafetyError, match="secret/API key"):
            safety.validate_type_text("ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij")

    def test_validate_type_api_key_slack(self) -> None:
        safety = ScreenSafety()
        with pytest.raises(ScreenSafetyError, match="secret/API key"):
            safety.validate_type_text("xoxb-123456789-abcdefghij")

    def test_validate_type_normal_text(self) -> None:
        safety = ScreenSafety()
        safety.validate_type_text("hello world")  # should not raise


class TestActionLimit:
    def test_action_limit_exceeded(self) -> None:
        safety = ScreenSafety(max_actions=5)
        for _ in range(5):
            safety.validate_click(100, 100)
        with pytest.raises(ScreenSafetyError, match="Exceeded max"):
            safety.validate_click(100, 100)

    def test_reset_count(self) -> None:
        safety = ScreenSafety(max_actions=5)
        for _ in range(5):
            safety.validate_click(100, 100)
        safety.reset_count()
        # Should work again after reset
        safety.validate_click(100, 100)
