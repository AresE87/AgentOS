"""Tests for ML-based task classifier (v2)."""

from __future__ import annotations

import json
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from pathlib import Path

from agentos.gateway.classifier_ml import MLClassifier
from agentos.types import TaskInput, TaskType


def _make_input(text: str) -> TaskInput:
    return TaskInput(text=text, source="test", chat_id="test-chat")


@pytest.mark.asyncio()
async def test_fallback_when_no_model() -> None:
    """Without a model file, MLClassifier falls back to rules."""
    clf = MLClassifier(model_path=None)
    assert not clf.is_model_loaded

    result = await clf.classify(_make_input("write a python function"))
    assert result.task_type == TaskType.CODE


@pytest.mark.asyncio()
async def test_classify_with_trained_model(tmp_path: Path) -> None:
    """Load pre-built weights and classify correctly."""
    weights = {
        "screenshot": {"vision": 5.0, "text": 0.1},
        "screen": {"vision": 4.0, "text": 0.1},
        "click": {"vision": 3.0, "text": 0.0},
    }
    model_path = tmp_path / "model.json"
    model_path.write_text(json.dumps(weights))

    clf = MLClassifier(model_path=model_path)
    assert clf.is_model_loaded

    result = await clf.classify(_make_input("take a screenshot of the screen"))
    assert result.task_type == TaskType.VISION


@pytest.mark.asyncio()
async def test_low_confidence_fallback(tmp_path: Path) -> None:
    """Low ML scores trigger fallback to rule-based classifier."""
    # Weights that produce tiny scores for everything
    weights = {
        "xyzzy": {"text": 0.01},
    }
    model_path = tmp_path / "model.json"
    model_path.write_text(json.dumps(weights))

    clf = MLClassifier(model_path=model_path)
    # Input with no matching weight words -- falls back to rules
    result = await clf.classify(_make_input("write a python function"))
    assert result.task_type == TaskType.CODE  # rules detect code keywords


def test_train_from_data(tmp_path: Path) -> None:
    """train_from_data produces a weights JSON file."""
    data = [
        ("write a python function", "code"),
        ("create a script", "code"),
        ("take a screenshot", "vision"),
        ("click the button", "vision"),
        ("hello there", "text"),
    ]
    output = tmp_path / "trained_model.json"
    MLClassifier.train_from_data(data, output)

    assert output.exists()
    weights = json.loads(output.read_text())
    assert isinstance(weights, dict)
    assert len(weights) > 0
    # "python" should have a code weight
    assert "python" in weights
    assert "code" in weights["python"]


@pytest.mark.asyncio()
async def test_is_model_loaded(tmp_path: Path) -> None:
    """is_model_loaded is True after successful load."""
    weights = {"test": {"text": 1.0}}
    model_path = tmp_path / "model.json"
    model_path.write_text(json.dumps(weights))

    clf_no_model = MLClassifier(model_path=None)
    assert not clf_no_model.is_model_loaded

    clf_with_model = MLClassifier(model_path=model_path)
    assert clf_with_model.is_model_loaded
