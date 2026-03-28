"""Offline mode detection and local LLM fallback."""

from __future__ import annotations

import asyncio
import contextlib
from typing import TYPE_CHECKING

import httpx

if TYPE_CHECKING:
    from agentos.gateway.local_provider import LocalLLMProvider

from agentos.utils.logging import get_logger

logger = get_logger("gateway.offline")

CONNECTIVITY_URLS = [
    "https://api.anthropic.com",
    "https://api.openai.com",
    "https://generativelanguage.googleapis.com",
]
CHECK_TIMEOUT = 5.0


class OfflineDetector:
    """Detects internet connectivity and manages offline mode."""

    def __init__(
        self,
        local_provider: LocalLLMProvider | None = None,
        check_interval: float = 60.0,
    ) -> None:
        self._local = local_provider
        self._check_interval = check_interval
        self._is_online = True
        self._prefer_local = False
        self._monitor_task: asyncio.Task | None = None  # type: ignore[type-arg]

    @property
    def is_online(self) -> bool:
        return self._is_online

    @property
    def prefer_local(self) -> bool:
        return self._prefer_local

    @prefer_local.setter
    def prefer_local(self, value: bool) -> None:
        self._prefer_local = value

    @property
    def has_local(self) -> bool:
        return self._local is not None and self._local.is_available

    @property
    def should_use_local(self) -> bool:
        """True if we should use local models (offline or user preference)."""
        if self._prefer_local and self.has_local:
            return True
        return bool(not self._is_online and self.has_local)

    async def check_connectivity(self) -> bool:
        """Check if any cloud provider is reachable."""
        async with httpx.AsyncClient(timeout=CHECK_TIMEOUT) as client:
            for url in CONNECTIVITY_URLS:
                try:
                    resp = await client.head(url)
                    if resp.status_code < 500:
                        self._is_online = True
                        return True
                except httpx.HTTPError:
                    continue
        self._is_online = False
        logger.warning("No internet connectivity detected")
        return False

    async def start_monitoring(self) -> None:
        """Start periodic connectivity checks."""
        self._monitor_task = asyncio.create_task(self._monitor_loop())

    async def stop_monitoring(self) -> None:
        if self._monitor_task:
            self._monitor_task.cancel()
            with contextlib.suppress(asyncio.CancelledError):
                await self._monitor_task

    async def _monitor_loop(self) -> None:
        while True:
            was_online = self._is_online
            await self.check_connectivity()

            if was_online and not self._is_online:
                logger.warning("Gone offline — switching to local models")
            elif not was_online and self._is_online:
                logger.info("Back online — cloud providers available")

            if self._local and not self._local.is_available:
                await self._local.health_check()

            await asyncio.sleep(self._check_interval)
