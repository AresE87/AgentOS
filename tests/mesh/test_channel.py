"""Tests for AOS-063 Secure Channel."""

from __future__ import annotations

import asyncio
import json
from unittest.mock import AsyncMock

import pytest
from cryptography.exceptions import InvalidTag

from agentos.mesh.channel import SecureChannel
from agentos.mesh.identity import NodeIdentity


def _make_paired_channels() -> tuple[SecureChannel, SecureChannel]:
    """Create two channels with a shared secret established."""
    alice = NodeIdentity()
    alice.generate_keypair()
    bob = NodeIdentity()
    bob.generate_keypair()

    ch_a = SecureChannel(alice)
    ch_a._shared_key = alice.compute_shared_secret(bob.get_public_key_bytes())
    ch_a._connected = True

    ch_b = SecureChannel(bob)
    ch_b._shared_key = bob.compute_shared_secret(alice.get_public_key_bytes())
    ch_b._connected = True

    return ch_a, ch_b


def test_encrypt_decrypt_roundtrip() -> None:
    """Encrypt then decrypt returns the original data."""
    ch_a, ch_b = _make_paired_channels()
    plaintext = b"hello mesh network"
    ciphertext = ch_a.encrypt(plaintext)
    result = ch_b.decrypt(ciphertext)
    assert result == plaintext


def test_different_iv_each_time() -> None:
    """Two encryptions of the same data produce different ciphertext."""
    ch_a, _ = _make_paired_channels()
    plaintext = b"same data"
    ct1 = ch_a.encrypt(plaintext)
    ct2 = ch_a.encrypt(plaintext)
    assert ct1 != ct2


def test_wrong_key_fails() -> None:
    """Decrypting with the wrong key raises an error."""
    ch_a, _ = _make_paired_channels()

    # Create a channel with a different key
    eve = NodeIdentity()
    eve.generate_keypair()
    ch_eve = SecureChannel(eve)
    ch_eve._shared_key = b"\x00" * 32  # wrong key
    ch_eve._connected = True

    ciphertext = ch_a.encrypt(b"secret message")
    with pytest.raises(InvalidTag):
        ch_eve.decrypt(ciphertext)


async def test_send_receive_mock() -> None:
    """Send and receive JSON through a mocked websocket."""
    ch_a, ch_b = _make_paired_channels()

    # Wire up a mock websocket: what ch_a sends, ch_b receives
    sent_data: list[bytes] = []
    mock_ws_a = AsyncMock()
    mock_ws_a.send = AsyncMock(side_effect=lambda data: sent_data.append(data))
    ch_a._ws = mock_ws_a

    mock_ws_b = AsyncMock()
    ch_b._ws = mock_ws_b

    # Send from A
    msg = {"type": "task", "payload": "do something"}
    await ch_a.send(msg)

    # B receives the encrypted bytes
    assert len(sent_data) == 1
    mock_ws_b.recv = AsyncMock(return_value=sent_data[0])

    received = await ch_b.receive()
    assert received == msg


async def test_heartbeat_sends_message() -> None:
    """Starting heartbeat sends at least one heartbeat message."""
    ch_a, _ = _make_paired_channels()

    sent_messages: list[bytes] = []
    mock_ws = AsyncMock()
    mock_ws.send = AsyncMock(side_effect=lambda data: sent_messages.append(data))
    ch_a._ws = mock_ws

    # Override heartbeat interval to be fast
    import agentos.mesh.channel as channel_mod

    original_interval = channel_mod.HEARTBEAT_INTERVAL
    channel_mod.HEARTBEAT_INTERVAL = 0.05

    try:
        await ch_a.start_heartbeat()
        # Wait enough for at least one heartbeat
        await asyncio.sleep(0.15)
        await ch_a.close()

        assert len(sent_messages) >= 1
        # Decrypt first heartbeat to verify format
        _, ch_b = _make_paired_channels()
        # Use ch_a's key to decrypt
        ch_b._shared_key = ch_a._shared_key
        plaintext = ch_b.decrypt(sent_messages[0])
        hb = json.loads(plaintext)
        assert hb["type"] == "heartbeat"
        assert "timestamp" in hb
    finally:
        channel_mod.HEARTBEAT_INTERVAL = original_interval
