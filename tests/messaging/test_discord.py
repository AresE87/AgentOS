"""Tests for DiscordAdapter."""

from __future__ import annotations

from agentos.messaging.discord import DiscordAdapter
from agentos.types import TaskInput, TaskResult, TaskStatus

# -- Helpers -------------------------------------------------------------------


def _make_result(
    *,
    status: TaskStatus = TaskStatus.COMPLETED,
    output_text: str = "",
    model_used: str | None = None,
    cost_estimate: float = 0.0,
    error_message: str = "",
) -> TaskResult:
    return TaskResult(
        task_id="abc123",
        input_text="test",
        source="discord",
        status=status,
        output_text=output_text,
        model_used=model_used,
        cost_estimate=cost_estimate,
        error_message=error_message,
    )


async def _dummy_callback(task_input: TaskInput) -> TaskResult:
    return _make_result(output_text="Done", model_used="gpt-4", cost_estimate=0.001)


# -- _format_result ------------------------------------------------------------


def test_format_result_success() -> None:
    result = _make_result(output_text="Hello world", model_used="gpt-4", cost_estimate=0.0012)
    text = DiscordAdapter._format_result(result)  # noqa: SLF001
    assert "Hello world" in text
    assert "gpt-4" in text
    assert "$0.0012" in text


def test_format_result_failed() -> None:
    result = _make_result(status=TaskStatus.FAILED, error_message="Something broke")
    text = DiscordAdapter._format_result(result)  # noqa: SLF001
    assert "**Error:**" in text
    assert "Something broke" in text


# -- _split_message ------------------------------------------------------------


def test_split_message_short() -> None:
    assert DiscordAdapter._split_message("short") == ["short"]  # noqa: SLF001


def test_split_message_discord_limit() -> None:
    # Discord limit is 2000
    text = "A" * 1500 + "\n" + "B" * 1500
    chunks = DiscordAdapter._split_message(text)  # noqa: SLF001
    assert len(chunks) == 2
    assert all(len(c) <= 2000 for c in chunks)
    assert chunks[0].startswith("A")
    assert "B" in chunks[1]


# -- Adapter init --------------------------------------------------------------


def test_adapter_init() -> None:
    adapter = DiscordAdapter(token="fake-token", on_message=_dummy_callback)
    assert adapter._token == "fake-token"  # noqa: SLF001
    assert adapter._bot is None  # noqa: SLF001
    assert adapter._running is False  # noqa: SLF001
