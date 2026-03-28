"""Vision analyzer — sends screenshots to the LLM Gateway for structured analysis.

Uses multimodal prompts (base64-encoded images) to describe screens, locate
UI elements, extract text (OCR), and compare before/after screenshots.
Results are cached with an LRU eviction policy keyed by screenshot hash.
"""

from __future__ import annotations

import json
from collections import OrderedDict

from agentos.screen.capture import ScreenCapture
from agentos.types import (
    LLMRequest,
    LLMTier,
    ScreenAnalysis,
    Screenshot,
    TaskType,
    UIElement,
)
from agentos.utils.logging import get_logger

logger = get_logger("screen.analyzer")

DESCRIBE_PROMPT = """Analyze this screenshot. Identify all visible UI elements and text.
Respond in JSON:
{
    "app_name": "string or null",
    "description": "brief description",
    "elements": [
        {"type": "button|input|link|menu|text|dropdown", "label": "visible text", \
"x_pct": 50, "y_pct": 50, "w_pct": 10, "h_pct": 5, "confidence": 0.9}
    ],
    "visible_text": "all text visible on screen"
}
Coordinates are percentages of screen dimensions (0-100)."""

LOCATE_PROMPT = """Find the UI element: "{target}"
Respond in JSON:
{{"found": true/false, "type": "button|input|link|...", "label": "text", \
"x_pct": 50, "y_pct": 50, "w_pct": 10, "h_pct": 5, "confidence": 0.9}}"""


