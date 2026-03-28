"""Screen executor — perception-action loop combining vision analysis and control.

Sends screenshots to the LLM for decision-making, then executes the chosen
action via ScreenController.  Repeats until the LLM reports "done", an error
occurs, or the iteration / stuck-detection limits are hit.
"""

from __future__ import annotations

import json
import time
from typing import TYPE_CHECKING

from agentos.screen.controller import KillSwitchError
from agentos.screen.safety import ScreenSafetyError
from agentos.types import (
    ExecutionResult,
    ExecutorType,
    LLMRequest,
    LLMTier,
    Screenshot,
    TaskType,
)
from agentos.utils.logging import get_logger

if TYPE_CHECKING:
    from agentos.screen.analyzer import VisionAnalyzer
    from agentos.screen.capture import ScreenCapture
    from agentos.screen.controller import ScreenController

logger = get_logger("screen.executor")

SCREEN_ACTION_PROMPT = """You are controlling a computer screen. You see:
{analysis}

User instruction: {instruction}

Actions taken so far: {history}

Decide the NEXT action. Respond in JSON:
- To act: {{"action_type": "click|type|hotkey|scroll|wait", "target": "description", "params": {{...}}}}
  click params: {{"element": "text of what to click"}}
  type params: {{"text": "text to type"}}
  hotkey params: {{"keys": ["ctrl", "c"]}}
  scroll params: {{"amount": 3, "direction": "down"}}
  wait params: {{"seconds": 2}}
- If DONE: {{"action_type": "done", "result": "what was accomplished"}}
- If STUCK: {{"action_type": "error", "reason": "why"}}
Only ONE action. Be precise."""


