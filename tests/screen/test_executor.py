"""Tests for agentos.screen.executor.

All external dependencies (gateway, capture, analyzer, controller) are mocked.
"""

from __future__ import annotations

import json
from datetime import UTC, datetime
from unittest.mock import AsyncMock

import pytest

from agentos.screen.controller import KillSwitchError
from agentos.screen.executor import ScreenExecutor
from agentos.screen.safety import ScreenSafetyError
from agentos.types import (
    ExecutorType,
    LLMResponse,
    ScreenAction,
    ScreenActionType,
    ScreenAnalysis,
    Screenshot,
    UIElement,
)

# ── Helpers ───────────────────────────────────────────────────────


def _make_screenshot(hash_val: str = "abc123") -> Screenshot:
    return Screenshot(
        image_bytes=b"\x89PNG",
        width=1920,
        height=1080,
        timestamp=datetime.now(UTC),
        hash=hash_val,
    )


def _make_analysis(description: str = "Desktop with icons") -> ScreenAnalysis:
    return ScreenAnalysis(
        description=description,
        elements=[],
        visible_text="",
        screenshot_hash="abc123",
    )


def _make_llm_response(content: str) -> LLMResponse:
    return LLMResponse(
        content=content,
        model="test-model",
        provider="test",
        tokens_in=10,
        tokens_out=20,
        cost_estimate=0.001,
        latency_ms=100,
    )


def _make_action(success: bool = True) -> ScreenAction:
    return ScreenAction(
        action_type=ScreenActionType.CLICK,
        params={"x": 100, "y": 200},
        timestamp=datetime.now(UTC),
        success=success,
        duration_ms=50.0,
    )


# ── Fixtures ──────────────────────────────────────────────────────


@pytest.fixture()
def gateway() -> AsyncMock:
    return AsyncMock()


@pytest.fixture()
def capture() -> AsyncMock:
    mock = AsyncMock()
    mock.capture_full.return_value = _make_screenshot()
    return mock


@pytest.fixture()
def analyzer() -> AsyncMock:
    mock = AsyncMock()
    mock.describe.return_value = _make_analysis()
    mock.locate.return_value = UIElement(
        element_type="button",
        label="OK",
        location="center",
        x=100,
        y=200,
    )
    return mock


@pytest.fixture()
def controller() -> AsyncMock:
    mock = AsyncMock()
    mock.click.return_value = _make_action()
    mock.type_text.return_value = _make_action()
    mock.hotkey.return_value = _make_action()
    mock.scroll.return_value = _make_action()
    mock.wait.return_value = _make_action()
    return mock


@pytest.fixture()
def executor(
    gateway: AsyncMock,
    capture: AsyncMock,
    analyzer: AsyncMock,
    controller: AsyncMock,
) -> ScreenExecutor:
    return ScreenExecutor(
        gateway=gateway,
        capture=capture,
        analyzer=analyzer,
        controller=controller,
        max_iterations=5,
        stuck_threshold=3,
    )


# ── Tests ─────────────────────────────────────────────────────────


async def test_execute_done(
    executor: ScreenExecutor,
    gateway: AsyncMock,
) -> None:
    """LLM says done after 1 iteration -> exit_code=0."""
    gateway.complete.return_value = _make_llm_response(
        json.dumps({"action_type": "done", "result": "Opened the file"})
    )

    result = await executor.execute("Open the file")

    assert result.exit_code == 0
    assert result.executor_type == ExecutorType.SCREEN
    assert "Opened the file" in result.stdout
    assert result.stderr == ""


async def test_execute_click_action(
    executor: ScreenExecutor,
    gateway: AsyncMock,
    analyzer: AsyncMock,
    controller: AsyncMock,
) -> None:
    """LLM says click -> locate finds element -> controller.click called."""
    gateway.complete.side_effect = [
        _make_llm_response(
            json.dumps(
                {
                    "action_type": "click",
                    "target": "OK button",
                    "params": {"element": "OK"},
                }
            )
        ),
        _make_llm_response(json.dumps({"action_type": "done", "result": "Clicked OK"})),
    ]

    result = await executor.execute("Click OK")

    assert result.exit_code == 0
    analyzer.locate.assert_called_once()
    controller.click.assert_called_once_with(100, 200)


