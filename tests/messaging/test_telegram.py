"""Tests for TelegramAdapter with mocked Telegram API."""

from __future__ import annotations

from unittest.mock import AsyncMock, MagicMock, patch

from agentos.messaging.telegram import TelegramAdapter, format_result, split_message
from agentos.types import TaskInput, TaskResult, TaskStatus

# ── Helpers ───────────────────────────────────────────────────────────


def _make_result(
    *,
    status: TaskStatus = TaskStatus.COMPLETED,
    output_text: str = "",
    model_used: str | None = None,
    cost_estimate: float = 0.0,
    duration_ms: float = 0.0,
    error_message: str = "",
) -> TaskResult:
    return TaskResult(
        task_id="abc123",
        input_text="test",
        source="telegram",
        status=status,
        output_text=output_text,
        model_used=model_used,
        cost_estimate=cost_estimate,
        duration_ms=duration_ms,
        error_message=error_message,
    )


def _make_update(text: str = "hello", chat_id: int = 12345) -> MagicMock:
    """Create a mock Telegram Update with a message."""
    update = MagicMock()
    update.message.text = text
    update.message.chat_id = chat_id
    update.message.reply_text = AsyncMock()
    update.message.chat.send_action = AsyncMock()
    return update


async def _dummy_callback(task_input: TaskInput) -> TaskResult:
    return _make_result(output_text="Done", model_used="gpt-4", cost_estimate=0.001)


# ── /start command ────────────────────────────────────────────────────


async def test_handle_start_sends_welcome() -> None:
    adapter = TelegramAdapter(token="fake:token", on_message=_dummy_callback)
    update = _make_update()
    context = MagicMock()

    await adapter._handle_start(update, context)  # noqa: SLF001

    update.message.reply_text.assert_awaited_once()
    text = update.message.reply_text.call_args.args[0]
    assert "Welcome to AgentOS" in text


# ── /status command ───────────────────────────────────────────────────


async def test_handle_status_shows_running() -> None:
    adapter = TelegramAdapter(token="fake:token", on_message=_dummy_callback)
    update = _make_update()
    context = MagicMock()

    await adapter._handle_status(update, context)  # noqa: SLF001

    text = update.message.reply_text.call_args.args[0]
    assert "running" in text.lower()


# ── /history command ──────────────────────────────────────────────────


async def test_handle_history_no_tasks() -> None:
    adapter = TelegramAdapter(token="fake:token", on_message=_dummy_callback)
    update = _make_update()
    context = MagicMock()

    await adapter._handle_history(update, context)  # noqa: SLF001

    text = update.message.reply_text.call_args.args[0]
    assert "No tasks yet" in text


# ── /help command ─────────────────────────────────────────────────────


async def test_handle_help_shows_commands() -> None:
    adapter = TelegramAdapter(token="fake:token", on_message=_dummy_callback)
    update = _make_update()
    context = MagicMock()

    await adapter._handle_help(update, context)  # noqa: SLF001

    text = update.message.reply_text.call_args.args[0]
    assert "/start" in text
    assert "/help" in text
    assert "/status" in text


# ── Normal text message ───────────────────────────────────────────────


async def test_handle_message_calls_on_message() -> None:
    callback = AsyncMock(return_value=_make_result(output_text="Done", model_used="gpt-4"))
    adapter = TelegramAdapter(token="fake:token", on_message=callback)
    adapter._app = MagicMock()  # noqa: SLF001
    adapter._app.bot.send_message = AsyncMock()  # noqa: SLF001

    update = _make_update(text="do something", chat_id=99)
    context = MagicMock()

    await adapter._handle_message(update, context)  # noqa: SLF001

    callback.assert_awaited_once()
    task_input: TaskInput = callback.call_args.args[0]
    assert task_input.text == "do something"
    assert task_input.chat_id == "99"
    assert task_input.source == "telegram"


# ── Long response splitting ──────────────────────────────────────────