class ScreenExecutor:
    """Perception-action loop that combines VisionAnalyzer + ScreenController.

    Each iteration captures a screenshot, asks the LLM what to do, then
    executes the chosen action.  The loop exits when the LLM says "done",
    reports an error, or a safety / iteration limit is reached.
    """

    def __init__(
        self,
        gateway: object,  # LLMGateway
        capture: ScreenCapture,
        analyzer: VisionAnalyzer,
        controller: ScreenController,
        max_iterations: int = 20,
        stuck_threshold: int = 3,
    ) -> None:
        self._gateway = gateway
        self._capture = capture
        self._analyzer = analyzer
        self._controller = controller
        self._max_iterations = max_iterations
        self._stuck_threshold = stuck_threshold

    # ── Public API ────────────────────────────────────────────────

    async def execute(self, instruction: str, task_id: str = "") -> ExecutionResult:
        """Execute a visual task via the screen control loop."""
        start = time.monotonic()
        history: list[dict] = []
        recent_hashes: list[str] = []

        try:
            for iteration in range(self._max_iterations):
                # 1. Capture screenshot
                screenshot = await self._capture.capture_full()

                # 2. Check if stuck (same hash N times in a row)
                recent_hashes.append(screenshot.hash)
                if len(recent_hashes) > self._stuck_threshold:
                    recent_hashes = recent_hashes[-self._stuck_threshold :]
                if len(recent_hashes) >= self._stuck_threshold and len(set(recent_hashes)) == 1:
                    return ExecutionResult(
                        command=f"screen: {instruction}",
                        exit_code=1,
                        stdout="",
                        stderr=(
                            f"Stuck: screen unchanged after {self._stuck_threshold} iterations"
                        ),
                        duration_ms=(time.monotonic() - start) * 1000,
                        executor_type=ExecutorType.SCREEN,
                    )

                # 3. Analyze screen
                analysis = await self._analyzer.describe(screenshot)

                # 4. Decide next action via LLM
                action_data = await self._decide_action(instruction, analysis, history)

                if action_data.get("action_type") == "done":
                    elapsed = (time.monotonic() - start) * 1000
                    summary = "\n".join(
                        f"Step {i + 1}: {h.get('action', 'unknown')}" for i, h in enumerate(history)
                    )
                    return ExecutionResult(
                        command=f"screen: {instruction}",
                        exit_code=0,
                        stdout=(
                            f"{action_data.get('result', 'Task completed')}\n\nSteps:\n{summary}"
                        ),
                        stderr="",
                        duration_ms=elapsed,
                        executor_type=ExecutorType.SCREEN,
                    )

                if action_data.get("action_type") == "error":
                    return ExecutionResult(
                        command=f"screen: {instruction}",
                        exit_code=1,
                        stdout="",
                        stderr=action_data.get("reason", "Unknown error"),
                        duration_ms=(time.monotonic() - start) * 1000,
                        executor_type=ExecutorType.SCREEN,
                    )

                # 5. Execute the action
                action_result = await self._execute_action(action_data, screenshot)
                history.append(
                    {
                        "action": action_data.get("action_type"),
                        "target": action_data.get("target", ""),
                        "success": (action_result.success if action_result is not None else False),
                        "iteration": iteration,
                    }
                )

            # Max iterations reached
            return ExecutionResult(
                command=f"screen: {instruction}",
                exit_code=1,
                stdout="",
                stderr=f"Max iterations ({self._max_iterations}) reached",
                duration_ms=(time.monotonic() - start) * 1000,
                executor_type=ExecutorType.SCREEN,
            )

        except KillSwitchError:
            return ExecutionResult(
                command=f"screen: {instruction}",
                exit_code=2,
                stdout="",
                stderr="Killed by user (kill switch activated)",
                duration_ms=(time.monotonic() - start) * 1000,
                executor_type=ExecutorType.SCREEN,
            )
        except ScreenSafetyError as e:
            return ExecutionResult(
                command=f"screen: {instruction}",
                exit_code=1,
                stdout="",
                stderr=f"Safety blocked: {e}",
                duration_ms=(time.monotonic() - start) * 1000,
                executor_type=ExecutorType.SCREEN,
            )
        except Exception as e:
            return ExecutionResult(
                command=f"screen: {instruction}",
                exit_code=1,
                stdout="",
                stderr=str(e),
                duration_ms=(time.monotonic() - start) * 1000,
                executor_type=ExecutorType.SCREEN,
            )

    # ── Internal helpers ──────────────────────────────────────────

    async def _decide_action(
        self,
        instruction: str,
        analysis: object,
        history: list[dict],
    ) -> dict:
        """Ask the LLM what to do next."""
        prompt = SCREEN_ACTION_PROMPT.format(
            analysis=analysis.description,  # type: ignore[union-attr]
            instruction=instruction,
            history=json.dumps(history[-5:]),  # Last 5 actions only
        )
        request = LLMRequest(
            prompt=prompt,
            tier=LLMTier.STANDARD,
            task_type=TaskType.VISION,
            system_prompt="You control a computer. Respond only with valid JSON.",
        )
        response = await self._gateway.complete(request)  # type: ignore[union-attr]
        try:
            text = response.content.strip()
            if text.startswith("```"):
                text = text.split("```")[1]
                if text.startswith("json"):
                    text = text[4:]
            return json.loads(text)
        except (json.JSONDecodeError, IndexError):
            return {
                "action_type": "error",
                "reason": f"Could not parse LLM response: {response.content[:100]}",
            }

    async def _execute_action(
        self,
        action_data: dict,
        screenshot: Screenshot,
    ) -> object | None:
        """Translate an LLM action dict to controller calls."""
        action_type = action_data.get("action_type", "")
        params = action_data.get("params", {})

        if action_type == "click":
            element_text = params.get("element", action_data.get("target", ""))
            element = await self._analyzer.locate(screenshot, element_text)
            if element:
                return await self._controller.click(element.x, element.y)
            logger.warning("Element not found: %s", element_text)
            return None

        if action_type == "type":
            return await self._controller.type_text(params.get("text", ""))

        if action_type == "hotkey":
            keys = params.get("keys", [])
            if keys:
                return await self._controller.hotkey(*keys)
            return None

        if action_type == "scroll":
            amount = params.get("amount", 3)
            if params.get("direction") == "down":
                amount = -abs(amount)
            return await self._controller.scroll(amount)

        if action_type == "wait":
            return await self._controller.wait(params.get("seconds", 1))

        logger.warning("Unknown action type: %s", action_type)
        return None