async def test_execute_type_action(
    executor: ScreenExecutor,
    gateway: AsyncMock,
    controller: AsyncMock,
) -> None:
    """LLM says type -> controller.type_text called."""
    gateway.complete.side_effect = [
        _make_llm_response(
            json.dumps(
                {
                    "action_type": "type",
                    "target": "search box",
                    "params": {"text": "hello world"},
                }
            )
        ),
        _make_llm_response(json.dumps({"action_type": "done", "result": "Typed text"})),
    ]

    result = await executor.execute("Type hello world")

    assert result.exit_code == 0
    controller.type_text.assert_called_once_with("hello world")


async def test_execute_max_iterations(
    executor: ScreenExecutor,
    gateway: AsyncMock,
    capture: AsyncMock,
) -> None:
    """LLM never says done -> fails after max_iterations."""
    # Return different hashes so stuck detection doesn't trigger
    call_count = 0

    async def varying_screenshot() -> Screenshot:
        nonlocal call_count
        call_count += 1
        return _make_screenshot(hash_val=f"hash_{call_count}")

    capture.capture_full.side_effect = varying_screenshot

    gateway.complete.return_value = _make_llm_response(
        json.dumps(
            {
                "action_type": "click",
                "target": "button",
                "params": {"element": "button"},
            }
        )
    )

    result = await executor.execute("Do something")

    assert result.exit_code == 1
    assert "Max iterations" in result.stderr


async def test_execute_stuck_detection(
    executor: ScreenExecutor,
    gateway: AsyncMock,
    capture: AsyncMock,
) -> None:
    """Same screenshot hash 3 times in a row -> fails with 'stuck'."""
    capture.capture_full.return_value = _make_screenshot(hash_val="same_hash")

    # The first iteration captures + detects stuck (threshold=3 needs 3 same)
    # We need the gateway to return a non-terminal action so the loop continues
    gateway.complete.return_value = _make_llm_response(
        json.dumps(
            {
                "action_type": "click",
                "target": "button",
                "params": {"element": "button"},
            }
        )
    )

    result = await executor.execute("Do something")

    assert result.exit_code == 1
    assert "Stuck" in result.stderr


async def test_execute_kill_switch(
    executor: ScreenExecutor,
    gateway: AsyncMock,
    controller: AsyncMock,
) -> None:
    """Controller raises KillSwitchError -> exit_code=2."""
    gateway.complete.return_value = _make_llm_response(
        json.dumps(
            {
                "action_type": "click",
                "target": "button",
                "params": {"element": "button"},
            }
        )
    )
    controller.click.side_effect = KillSwitchError("killed")

    result = await executor.execute("Click something")

    assert result.exit_code == 2
    assert "kill switch" in result.stderr.lower()


async def test_execute_safety_error(
    executor: ScreenExecutor,
    gateway: AsyncMock,
    controller: AsyncMock,
) -> None:
    """Controller raises ScreenSafetyError -> exit_code=1."""
    gateway.complete.return_value = _make_llm_response(
        json.dumps(
            {
                "action_type": "type",
                "target": "input",
                "params": {"text": "sk-secret123456789012345"},
            }
        )
    )
    controller.type_text.side_effect = ScreenSafetyError(
        "type_text", "Text contains what appears to be a secret/API key"
    )

    result = await executor.execute("Type a secret")

    assert result.exit_code == 1
    assert "Safety blocked" in result.stderr


async def test_execute_error_action(
    executor: ScreenExecutor,
    gateway: AsyncMock,
) -> None:
    """LLM says error -> exit_code=1 with reason."""
    gateway.complete.return_value = _make_llm_response(
        json.dumps({"action_type": "error", "reason": "Cannot find the application"})
    )

    result = await executor.execute("Open missing app")

    assert result.exit_code == 1
    assert "Cannot find the application" in result.stderr


async def test_execute_element_not_found(
    executor: ScreenExecutor,
    gateway: AsyncMock,
    analyzer: AsyncMock,
    capture: AsyncMock,
) -> None:
    """locate returns None -> action skipped, loop continues."""
    call_count = 0

    async def varying_screenshot() -> Screenshot:
        nonlocal call_count
        call_count += 1
        return _make_screenshot(hash_val=f"hash_{call_count}")

    capture.capture_full.side_effect = varying_screenshot
    analyzer.locate.return_value = None

    gateway.complete.side_effect = [
        _make_llm_response(
            json.dumps(
                {
                    "action_type": "click",
                    "target": "ghost button",
                    "params": {"element": "ghost"},
                }
            )
        ),
        _make_llm_response(json.dumps({"action_type": "done", "result": "Gave up and done"})),
    ]

    result = await executor.execute("Click ghost")

    assert result.exit_code == 0
    assert "Gave up and done" in result.stdout
