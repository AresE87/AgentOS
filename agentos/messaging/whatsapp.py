"""WhatsApp Business Cloud API adapter."""

from __future__ import annotations

from typing import TYPE_CHECKING

import httpx

from agentos.messaging.base import BaseMessagingAdapter
from agentos.types import TaskInput, TaskResult, TaskStatus
from agentos.utils.logging import get_logger

if TYPE_CHECKING:
    from collections.abc import Awaitable, Callable

logger = get_logger("messaging.whatsapp")

WA_API_BASE = "https://graph.facebook.com/v18.0"


class WhatsAppAdapter(BaseMessagingAdapter):
    """WhatsApp Business Cloud API adapter."""

    def __init__(
        self,
        phone_number_id: str,
        access_token: str,
        verify_token: str,
        on_message: Callable[[TaskInput], Awaitable[TaskResult]],
    ) -> None:
        super().__init__(on_message)
        self._phone_id = phone_number_id
        self._token = access_token
        self._verify_token = verify_token
        self._client: httpx.AsyncClient | None = None
        self._running = False

    # -- Lifecycle -------------------------------------------------------------

    async def start(self) -> None:
        """Start the WhatsApp adapter (initialise HTTP client)."""
        self._client = httpx.AsyncClient(
            base_url=f"{WA_API_BASE}/{self._phone_id}",
            headers={"Authorization": f"Bearer {self._token}"},
            timeout=30.0,
        )
        self._running = True
        logger.info("WhatsApp adapter started")

    async def stop(self) -> None:
        """Stop the WhatsApp adapter and close the HTTP client."""
        self._running = False
        if self._client:
            await self._client.aclose()
        logger.info("WhatsApp adapter stopped")

    # -- Sending ---------------------------------------------------------------

    async def send_message(self, chat_id: str, text: str) -> None:
        """Send a text message to a WhatsApp user."""
        if not self._client:
            return
        for chunk in self._split_message(text):
            payload = {
                "messaging_product": "whatsapp",
                "to": chat_id,
                "type": "text",
                "text": {"body": chunk},
            }
            try:
                await self._client.post("/messages", json=payload)
            except httpx.HTTPError:
                logger.exception("Failed to send WhatsApp message to %s", chat_id)

    # -- Webhook handling ------------------------------------------------------

    async def handle_webhook(self, body: dict) -> None:
        """Process an incoming webhook payload from WhatsApp Cloud API."""
        for entry in body.get("entry", []):
            for change in entry.get("changes", []):
                value = change.get("value", {})
                for msg in value.get("messages", []):
                    if msg.get("type") == "text":
                        sender = msg["from"]
                        text = msg["text"]["body"]
                        await self._process_message(sender, text)

    def verify_webhook(self, mode: str, token: str, challenge: str) -> str | None:
        """Verify webhook subscription from Meta.

        Returns the challenge string on success, ``None`` on failure.
        """
        if mode == "subscribe" and token == self._verify_token:
            return challenge
        return None

    # -- Internal --------------------------------------------------------------

    async def _process_message(self, sender: str, text: str) -> None:
        if not text.strip():
            await self.send_message(sender, "Send me a message and I'll help!")
            return

        task_input = TaskInput(text=text, source="whatsapp", chat_id=sender)
        logger.info("WhatsApp message from %s, task_id=%s", sender, task_input.task_id)

        try:
            result = await self._on_message(task_input)
            response_text = self._format_result(result)
            await self.send_message(sender, response_text)
        except Exception:
            logger.exception("Error processing WhatsApp message")
            await self.send_message(sender, "An error occurred. Please try again.")

    @staticmethod
    def _format_result(result: TaskResult) -> str:
        if result.status == TaskStatus.FAILED:
            return f"Error: {result.error_message or 'Unknown error'}"
        parts = [result.output_text or "Done."]
        if result.model_used:
            parts.append(f"\n_Model: {result.model_used} · Cost: ${result.cost_estimate:.4f}_")
        return "\n".join(parts)

    @staticmethod
    def _split_message(text: str, max_length: int = 4096) -> list[str]:
        """Split *text* into WhatsApp-safe chunks (limit 4096 chars)."""
        if len(text) <= max_length:
            return [text]
        chunks: list[str] = []
        while text:
            if len(text) <= max_length:
                chunks.append(text)
                break
            split = text.rfind("\n", 0, max_length)
            if split == -1:
                split = max_length
            chunks.append(text[:split])
            text = text[split:].lstrip()
        return chunks
