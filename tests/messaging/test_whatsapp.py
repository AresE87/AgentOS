"""Tests for WhatsAppAdapter."""

from __future__ import annotations

from unittest.mock import AsyncMock

from agentos.messaging.whatsapp import WhatsAppAdapter
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
        source="whatsapp",
        status=status,
        output_text=output_text,
        model_used=model_used,
        cost_estimate=cost_estimate,
        error_message=error_message,
    )


async def _dummy_callback(task_input: TaskInput) -> TaskResult:
    return _make_result(output_text="Done", model_used="gpt-4", cost_estimate=0.001)


def _make_adapter(
    on_message=_dummy_callback,
) -> WhatsAppAdapter:
    return WhatsAppAdapter(
        phone_number_id="123456",
        access_token="fake-token",
        verify_token="my-verify",
        on_message=on_message,
    )


# -- _format_result ------------------------------------------------------------


def test_format_result_success() -> None:
    result = _make_result(output_text="Hello world", model_used="gpt-4", cost_estimate=0.0012)
    text = WhatsAppAdapter._format_result(result)  # noqa: SLF001
    assert "Hello world" in text
    assert "gpt-4" in text
    assert "$0.0012" in text


def test_format_result_failed() -> None:
    result = _make_result(status=TaskStatus.FAILED, error_message="Something broke")
    text = WhatsAppAdapter._format_result(result)  # noqa: SLF001
    assert "Error" in text
    assert "Something broke" in text


# -- _split_message ------------------------------------------------------------


def test_split_message_short() -> None:
    assert WhatsAppAdapter._split_message("short") == ["short"]  # noqa: SLF001


def test_split_message_long() -> None:
    text = "A" * 100 + "\n" + "B" * 100
    chunks = WhatsAppAdapter._split_message(text, max_length=110)  # noqa: SLF001
    assert len(chunks) == 2
    assert chunks[0].startswith("A")
    assert "B" in chunks[1]


# -- handle_webhook ------------------------------------------------------------


async def test_handle_webhook_text() -> None:
    callback = AsyncMock(return_value=_make_result(output_text="OK"))
    adapter = _make_adapter(on_message=callback)
    # Fake an initialised client so send_message works
    mock_client = AsyncMock()
    adapter._client = mock_client  # noqa: SLF001

    body = {
        "entry": [
            {
                "changes": [
                    {
                        "value": {
                            "messages": [
                                {
                                    "from": "15551234567",
                                    "type": "text",
                                    "text": {"body": "hello agent"},
                                }
                            ]
                        }
                    }
                ]
            }
        ]
    }

    await adapter.handle_webhook(body)

    callback.assert_awaited_once()
    task: TaskInput = callback.call_args.args[0]
    assert task.text == "hello agent"
    assert task.source == "whatsapp"
    assert task.chat_id == "15551234567"


async def test_handle_webhook_empty() -> None:
    callback = AsyncMock()
    adapter = _make_adapter(on_message=callback)

    await adapter.handle_webhook({})

    callback.assert_not_awaited()


# -- verify_webhook ------------------------------------------------------------


def test_verify_webhook_valid() -> None:
    adapter = _make_adapter()
    result = adapter.verify_webhook("subscribe", "my-verify", "challenge-123")
    assert result == "challenge-123"


def test_verify_webhook_invalid() -> None:
    adapter = _make_adapter()
    assert adapter.verify_webhook("subscribe", "wrong-token", "challenge") is None
    assert adapter.verify_webhook("unsubscribe", "my-verify", "challenge") is None


# -- send_message payload ------------------------------------------------------


async def test_send_message_formats_payload() -> None:
    adapter = _make_adapter()
    mock_client = AsyncMock()
    adapter._client = mock_client  # noqa: SLF001

    await adapter.send_message("15551234567", "Hello!")

    mock_client.post.assert_awaited_once()
    args, kwargs = mock_client.post.call_args
    assert args[0] == "/messages"
    payload = kwargs["json"]
    assert payload["messaging_product"] == "whatsapp"
    assert payload["to"] == "15551234567"
    assert payload["type"] == "text"
    assert payload["text"]["body"] == "Hello!"
