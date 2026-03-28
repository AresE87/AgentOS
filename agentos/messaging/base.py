"""Base interface for messaging adapters.

All messaging channels (Telegram, Discord, WhatsApp, etc.) implement
BaseMessagingAdapter so the agent core can treat them uniformly.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Awaitable, Callable

    from agentos.types import TaskInput, TaskResult


class BaseMessagingAdapter(ABC):
    """Base interface for all messaging channels."""

    def __init__(self, on_message: Callable[[TaskInput], Awaitable[TaskResult]]) -> None:
        self._on_message = on_message

    @abstractmethod
    async def start(self) -> None: ...

    @abstractmethod
    async def stop(self) -> None: ...

    @abstractmethod
    async def send_message(self, chat_id: str, text: str) -> None: ...
