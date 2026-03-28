"""Proactive suggestions — detects patterns and suggests actions."""

from __future__ import annotations

from collections import Counter
from dataclasses import dataclass
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from datetime import datetime

from agentos.utils.logging import get_logger

logger = get_logger("proactive")


@dataclass
class Suggestion:
    id: str
    type: str  # "recurring_task", "sequence", "optimization", "maintenance"
    title: str
    description: str
    action: str  # What to do if accepted
    confidence: float = 0.0
    dismissed: bool = False
    snoozed_until: datetime | None = None


class ProactiveEngine:
    """Analyzes usage patterns and generates suggestions."""

    def __init__(self, store=None, max_suggestions: int = 3) -> None:
        self._store = store
        self._max_suggestions = max_suggestions
        self._suggestions: list[Suggestion] = []
        self._dismissed_ids: set[str] = set()

    async def analyze(self) -> list[Suggestion]:
        """Analyze patterns and generate suggestions."""
        self._suggestions.clear()

        if not self._store:
            return []

        tasks = await self._store.get_recent_tasks(limit=500)
        if len(tasks) < 5:
            return []

        self._detect_recurring(tasks)
        self._detect_sequences(tasks)
        self._detect_optimization(tasks)

        # Filter dismissed and limit
        active = [
            s for s in self._suggestions if s.id not in self._dismissed_ids and not s.dismissed
        ]
        return active[: self._max_suggestions]

    def dismiss(self, suggestion_id: str) -> None:
        self._dismissed_ids.add(suggestion_id)

    def snooze(self, suggestion_id: str, until: datetime) -> None:
        for s in self._suggestions:
            if s.id == suggestion_id:
                s.snoozed_until = until

    def _detect_recurring(self, tasks: list[dict]) -> None:
        """Detect tasks that repeat at similar times."""
        text_counter: Counter = Counter()
        for t in tasks:
            text = t.get("input_text", "").lower().strip()
            if text and len(text) > 5:
                # Normalize
                key = text[:50]
                text_counter[key] += 1

        for text, count in text_counter.most_common(3):
            if count >= 3:
                self._suggestions.append(
                    Suggestion(
                        id=f"recurring_{hash(text) % 10000}",
                        type="recurring_task",
                        title="Recurring task detected",
                        description=(
                            f"You've run '{text[:40]}...' {count} times. Want to automate it?"
                        ),
                        action=f"schedule:{text}",
                        confidence=min(count / 10, 0.9),
                    )
                )

    def _detect_sequences(self, tasks: list[dict]) -> None:
        """Detect tasks that often happen in sequence."""
        if len(tasks) < 4:
            return
        pairs: Counter = Counter()
        for i in range(len(tasks) - 1):
            a = tasks[i].get("input_text", "")[:30]
            b = tasks[i + 1].get("input_text", "")[:30]
            if a and b and a != b:
                pairs[(a, b)] += 1

        for (a, b), count in pairs.most_common(1):
            if count >= 2:
                self._suggestions.append(
                    Suggestion(
                        id=f"sequence_{hash(a + b) % 10000}",
                        type="sequence",
                        title="Frequent sequence detected",
                        description=(f"You often do '{a}' then '{b}'. Create a combined playbook?"),
                        action=f"create_playbook:{a}|{b}",
                        confidence=min(count / 5, 0.8),
                    )
                )

    def _detect_optimization(self, tasks: list[dict]) -> None:
        """Detect cost optimization opportunities."""
        tier3_simple = 0
        for t in tasks:
            tier = t.get("tier", 1) or 1
            complexity = t.get("complexity", 1) or 1
            if tier >= 3 and complexity <= 2:
                tier3_simple += 1

        if tier3_simple >= 3:
            self._suggestions.append(
                Suggestion(
                    id="optimize_tier",
                    type="optimization",
                    title="Cost optimization available",
                    description=(
                        f"{tier3_simple} simple tasks used premium models. "
                        "Switch to cheaper models to save money."
                    ),
                    action="optimize_routing",
                    confidence=0.85,
                )
            )
