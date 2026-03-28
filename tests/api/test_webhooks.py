"""Tests for webhook delivery system (AOS-073)."""

from __future__ import annotations

from unittest.mock import AsyncMock, patch

import httpx
import pytest

from agentos.api.webhooks import WebhookManager


@pytest.fixture
def manager() -> WebhookManager:
    return WebhookManager(max_retries=3)


def test_register_webhook(manager: WebhookManager) -> None:
    wh = manager.register(
        url="https://example.com/hook",
        events=["task.completed"],
        secret="test-secret",
    )
    assert wh.id
    assert wh.url == "https://example.com/hook"
    assert wh.secret == "test-secret"
    assert len(manager.list_webhooks()) == 1


def test_unregister_webhook(manager: WebhookManager) -> None:
    wh = manager.register(
        url="https://example.com/hook",
        events=["task.completed"],
    )
    assert manager.unregister(wh.id) is True
    assert len(manager.list_webhooks()) == 0
    assert manager.unregister("nonexistent") is False


async def test_dispatch_matching_event(manager: WebhookManager) -> None:
    manager.register(
        url="https://example.com/hook",
        events=["task.completed"],
        secret="s",
    )

    mock_response = httpx.Response(200, request=httpx.Request("POST", "https://example.com/hook"))

    with patch("httpx.AsyncClient.post", new_callable=AsyncMock, return_value=mock_response):
        deliveries = await manager.dispatch("task.completed", {"task_id": "t1"})

    assert len(deliveries) == 1
    assert deliveries[0].success is True
    assert deliveries[0].status_code == 200


async def test_dispatch_non_matching(manager: WebhookManager) -> None:
    manager.register(
        url="https://example.com/hook",
        events=["task.completed"],
        secret="s",
    )
    deliveries = await manager.dispatch("task.started", {"task_id": "t1"})
    assert len(deliveries) == 0


def test_signature_verification() -> None:
    body = '{"event":"task.completed"}'
    secret = "my-secret"
    sig = WebhookManager._sign(body, secret)
    assert WebhookManager.verify_signature(body, secret, sig) is True


def test_signature_invalid() -> None:
    body = '{"event":"task.completed"}'
    sig = WebhookManager._sign(body, "correct-secret")
    assert WebhookManager.verify_signature(body, "wrong-secret", sig) is False


async def test_delivery_with_mock(manager: WebhookManager) -> None:
    wh = manager.register(
        url="https://example.com/hook",
        events=["task.completed"],
        secret="s",
    )

    mock_response = httpx.Response(200, request=httpx.Request("POST", "https://example.com/hook"))

    with patch("httpx.AsyncClient.post", new_callable=AsyncMock, return_value=mock_response):
        deliveries = await manager.dispatch("task.completed", {"result": "ok"})

    assert len(deliveries) == 1
    assert deliveries[0].success is True
    assert deliveries[0].webhook_id == wh.id


async def test_delivery_failure_retry(manager: WebhookManager) -> None:
    manager.register(
        url="https://example.com/hook",
        events=["task.failed"],
        secret="s",
    )

    mock_response = httpx.Response(500, request=httpx.Request("POST", "https://example.com/hook"))

    with (
        patch("httpx.AsyncClient.post", new_callable=AsyncMock, return_value=mock_response),
        patch("asyncio.sleep", new_callable=AsyncMock),
    ):
        deliveries = await manager.dispatch("task.failed", {"error": "boom"})

    assert len(deliveries) == 1
    assert deliveries[0].success is False
    assert deliveries[0].attempt == 3  # exhausted all retries


async def test_get_deliveries(manager: WebhookManager) -> None:
    wh = manager.register(
        url="https://example.com/hook",
        events=["task.completed"],
        secret="s",
    )

    mock_response = httpx.Response(200, request=httpx.Request("POST", "https://example.com/hook"))

    with patch("httpx.AsyncClient.post", new_callable=AsyncMock, return_value=mock_response):
        await manager.dispatch("task.completed", {"task_id": "t1"})
        await manager.dispatch("task.completed", {"task_id": "t2"})

    all_deliveries = manager.get_deliveries()
    assert len(all_deliveries) == 2

    filtered = manager.get_deliveries(webhook_id=wh.id)
    assert len(filtered) == 2
