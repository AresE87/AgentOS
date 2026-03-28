"""Tests for ScreenCapture — uses synthetic PIL images, no real display needed."""

from __future__ import annotations

import base64
import io
from datetime import UTC, datetime
from unittest.mock import patch

import pytest
from PIL import Image

from agentos.screen.capture import ScreenCapture
from agentos.types import Screenshot

# ── Helpers ──────────────────────────────────────────────────────────


def _make_screenshot(width: int = 200, height: int = 150, color: tuple = (255, 0, 0)) -> Screenshot:
    """Build a Screenshot from a synthetic solid-color PIL image."""
    img = Image.new("RGB", (width, height), color=color)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    import hashlib

    image_bytes = buf.getvalue()
    return Screenshot(
        image_bytes=image_bytes,
        width=width,
        height=height,
        timestamp=datetime.now(UTC),
        region=None,
        hash=hashlib.sha256(image_bytes).hexdigest()[:16],
    )


@pytest.fixture
def capture() -> ScreenCapture:
    return ScreenCapture(max_width=1280, quality=85)


# ── Tests ────────────────────────────────────────────────────────────


def test_process_image(capture: ScreenCapture) -> None:
    """_process_image should return a Screenshot with correct dimensions."""
    img = Image.new("RGB", (640, 480), color=(0, 128, 255))
    result = capture._process_image(img, region=None)

    assert isinstance(result, Screenshot)
    assert result.width == 640
    assert result.height == 480
    assert result.region is None
    assert len(result.hash) == 16
    assert len(result.image_bytes) > 0


def test_process_image_respects_max_width(capture: ScreenCapture) -> None:
    """Images wider than max_width should be downscaled."""
    cap = ScreenCapture(max_width=800)
    img = Image.new("RGB", (1600, 1000), color=(0, 0, 0))
    result = cap._process_image(img, region=(0, 0, 1600, 1000))

    assert result.width == 800
    assert result.height == 500
    assert result.region == (0, 0, 1600, 1000)


def test_resize_for_llm_downscale(capture: ScreenCapture) -> None:
    """A 2000x1000 image should downscale to 1024x512."""
    ss = _make_screenshot(2000, 1000)
    resized = capture.resize_for_llm(ss, max_dimension=1024)

    assert resized.width == 1024
    assert resized.height == 512
    assert len(resized.hash) == 16


def test_resize_for_llm_no_upscale(capture: ScreenCapture) -> None:
    """An 800x600 image should stay 800x600 (no upscaling)."""
    ss = _make_screenshot(800, 600)
    resized = capture.resize_for_llm(ss, max_dimension=1024)

    assert resized.width == 800
    assert resized.height == 600
    # Should be the exact same object (short-circuit)
    assert resized is ss


def test_to_base64_png(capture: ScreenCapture) -> None:
    """to_base64 with PNG format should produce valid base64."""
    ss = _make_screenshot(100, 100)
    b64 = capture.to_base64(ss, format="png")

    decoded = base64.b64decode(b64)
    assert decoded == ss.image_bytes


def test_to_base64_jpeg(capture: ScreenCapture) -> None:
    """to_base64 with JPEG format should produce a valid, decodable JPEG base64 string."""
    ss = _make_screenshot(100, 100, color=(50, 100, 150))
    b64_jpg = capture.to_base64(ss, format="jpeg", jpeg_quality=50)

    # Should decode to valid JPEG bytes
    decoded_jpg = base64.b64decode(b64_jpg)
    assert len(decoded_jpg) > 0

    # Verify the result is a valid JPEG image
    img = Image.open(io.BytesIO(decoded_jpg))
    assert img.format == "JPEG"
    assert img.size == (100, 100)


def test_hash_changes(capture: ScreenCapture) -> None:
    """Different images must produce different hashes."""
    ss_red = _make_screenshot(100, 100, color=(255, 0, 0))
    ss_blue = _make_screenshot(100, 100, color=(0, 0, 255))

    assert ss_red.hash != ss_blue.hash


def test_hash_same(capture: ScreenCapture) -> None:
    """Identical images must produce the same hash."""
    ss1 = _make_screenshot(100, 100, color=(42, 42, 42))
    ss2 = _make_screenshot(100, 100, color=(42, 42, 42))

    assert ss1.hash == ss2.hash


@pytest.mark.asyncio
async def test_capture_full_no_display(capture: ScreenCapture) -> None:
    """When mss raises (no display), capture_full returns a fallback 100x100 gray image."""
    with patch("agentos.screen.capture.mss.mss", side_effect=Exception("No display")):
        result = await capture.capture_full()

    assert isinstance(result, Screenshot)
    assert result.width == 100
    assert result.height == 100
    assert result.region is None

    # Verify it is a valid PNG we can open
    img = Image.open(io.BytesIO(result.image_bytes))
    assert img.size == (100, 100)


@pytest.mark.asyncio
async def test_capture_region_no_display(capture: ScreenCapture) -> None:
    """When mss raises (no display), capture_region returns a fallback image."""
    with patch("agentos.screen.capture.mss.mss", side_effect=Exception("No display")):
        result = await capture.capture_region(10, 20, 300, 200)

    assert isinstance(result, Screenshot)
    assert result.width == 100
    assert result.height == 100
    # Region metadata is preserved even in fallback
    assert result.region == (10, 20, 300, 200)
