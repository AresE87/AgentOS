"""ML-based task classifier (v2).

Uses a simple keyword-weight model trained from historical data.
Falls back to rule-based classifier (v1) when model unavailable.
"""

from __future__ import annotations

import json
from collections import Counter
from typing import TYPE_CHECKING

from agentos.gateway.classifier import BaseClassifier, RuleBasedClassifier
from agentos.types import TaskClassification, TaskInput, TaskType
from agentos.utils.logging import get_logger

if TYPE_CHECKING:
    from pathlib import Path

logger = get_logger("gateway.classifier_ml")


class MLClassifier(BaseClassifier):
    """ML-based classifier with rule-based fallback.

    v2 uses a keyword-weight model derived from historical classification data.
    If model not available, falls back to RuleBasedClassifier.
    """

    def __init__(self, model_path: Path | None = None) -> None:
        self._fallback = RuleBasedClassifier()
        self._model_loaded = False
        self._weights: dict[str, dict[str, float]] = {}  # word -> {task_type: weight}

        if model_path and model_path.exists():
            self._load_model(model_path)

    def _load_model(self, path: Path) -> None:
        """Load keyword weights from JSON."""
        try:
            with open(path) as f:
                self._weights = json.load(f)
            self._model_loaded = True
            logger.info("ML classifier model loaded from %s", path)
        except Exception:
            logger.warning("Failed to load ML model, using fallback")

    @property
    def is_model_loaded(self) -> bool:
        return self._model_loaded

    async def classify(self, task_input: TaskInput) -> TaskClassification:
        """Classify using ML model with fallback to rules."""
        if not self._model_loaded:
            return await self._fallback.classify(task_input)

        text = task_input.text.lower().strip()
        if not text:
            return await self._fallback.classify(task_input)

        # Score each task type
        scores: dict[str, float] = {t.value: 0.0 for t in TaskType}
        words = text.split()

        for word in words:
            if word in self._weights:
                for task_type, weight in self._weights[word].items():
                    if task_type in scores:
                        scores[task_type] += weight

        # Find best type
        best_type = max(scores, key=lambda k: scores[k])
        best_score = scores[best_type]

        if best_score < 0.5:
            # Low confidence -- fallback to rules
            return await self._fallback.classify(task_input)

        # Normalize confidence
        total = sum(scores.values()) or 1.0
        confidence = best_score / total

        # Estimate complexity from rules (reuse v1 logic)
        rule_result = await self._fallback.classify(task_input)

        task_type = TaskType(best_type)
        tier = rule_result.tier  # Reuse complexity->tier from rules

        return TaskClassification(
            task_type=task_type,
            complexity=rule_result.complexity,
            tier=tier,
            confidence=min(confidence, 0.95),
            reasoning=f"ML classifier (score={best_score:.2f})",
        )

    @staticmethod
    def train_from_data(data: list[tuple[str, str]], output_path: Path) -> None:
        """Train keyword weights from (text, task_type) pairs.

        Simple bag-of-words approach with TF-IDF-like weighting.
        """
        type_word_counts: dict[str, Counter[str]] = {t.value: Counter() for t in TaskType}
        type_counts: dict[str, int] = {t.value: 0 for t in TaskType}

        for text, task_type in data:
            if task_type not in type_counts:
                continue
            type_counts[task_type] += 1
            for word in text.lower().split():
                if len(word) > 2:
                    type_word_counts[task_type][word] += 1

        # Convert to weights
        weights: dict[str, dict[str, float]] = {}
        for task_type, word_counts in type_word_counts.items():
            doc_count = type_counts[task_type] or 1
            for word, count in word_counts.items():
                if word not in weights:
                    weights[word] = {}
                # TF-IDF-ish: frequency in type / (total docs of that type)
                weights[word][task_type] = count / doc_count

        output_path.parent.mkdir(parents=True, exist_ok=True)
        with open(output_path, "w") as f:
            json.dump(weights, f)

        logger.info("Trained ML model: %d words, saved to %s", len(weights), output_path)