class VisionAnalyzer:
    """Analyze screenshots via multimodal LLM calls through the gateway.

    Args:
        gateway: An LLMGateway instance used to send completion requests.
        capture: Optional ScreenCapture instance for resizing / encoding.
        cache_size: Maximum number of analysis results to cache.
    """

    def __init__(
        self,
        gateway: object,
        capture: ScreenCapture | None = None,
        cache_size: int = 50,
    ) -> None:
        self._gateway = gateway
        self._capture = capture or ScreenCapture()
        self._cache: OrderedDict[str, ScreenAnalysis] = OrderedDict()
        self._cache_size = cache_size

    # ── Public API ────────────────────────────────────────────────────

    async def describe(self, screenshot: Screenshot) -> ScreenAnalysis:
        """Full screen analysis with element detection.

        Returns a cached result when the same screenshot hash is seen again.
        """
        cache_key = f"describe:{screenshot.hash}"
        if cache_key in self._cache:
            self._cache.move_to_end(cache_key)
            return self._cache[cache_key]

        resized = self._capture.resize_for_llm(screenshot)
        b64 = self._capture.to_base64(resized, format="jpeg", jpeg_quality=85)

        prompt = f"[Image: data:image/jpeg;base64,{b64}]\n\n{DESCRIBE_PROMPT}"

        request = LLMRequest(
            prompt=prompt,
            tier=LLMTier.STANDARD,
            task_type=TaskType.VISION,
            system_prompt="You are a UI analysis assistant. Respond only with valid JSON.",
        )
        response = await self._gateway.complete(request)

        analysis = self._parse_describe_response(response.content, screenshot)
        analysis = ScreenAnalysis(
            description=analysis.description,
            elements=analysis.elements,
            visible_text=analysis.visible_text,
            app_name=analysis.app_name,
            screenshot_hash=screenshot.hash,
            model_used=response.model,
            tokens_used=response.tokens_in + response.tokens_out,
            cost=response.cost_estimate,
        )

        self._cache[cache_key] = analysis
        if len(self._cache) > self._cache_size:
            self._cache.popitem(last=False)

        return analysis

    async def locate(self, screenshot: Screenshot, target: str) -> UIElement | None:
        """Find a specific UI element by description.

        Args:
            screenshot: The screenshot to search in.
            target: Natural-language description of the element to find.

        Returns:
            UIElement with pixel coordinates, or None if not found.
        """
        resized = self._capture.resize_for_llm(screenshot)
        b64 = self._capture.to_base64(resized, format="jpeg")

        prompt = f"[Image: data:image/jpeg;base64,{b64}]\n\n{LOCATE_PROMPT.format(target=target)}"

        request = LLMRequest(
            prompt=prompt,
            tier=LLMTier.STANDARD,
            task_type=TaskType.VISION,
            system_prompt="You are a UI element locator. Respond only with valid JSON.",
        )
        response = await self._gateway.complete(request)
        return self._parse_locate_response(response.content, screenshot)

    async def read_text(self, screenshot: Screenshot) -> str:
        """Extract all visible text from a screenshot (OCR via vision model).

        Args:
            screenshot: The screenshot to read.

        Returns:
            Extracted text as a plain string.
        """
        resized = self._capture.resize_for_llm(screenshot)
        b64 = self._capture.to_base64(resized)

        prompt = (
            f"[Image: data:image/png;base64,{b64}]\n\n"
            "Extract ALL visible text from this screenshot. Return only the text, "
            "no formatting."
        )
        request = LLMRequest(
            prompt=prompt,
            tier=LLMTier.CHEAP,
            task_type=TaskType.VISION,
        )
        response = await self._gateway.complete(request)
        return response.content.strip()

    async def compare(self, before: Screenshot, after: Screenshot) -> str:
        """Describe what changed between two screenshots.

        Args:
            before: The earlier screenshot.
            after: The later screenshot.

        Returns:
            A brief natural-language description of the differences.
        """
        b64_before = self._capture.to_base64(self._capture.resize_for_llm(before), format="jpeg")
        b64_after = self._capture.to_base64(self._capture.resize_for_llm(after), format="jpeg")
        prompt = (
            f"[Image 1: data:image/jpeg;base64,{b64_before}]\n"
            f"[Image 2: data:image/jpeg;base64,{b64_after}]\n\n"
            "Compare these screenshots. What changed? Be brief (1-2 sentences)."
        )
        request = LLMRequest(
            prompt=prompt,
            tier=LLMTier.CHEAP,
            task_type=TaskType.VISION,
        )
        response = await self._gateway.complete(request)
        return response.content.strip()

    # ── Response parsing ──────────────────────────────────────────────

    def _parse_describe_response(self, content: str, screenshot: Screenshot) -> ScreenAnalysis:
        """Parse JSON response from the describe prompt."""
        try:
            text = content.strip()
            if text.startswith("```"):
                text = text.split("```")[1]
                if text.startswith("json"):
                    text = text[4:]
            data = json.loads(text)
        except (json.JSONDecodeError, IndexError):
            logger.warning("Failed to parse describe response as JSON; using raw text")
            return ScreenAnalysis(
                description=content,
                elements=[],
                visible_text=content,
                screenshot_hash=screenshot.hash,
            )

        elements: list[UIElement] = []
        for elem in data.get("elements", []):
            x = int(elem.get("x_pct", 0) * screenshot.width / 100)
            y = int(elem.get("y_pct", 0) * screenshot.height / 100)
            w = int(elem.get("w_pct", 0) * screenshot.width / 100)
            h = int(elem.get("h_pct", 0) * screenshot.height / 100)
            elements.append(
                UIElement(
                    element_type=elem.get("type", "unknown"),
                    label=elem.get("label", ""),
                    location=f"({elem.get('x_pct', 0)}%, {elem.get('y_pct', 0)}%)",
                    x=x,
                    y=y,
                    width=w,
                    height=h,
                    confidence=elem.get("confidence", 0.0),
                )
            )

        return ScreenAnalysis(
            description=data.get("description", ""),
            elements=elements,
            visible_text=data.get("visible_text", ""),
            app_name=data.get("app_name"),
            screenshot_hash=screenshot.hash,
        )

    def _parse_locate_response(self, content: str, screenshot: Screenshot) -> UIElement | None:
        """Parse JSON response from the locate prompt."""
        try:
            text = content.strip()
            if text.startswith("```"):
                text = text.split("```")[1]
                if text.startswith("json"):
                    text = text[4:]
            data = json.loads(text)
        except (json.JSONDecodeError, IndexError):
            logger.warning("Failed to parse locate response as JSON")
            return None

        if not data.get("found", False):
            return None

        x = int(data.get("x_pct", 0) * screenshot.width / 100)
        y = int(data.get("y_pct", 0) * screenshot.height / 100)
        w = int(data.get("w_pct", 0) * screenshot.width / 100)
        h = int(data.get("h_pct", 0) * screenshot.height / 100)

        return UIElement(
            element_type=data.get("type", "unknown"),
            label=data.get("label", ""),
            location=f"({data.get('x_pct', 0)}%, {data.get('y_pct', 0)}%)",
            x=x,
            y=y,
            width=w,
            height=h,
            confidence=data.get("confidence", 0.0),
        )
