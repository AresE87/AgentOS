"""Tests for VisionAnalyzer with a mocked LLM gateway."""

from __future__ import annotations

import hashlib
import io
import json
from datetime import UTC, datetime
from unittest.mock import AsyncMock

import pytest
from PIL import Image

from agentos.screen.analyzer import VisionAnalyzer
from agentos.screen.capture import ScreenCapture
from agentos.types import LLMResponse, Screenshot

# ── Helpers ───────────────────────────────────────────────────────────


def _make_screenshot(width: int = 1920, height: int = 1080) -> Screenshot:
    """Create a small synthetic screenshot with valid PNG bytes."""
    img = Image.new("RGB", (width, height), color=(50, 100, 150))
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    raw = buf.getvalue()
    return Screenshot(
        image_bytes=raw,
        width=width,
        height=height,
        timestamp=datetime.now(UTC),
        region=None,
        hash=hashlib.sha256(raw).hexdigest()[:16],
    )


def _make_response(content: str) -> LLMResponse:
    """Build a fake LLMResponse with the given content."""
    return LLMResponse(
        content=content,
        model="gpt-4o",
        provider="openai",
        tokens_in=100,
        tokens_out=200,
        cost_estimate=0.005,
        latency_ms=500.0,
    )


def _mock_gateway(content: str) -> AsyncMock:
    """Return a gateway mock whose complete() returns the given content."""
    gw = AsyncMock()
    gw.complete.return_value = _make_response(content)
    return gw


# ── Tests ─────────────────────────────────────────────────────────────


async def test_describe_parses_json():
    """Gateway returns valid JSON -> ScreenAnalysis with parsed elements."""
    payload = json.dumps(
        {
            "app_name": "Notepad",
            "description": "A text editor window",
            "elements": [
                {
                    "type": "button",
                    "label": "Save",
                    "x_pct": 10,
                    "y_pct": 5,
                    "w_pct": 8,
                    "h_pct": 3,
                    "confidence": 0.95,
                },
                {
                    "type": "input",
                    "label": "Search",
                    "x_pct": 50,
                    "y_pct": 10,
                    "w_pct": 30,
                    "h_pct": 4,
                    "confidence": 0.88,
                },
            ],
            "visible_text": "File Edit Save",
        }
    )
    gw = _mock_gateway(payload)
    analyzer = VisionAnalyzer(gateway=gw, capture=ScreenCapture())
    ss = _make_screenshot()

    result = await analyzer.describe(ss)

    assert result.app_name == "Notepad"
    assert result.description == "A text editor window"
    assert len(result.elements) == 2
    assert result.elements[0].element_type == "button"
    assert result.elements[0].label == "Save"
    assert result.visible_text == "File Edit Save"
    assert result.model_used == "gpt-4o"
    assert result.tokens_used == 300  # 100 + 200
    assert result.screenshot_hash == ss.hash
    gw.complete.assert_awaited_once()


async def test_describe_handles_invalid_json():
    """Gateway returns non-JSON text -> falls back to raw text description."""
    gw = _mock_gateway("This is not valid JSON at all")
    analyzer = VisionAnalyzer(gateway=gw, capture=ScreenCapture())
    ss = _make_screenshot()

    result = await analyzer.describe(ss)

    assert result.description == "This is not valid JSON at all"
    assert result.elements == []
    assert result.visible_text == "This is not valid JSON at all"


async def test_locate_found():
    """Gateway returns found=true -> UIElement with pixel coordinates."""
    payload = json.dumps(
        {
            "found": True,
            "type": "button",
            "label": "Submit",
            "x_pct": 50,
            "y_pct": 50,
            "w_pct": 10,
            "h_pct": 5,
            "confidence": 0.92,
        }
    )
    gw = _mock_gateway(payload)
    analyzer = VisionAnalyzer(gateway=gw, capture=ScreenCapture())
    ss = _make_screenshot(width=1920, height=1080)

    elem = await analyzer.locate(ss, "Submit button")

    assert elem is not None
    assert elem.element_type == "button"
    assert elem.label == "Submit"
    assert elem.x == 960  # 50% of 1920
    assert elem.y == 540  # 50% of 1080
    assert elem.confidence == pytest.approx(0.92)


async def test_locate_not_found():
    """Gateway returns found=false -> None."""
    payload = json.dumps({"found": False})
    gw = _mock_gateway(payload)
    analyzer = VisionAnalyzer(gateway=gw, capture=ScreenCapture())
    ss = _make_screenshot()

    elem = await analyzer.locate(ss, "Nonexistent element")

    assert elem is None


async def test_read_text():
    """Gateway returns text -> stripped string."""
    gw = _mock_gateway("  Hello World from the screenshot  \n")
    analyzer = VisionAnalyzer(gateway=gw, capture=ScreenCapture())
    ss = _make_screenshot()

    text = await analyzer.read_text(ss)

    assert text == "Hello World from the screenshot"
    gw.complete.assert_awaited_once()


async def test_compare():
    """Gateway returns change description -> string."""
    gw = _mock_gateway("A dialog box appeared in the center of the screen.")
    analyzer = VisionAnalyzer(gateway=gw, capture=ScreenCapture())
    before = _make_screenshot()
    after = _make_screenshot()

    result = await analyzer.compare(before, after)

    assert result == "A dialog box appeared in the center of the screen."
    gw.complete.assert_awaited_once()


async def test_cache_hit():
    """Same screenshot described twice -> gateway called only once."""
    payload = json.dumps(
        {
            "app_name": "Browser",
            "description": "A web browser",
            "elements": [],
            "visible_text": "Google",
        }
    )
    gw = _mock_gateway(payload)
    analyzer = VisionAnalyzer(gateway=gw, capture=ScreenCapture())
    ss = _make_screenshot()

    result1 = await analyzer.describe(ss)
    result2 = await analyzer.describe(ss)

    assert result1.description == result2.description
    assert gw.complete.await_count == 1


async def test_pct_to_px_conversion():
    """50% coordinates on a 1920x1080 screenshot -> (960, 540) pixels."""
    payload = json.dumps(
        {
            "app_name": None,
            "description": "Desktop",
            "elements": [
                {
                    "type": "button",
                    "label": "Center",
                    "x_pct": 50,
                    "y_pct": 50,
                    "w_pct": 10,
                    "h_pct": 5,
                    "confidence": 1.0,
                }
            ],
            "visible_text": "",
        }
    )
    gw = _mock_gateway(payload)
    analyzer = VisionAnalyzer(gateway=gw, capture=ScreenCapture())
    ss = _make_screenshot(width=1920, height=1080)

    result = await analyzer.describe(ss)

    elem = result.elements[0]
    assert elem.x == 960  # 50% of 1920
    assert elem.y == 540  # 50% of 1080
    assert elem.width == 192  # 10% of 1920
    assert elem.height == 54  # 5% of 1080
