"""Tests for TaskDecomposer (AOS-034)."""

from __future__ import annotations

import json
from unittest.mock import AsyncMock, MagicMock

import pytest

from agentos.hierarchy.decomposer import TaskDecomposer
from agentos.hierarchy.levels import AgentLevel
from agentos.types import LLMTier, TaskClassification, TaskInput, TaskType


def _make_classification(complexity: int) -> TaskClassification:
    return TaskClassification(
        task_type=TaskType.TEXT,
        complexity=complexity,
        tier=LLMTier.STANDARD,
        confidence=0.9,
        reasoning="test",
    )


def _make_task_input(text: str = "Build a web app") -> TaskInput:
    return TaskInput(text=text, source="test", task_id="t1")


# ---- should_decompose ---------------------------------------------------


def test_should_decompose_simple():
    decomposer = TaskDecomposer()
    classification = _make_classification(complexity=2)
    assert decomposer.should_decompose(classification) is False


def test_should_decompose_complex():
    decomposer = TaskDecomposer()
    classification = _make_classification(complexity=3)
    assert decomposer.should_decompose(classification) is True


# ---- decompose -----------------------------------------------------------


@pytest.mark.asyncio
async def test_decompose_parses_json():
    gateway = MagicMock()
    response = MagicMock()
    response.content = json.dumps(
        {
            "subtasks": [
                {
                    "id": "subtask_1",
                    "description": "Set up project",
                    "depends_on": [],
                    "suggested_level": "junior",
                    "suggested_specialist": None,
                    "estimated_complexity": 1,
                },
                {
                    "id": "subtask_2",
                    "description": "Build backend",
                    "depends_on": ["subtask_1"],
                    "suggested_level": "specialist",
                    "suggested_specialist": "software_development",
                    "estimated_complexity": 3,
                },
            ],
            "reasoning": "Split into setup and build phases",
        }
    )
    gateway.complete = AsyncMock(return_value=response)

    decomposer = TaskDecomposer(gateway=gateway)
    plan = await decomposer.decompose(_make_task_input(), _make_classification(complexity=4))

    assert len(plan.subtasks) == 2
    assert plan.subtasks[0].id == "subtask_1"
    assert plan.subtasks[0].suggested_level == AgentLevel.JUNIOR
    assert plan.subtasks[1].depends_on == ["subtask_1"]
    assert plan.subtasks[1].suggested_level == AgentLevel.SPECIALIST
    assert plan.reasoning == "Split into setup and build phases"
    gateway.complete.assert_awaited_once()


@pytest.mark.asyncio
async def test_decompose_invalid_json():
    gateway = MagicMock()
    response = MagicMock()
    response.content = "This is not valid JSON at all!"
    gateway.complete = AsyncMock(return_value=response)

    decomposer = TaskDecomposer(gateway=gateway)
    plan = await decomposer.decompose(
        _make_task_input("Do something"), _make_classification(complexity=4)
    )

    assert len(plan.subtasks) == 1
    assert plan.subtasks[0].description == "Do something"
    assert "fallback" in plan.reasoning.lower()


@pytest.mark.asyncio
async def test_decompose_max_10():
    gateway = MagicMock()
    subtasks = [
        {"id": f"subtask_{i}", "description": f"Step {i}", "depends_on": []} for i in range(15)
    ]
    response = MagicMock()
    response.content = json.dumps({"subtasks": subtasks, "reasoning": "many steps"})
    gateway.complete = AsyncMock(return_value=response)

    decomposer = TaskDecomposer(gateway=gateway)
    plan = await decomposer.decompose(_make_task_input(), _make_classification(complexity=5))

    assert len(plan.subtasks) == 10


@pytest.mark.asyncio
async def test_decompose_no_gateway():
    decomposer = TaskDecomposer(gateway=None)
    plan = await decomposer.decompose(
        _make_task_input("Simple task"), _make_classification(complexity=4)
    )

    assert len(plan.subtasks) == 1
    assert plan.subtasks[0].description == "Simple task"
    assert "no gateway" in plan.reasoning.lower()
