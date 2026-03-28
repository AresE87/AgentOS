"""Screen capture service using mss for fast cross-platform capture."""

from __future__ import annotations

import asyncio
import base64
import hashlib
import io
from datetime import UTC, datetime

import mss
from PIL import Image

from agentos.types import Screenshot
from agentos.utils.logging import get_logger

logger = get_logger("screen.capture")


class ScreenCapture:
    """Screen capture service using mss.

    Uses mss for fast cross-platform screen grabbing and Pillow for image
    processing.  All async methods delegate to threads via asyncio.to_thread
    because mss is not natively async.
    """

    def __init__(self, max_width: int = 1280, quality: int = 85) -> None:
        self._max_width = max_width
        self._quality = quality

    # ── Public async API ─────────────────────────────────────────────

    async def capture_full(self) -> Screenshot:
        """Capture the full screen (all monitors combined)."""
        return await asyncio.to_thread(self._capture_full_sync)

    async def capture_region(self, x: int, y: int, width: int, height: int) -> Screenshot:
        """Capture a rectangular region of the screen."""
        return await asyncio.to_thread(self._capture_region_sync, x, y, width, height)

    def resize_for_llm(self, screenshot: Screenshot, max_dimension: int = 1024) -> Screenshot:
        """Resize a screenshot for LLM consumption, preserving aspect ratio."""
        img = Image.open(io.BytesIO(screenshot.image_bytes))
        w, h = img.size
        if max(w, h) <= max_dimension:
            return screenshot

        scale = max_dimension / max(w, h)
        new_w, new_h = int(w * scale), int(h * scale)
        img = img.resize((new_w, new_h), Image.LANCZOS)

        buf = io.BytesIO()
        img.save(buf, format="PNG")
        image_bytes = buf.getvalue()

        return Screenshot(
            image_bytes=image_bytes,
            width=new_w,
            height=new_h,
            timestamp=screenshot.timestamp,
            region=screenshot.region,
            hash=hashlib.sha256(image_bytes).hexdigest()[:16],
        )

    def to_base64(
        self,
        screenshot: Screenshot,
        format: str = "png",  # noqa: A002
        jpeg_quality: int = 85,
    ) -> str:
        """Encode a screenshot as a base64 string."""
        if format == "jpeg":
            img = Image.open(io.BytesIO(screenshot.image_bytes))
            buf = io.BytesIO()
            img.save(buf, format="JPEG", quality=jpeg_quality)
            return base64.b64encode(buf.getvalue()).decode()
        return base64.b64encode(screenshot.image_bytes).decode()

    # ── Sync internals ───────────────────────────────────────────────

    def _capture_full_sync(self) -> Screenshot:
        try:
            with mss.mss() as sct:
                monitor = sct.monitors[0]  # All monitors combined
                raw = sct.grab(monitor)
                img = Image.frombytes("RGB", raw.size, raw.bgra, "raw", "BGRX")
                return self._process_image(img, region=None)
        except Exception:
            logger.warning("mss capture failed (no display?); returning fallback image")
            return self._fallback_image(region=None)

    def _capture_region_sync(self, x: int, y: int, width: int, height: int) -> Screenshot:
        try:
            with mss.mss() as sct:
                region = {"left": x, "top": y, "width": width, "height": height}
                raw = sct.grab(region)
                img = Image.frombytes("RGB", raw.size, raw.bgra, "raw", "BGRX")
                return self._process_image(img, region=(x, y, width, height))
        except Exception:
            logger.warning("mss region capture failed (no display?); returning fallback image")
            return self._fallback_image(region=(x, y, width, height))

    def _process_image(
        self,
        img: Image.Image,
        region: tuple[int, int, int, int] | None,
    ) -> Screenshot:
        """Resize if wider than max_width, then encode as PNG."""
        w, h = img.size
        if w > self._max_width:
            scale = self._max_width / w
            img = img.resize((self._max_width, int(h * scale)), Image.LANCZOS)

        buf = io.BytesIO()
        img.save(buf, format="PNG")
        image_bytes = buf.getvalue()

        return Screenshot(
            image_bytes=image_bytes,
            width=img.size[0],
            height=img.size[1],
            timestamp=datetime.now(UTC),
            region=region,
            hash=hashlib.sha256(image_bytes).hexdigest()[:16],
        )

    def _fallback_image(
        self,
        region: tuple[int, int, int, int] | None,
    ) -> Screenshot:
        """Generate a synthetic 100x100 gray image for CI / headless envs."""
        img = Image.new("RGB", (100, 100), color=(128, 128, 128))
        buf = io.BytesIO()
        img.save(buf, format="PNG")
        image_bytes = buf.getvalue()

        return Screenshot(
            image_bytes=image_bytes,
            width=100,
            height=100,
            timestamp=datetime.now(UTC),
            region=region,
            hash=hashlib.sha256(image_bytes).hexdigest()[:16],
        )
