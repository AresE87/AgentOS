"""Webhook delivery system for AgentOS events."""

from __future__ import annotations

import asyncio
import hashlib
import hmac
import json
import uuid
from dataclasses import dataclass, field
from datetime import UTC, datetime
from enum import StrEnum

import httpx

from agentos.utils.logging import get_logger

logger = get_logger("api.webhooks")


class WebhookEvent(StrEnum):
    TASK_STARTED = "task.started"
    TASK_COMPLETED = "task.completed"
    TASK_FAILED = "task.failed"
    CHAIN_COMPLETED = "chain.completed"


@dataclass
class WebhookConfig:
    id: str
    url: str
    events: list[WebhookEvent]
    secret: str  # For HMAC signing
    active: bool = True
    created_at: datetime = field(default_factory=lambda: datetime.now(UTC))


@dataclass
class WebhookDelivery:
    id: str
    webhook_id: str
    event: str
    status_code: int | None = None
    success: bool = False
    attempt: int = 1
    error: str | None = None
    timestamp: datetime = field(default_factory=lambda: datetime.now(UTC))


class WebhookManager:
    """Manages webhook subscriptions and deliveries."""

    def __init__(self, max_retries: int = 3) -> None:
        self._webhooks: dict[str, WebhookConfig] = {}
        self._deliveries: list[WebhookDelivery] = []
        self._max_retries = max_retries

    def register(self, url: str, events: list[str], secret: str | None = None) -> WebhookConfig:
        wh_id = uuid.uuid4().hex[:12]
        wh_secret = secret or uuid.uuid4().hex
        config = WebhookConfig(
            id=wh_id,
            url=url,
            events=[WebhookEvent(e) for e in events],
            secret=wh_secret,
        )
        self._webhooks[wh_id] = config
        logger.info("Webhook registered: %s -> %s", wh_id, url)
        return config

    def unregister(self, webhook_id: str) -> bool:
        if webhook_id in self._webhooks:
            del self._webhooks[webhook_id]
            return True
        return False

    def list_webhooks(self) -> list[WebhookConfig]:
        return list(self._webhooks.values())

    async def dispatch(self, event: str, payload: dict) -> list[WebhookDelivery]:
        """Send event to all matching webhooks."""
        deliveries = []
        for wh in self._webhooks.values():
            if not wh.active:
                continue
            if event not in [e.value for e in wh.events]:
                continue
            delivery = await self._deliver(wh, event, payload)
            deliveries.append(delivery)
            self._deliveries.append(delivery)
        return deliveries

    async def _deliver(self, webhook: WebhookConfig, event: str, payload: dict) -> WebhookDelivery:
        body = json.dumps(
            {
                "event": event,
                "payload": payload,
                "timestamp": datetime.now(UTC).isoformat(),
            }
        )
        signature = self._sign(body, webhook.secret)
        delivery_id = uuid.uuid4().hex[:12]
        headers = {
            "Content-Type": "application/json",
            "X-AgentOS-Event": event,
            "X-AgentOS-Signature": signature,
            "X-AgentOS-Delivery": delivery_id,
        }

        delivery = WebhookDelivery(
            id=delivery_id,
            webhook_id=webhook.id,
            event=event,
        )

        for attempt in range(1, self._max_retries + 1):
            try:
                async with httpx.AsyncClient(timeout=10.0) as client:
                    resp = await client.post(webhook.url, content=body, headers=headers)
                    success = 200 <= resp.status_code < 300
                    delivery = WebhookDelivery(
                        id=delivery_id,
                        webhook_id=webhook.id,
                        event=event,
                        status_code=resp.status_code,
                        success=success,
                        attempt=attempt,
                    )
                    if success:
                        return delivery
            except httpx.HTTPError as e:
                delivery = WebhookDelivery(
                    id=delivery_id,
                    webhook_id=webhook.id,
                    event=event,
                    success=False,
                    attempt=attempt,
                    error=str(e),
                )

            # Exponential backoff before retry
            if attempt < self._max_retries:
                await asyncio.sleep(2**attempt)

        return delivery

    @staticmethod
    def _sign(body: str, secret: str) -> str:
        return hmac.new(secret.encode(), body.encode(), hashlib.sha256).hexdigest()

    @staticmethod
    def verify_signature(body: str, secret: str, signature: str) -> bool:
        expected = hmac.new(secret.encode(), body.encode(), hashlib.sha256).hexdigest()
        return hmac.compare_digest(expected, signature)

    def get_deliveries(
        self, webhook_id: str | None = None, limit: int = 50
    ) -> list[WebhookDelivery]:
        results = self._deliveries
        if webhook_id:
            results = [d for d in results if d.webhook_id == webhook_id]
        return results[-limit:]
