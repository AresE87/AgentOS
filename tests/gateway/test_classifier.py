"""Tests for RuleBasedClassifier — 30+ cases covering all task types and edge cases."""

from __future__ import annotations

import pytest

from agentos.gateway.classifier import RuleBasedClassifier
from agentos.types import TaskClassification, TaskInput, TaskType

# ── Helpers ───────────────────────────────────────────────────────────


def _make_input(text: str) -> TaskInput:
    return TaskInput(text=text, source="test", chat_id="test-chat")


@pytest.fixture()
def classifier() -> RuleBasedClassifier:
    return RuleBasedClassifier()


# Allow ±1 tolerance on complexity
def _assert_complexity(result: TaskClassification, expected: int) -> None:
    assert abs(result.complexity - expected) <= 1, (
        f"Complexity {result.complexity} not within ±1 of expected {expected}"
    )


# ── TEXT tests (8) ────────────────────────────────────────────────────


@pytest.mark.asyncio()
async def test_text_hello(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(_make_input("hello"))
    assert r.task_type == TaskType.TEXT
    _assert_complexity(r, 1)


@pytest.mark.asyncio()
async def test_text_time(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(_make_input("what time is it?"))
    assert r.task_type == TaskType.TEXT
    _assert_complexity(r, 1)


@pytest.mark.asyncio()
async def test_text_explain(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(_make_input("explain quantum computing"))
    assert r.task_type == TaskType.TEXT
    _assert_complexity(r, 2)


@pytest.mark.asyncio()
async def test_text_hola(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(_make_input("hola, cómo estás?"))
    assert r.task_type == TaskType.TEXT
    _assert_complexity(r, 1)


@pytest.mark.asyncio()
async def test_text_email(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(
        _make_input("write me an email to my boss explaining I'll be late")
    )
    assert r.task_type == TaskType.TEXT
    _assert_complexity(r, 2)


@pytest.mark.asyncio()
async def test_text_complex_summary(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(
        _make_input(
            "summarize the following article and highlight key points "
            "and suggest follow-up questions"
        )
    )
    assert r.task_type == TaskType.TEXT
    _assert_complexity(r, 3)


@pytest.mark.asyncio()
async def test_text_translate(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(_make_input("translate this paragraph to Spanish"))
    assert r.task_type == TaskType.TEXT
    _assert_complexity(r, 2)


@pytest.mark.asyncio()
async def test_text_opinion(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(_make_input("what do you think about the future of remote work?"))
    assert r.task_type == TaskType.TEXT
    _assert_complexity(r, 3)


# ── CODE tests (8) ───────────────────────────────────────────────────


@pytest.mark.asyncio()
async def test_code_list_files(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(_make_input("list files in /home"))
    assert r.task_type == TaskType.CODE
    _assert_complexity(r, 1)


@pytest.mark.asyncio()
async def test_code_python_function(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(_make_input("write a Python function to sort a list"))
    assert r.task_type == TaskType.CODE
    _assert_complexity(r, 2)


@pytest.mark.asyncio()
async def test_code_block(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(
        _make_input("```python\ndef hello():\n    pass\n```\nfix this function")
    )
    assert r.task_type == TaskType.CODE
    _assert_complexity(r, 2)


@pytest.mark.asyncio()
async def test_code_debug_error(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(_make_input("debug this error: ModuleNotFoundError"))
    assert r.task_type == TaskType.CODE
    _assert_complexity(r, 2)


@pytest.mark.asyncio()
async def test_code_rest_api(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(
        _make_input("create a REST API with authentication and database connection")
    )
    assert r.task_type == TaskType.CODE
    _assert_complexity(r, 4)


@pytest.mark.asyncio()
async def test_code_multi_step_script(
    classifier: RuleBasedClassifier,
) -> None:
    r = await classifier.classify(
        _make_input(
            "write a script that reads a CSV, processes data, "
            "generates a report, and sends it by email"
        )
    )
    assert r.task_type == TaskType.CODE
    _assert_complexity(r, 4)


@pytest.mark.asyncio()
async def test_code_spanish(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(
        _make_input("escribe una función en Python que ordene diccionarios")
    )
    assert r.task_type == TaskType.CODE
    _assert_complexity(r, 2)


@pytest.mark.asyncio()
async def test_code_review(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(
        _make_input("review this code and suggest improvements for performance and readability")
    )
    assert r.task_type == TaskType.CODE
    _assert_complexity(r, 3)


# ── VISION tests (4) ─────────────────────────────────────────────────


@pytest.mark.asyncio()
async def test_vision_screenshot(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(_make_input("take a screenshot"))
    assert r.task_type == TaskType.VISION
    _assert_complexity(r, 1)


@pytest.mark.asyncio()
async def test_vision_screen(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(_make_input("what's on my screen?"))
    assert r.task_type == TaskType.VISION
    _assert_complexity(r, 2)


@pytest.mark.asyncio()
async def test_vision_click(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(_make_input("click the submit button"))
    assert r.task_type == TaskType.VISION
    _assert_complexity(r, 1)


@pytest.mark.asyncio()
async def test_vision_navigate(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(
        _make_input("navigate to settings and change the theme to dark mode")
    )
    assert r.task_type == TaskType.VISION
    _assert_complexity(r, 3)


# ── DATA tests (5) ───────────────────────────────────────────────────


@pytest.mark.asyncio()
async def test_data_csv(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(_make_input("analyze this CSV file"))
    assert r.task_type == TaskType.DATA
    _assert_complexity(r, 2)


@pytest.mark.asyncio()
async def test_data_chart(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(_make_input("create a chart showing monthly revenue"))
    assert r.task_type == TaskType.DATA
    _assert_complexity(r, 2)


@pytest.mark.asyncio()
async def test_data_stats(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(
        _make_input("calculate the average and standard deviation of column B")
    )
    assert r.task_type == TaskType.DATA
    _assert_complexity(r, 2)


@pytest.mark.asyncio()
async def test_data_multi_step(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(
        _make_input(
            "process the sales data, create a pivot table, generate charts, and prepare a report"
        )
    )
    assert r.task_type == TaskType.DATA
    _assert_complexity(r, 4)


@pytest.mark.asyncio()
async def test_data_spanish(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(_make_input("cuántos registros hay en la planilla?"))
    assert r.task_type == TaskType.DATA
    _assert_complexity(r, 1)


# ── GENERATION tests (3) ─────────────────────────────────────────────


@pytest.mark.asyncio()
async def test_gen_logo(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(_make_input("generate a logo for my company"))
    assert r.task_type == TaskType.GENERATION
    _assert_complexity(r, 2)


@pytest.mark.asyncio()
async def test_gen_image(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(_make_input("create an image of a sunset over mountains"))
    assert r.task_type == TaskType.GENERATION
    _assert_complexity(r, 2)


@pytest.mark.asyncio()
async def test_gen_poster(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(
        _make_input("design a poster for our event with illustrations and typography")
    )
    assert r.task_type == TaskType.GENERATION
    _assert_complexity(r, 3)


# ── Edge cases (2) ───────────────────────────────────────────────────


@pytest.mark.asyncio()
async def test_edge_empty(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(_make_input(""))
    assert r.task_type == TaskType.TEXT
    assert r.complexity == 1
    assert r.confidence < 0.5


@pytest.mark.asyncio()
async def test_edge_multi_type(classifier: RuleBasedClassifier) -> None:
    r = await classifier.classify(
        _make_input("write code to generate an image from data in a spreadsheet")
    )
    # CODE wins by priority; moderate complexity
    assert r.task_type == TaskType.CODE
    _assert_complexity(r, 2)
