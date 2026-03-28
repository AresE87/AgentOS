"""Tests for OfflineDetector (AOS-054)."""

from __future__ import annotations

from unittest.mock import AsyncMock, MagicMock, patch

import httpx
import pytest

from agentos.gateway.local_provider import LocalLLMProvider
from agentos.gateway.offline import OfflineDetector


def _make_local(available: bool) -> LocalLLMProvider:
    """Create a mock-like LocalLLMProvider with controlled availability."""
    provider = MagicMock(spec=LocalLLMProvider)
    provider.is_available = available
    return provider


# ── check_connectivity ────────────────────────────────────────────────


@pytest.mark.asyncio
async def test_check_connectivity_online() -> None:
    detector = OfflineDetector()
    mock_response = httpx.Response(200)

    with patch("agentos.gateway.offline.httpx.AsyncClient") as mock_cls:
        mock_client = AsyncMock()
        mock_client.head = AsyncMock(return_value=mock_response)
        mock_client.__aenter__ = AsyncMock(return_value=mock_client)
        mock_client.__aexit__ = AsyncMock(return_value=False)
        mock_cls.return_value = mock_client

        result = await detector.check_connectivity()

    assert result is True
    assert detector.is_online is True


@pytest.mark.asyncio
async def test_check_connectivity_offline() -> None:
    detector = OfflineDetector()

    with patch("agentos.gateway.offline.httpx.AsyncClient") as mock_cls:
        mock_client = AsyncMock()
        mock_client.head = AsyncMock(side_effect=httpx.ConnectError("no network"))
        mock_client.__aenter__ = AsyncMock(return_value=mock_client)
        mock_client.__aexit__ = AsyncMock(return_value=False)
        mock_cls.return_value = mock_client

        result = await detector.check_connectivity()

    assert result is False
    assert detector.is_online is False


# ── should_use_local ──────────────────────────────────────────────────


@pytest.mark.asyncio
async def test_should_use_local_when_offline() -> None:
    local = _make_local(available=True)
    detector = OfflineDetector(local_provider=local)
    detector._is_online = False

    assert detector.should_use_local is True


@pytest.mark.asyncio
async def test_should_use_local_when_preferred() -> None:
    local = _make_local(available=True)
    detector = OfflineDetector(local_provider=local)
    detector.prefer_local = True

    assert detector.should_use_local is True


@pytest.mark.asyncio
async def test_should_not_use_local_online() -> None:
    local = _make_local(available=True)
    detector = OfflineDetector(local_provider=local)
    # Default: online, no preference
    assert detector.should_use_local is False


@pytest.mark.asyncio
async def test_no_local_provider() -> None:
    detector = OfflineDetector(local_provider=None)
    detector._is_online = False

    assert detector.has_local is False
    assert detector.should_use_local is False