async def test_long_response_splits_with_markers() -> None:
    long_text = "\n".join([f"Line {i}" for i in range(800)])
    callback = AsyncMock(return_value=_make_result(output_text=long_text))
    adapter = TelegramAdapter(token="fake:token", on_message=callback)
    adapter._app = MagicMock()  # noqa: SLF001
    adapter._app.bot.send_message = AsyncMock()  # noqa: SLF001

    update = _make_update(text="generate long output")
    context = MagicMock()

    await adapter._handle_message(update, context)  # noqa: SLF001

    call_count = adapter._app.bot.send_message.await_count  # noqa: SLF001
    assert call_count > 1

    # Each chunk except possibly the last should have a (i/N) marker.
    for call in adapter._app.bot.send_message.call_args_list:  # noqa: SLF001
        chunk_text: str = call.kwargs.get("text", call.args[0] if call.args else "")
        assert "(1/" in chunk_text or "(2/" in chunk_text or "/" in chunk_text


# ── AgentCore failure ─────────────────────────────────────────────────


async def test_handle_message_error_sends_error_emoji() -> None:
    callback = AsyncMock(side_effect=RuntimeError("boom"))
    adapter = TelegramAdapter(token="fake:token", on_message=callback)
    adapter._app = MagicMock()  # noqa: SLF001

    update = _make_update(text="bad task")
    context = MagicMock()

    await adapter._handle_message(update, context)  # noqa: SLF001

    update.message.reply_text.assert_awaited_once()
    text = update.message.reply_text.call_args.args[0]
    assert "\u274c" in text  # ❌
    assert "/status" in text


# ── Empty message ─────────────────────────────────────────────────────


async def test_handle_message_empty_text_sends_help() -> None:
    callback = AsyncMock()
    adapter = TelegramAdapter(token="fake:token", on_message=callback)

    update = _make_update(text="   ")
    context = MagicMock()

    await adapter._handle_message(update, context)  # noqa: SLF001

    callback.assert_not_awaited()
    text = update.message.reply_text.call_args.args[0]
    assert "/help" in text


async def test_handle_message_none_message_ignored() -> None:
    callback = AsyncMock()
    adapter = TelegramAdapter(token="fake:token", on_message=callback)

    update = MagicMock()
    update.message = None
    context = MagicMock()

    await adapter._handle_message(update, context)  # noqa: SLF001

    callback.assert_not_awaited()


# ── format_result ─────────────────────────────────────────────────────


def test_format_result_success_has_checkmark_and_model() -> None:
    result = _make_result(
        output_text="Hello world",
        model_used="gpt-4",
        cost_estimate=0.0012,
        duration_ms=1500.0,
    )
    formatted = format_result(result)
    assert "\u2705" in formatted  # ✅
    assert "Done" in formatted
    assert "Hello world" in formatted
    assert "gpt-4" in formatted
    assert "$0.0012" in formatted
    assert "1.5s" in formatted


def test_format_result_error_has_cross_and_message() -> None:
    result = _make_result(
        status=TaskStatus.FAILED,
        error_message="Something broke",
    )
    formatted = format_result(result)
    assert "\u274c" in formatted  # ❌
    assert "Something broke" in formatted
    assert "/status" in formatted


def test_format_result_failed_no_message() -> None:
    result = _make_result(status=TaskStatus.FAILED)
    formatted = format_result(result)
    assert "Unknown error" in formatted


def test_format_result_no_output() -> None:
    result = _make_result()
    formatted = format_result(result)
    assert "no output" in formatted.lower()


# ── split_message ─────────────────────────────────────────────────────


def test_split_message_short_unchanged() -> None:
    assert split_message("Short message") == ["Short message"]


def test_split_message_adds_markers() -> None:
    text = "a" * 5000
    chunks = split_message(text, max_length=2000)
    assert len(chunks) > 1
    assert "(1/" in chunks[0]
    assert "(2/" in chunks[1]


def test_split_message_prefers_newline_split() -> None:
    text = "A" * 100 + "\n" + "B" * 100
    chunks = split_message(text, max_length=110)
    # Should have split on the newline
    assert len(chunks) == 2
    assert chunks[0].startswith("A")
    assert "B" in chunks[1]


# ── Invalid token handling ────────────────────────────────────────────


async def test_invalid_token_disables_adapter() -> None:
    from telegram.error import InvalidToken as _InvalidToken

    adapter = TelegramAdapter(token="bad-token", on_message=_dummy_callback)

    with patch("agentos.messaging.telegram.Application.builder") as mock_builder:
        mock_builder.return_value.token.return_value.build.side_effect = _InvalidToken("bad")

        await adapter.start()

    assert adapter._disabled is True  # noqa: SLF001
