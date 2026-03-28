"""Secure WebSocket channel with E2E encryption."""

from __future__ import annotations

import asyncio
import json
import os
import time
from typing import TYPE_CHECKING

from cryptography.hazmat.primitives.ciphers.aead import AESGCM

from agentos.utils.logging import get_logger

if TYPE_CHECKING:
    from agentos.mesh.identity import NodeIdentity

logger = get_logger("mesh.channel")

HEARTBEAT_INTERVAL = 30.0
IV_SIZE = 12


class SecureChannel:
    """E2E encrypted WebSocket channel between two nodes."""

    def __init__(self, identity: NodeIdentity) -> None:
        self._identity = identity
        self._shared_key: bytes | None = None
        self._ws: object | None = None
        self._peer_node_id: str = ""
        self._connected = False
        self._heartbeat_task: asyncio.Task[None] | None = None
        self._last_heartbeat: float = 0

    async def connect(self, uri: str, peer_public_key: bytes) -> None:
        """Connect to a peer and establish encrypted channel."""
        import websockets

        self._shared_key = self._identity.compute_shared_secret(peer_public_key)
        self._ws = await websockets.connect(uri)
        self._connected = True
        # Send our public key for mutual auth
        await self._ws.send(self._identity.get_public_key_bytes())  # type: ignore[union-attr]
        logger.info("Secure channel established with %s", uri)

    async def accept(self, websocket: object, peer_public_key: bytes) -> None:
        """Accept an incoming connection."""
        self._shared_key = self._identity.compute_shared_secret(peer_public_key)
        self._ws = websocket
        self._connected = True

    def encrypt(self, data: bytes) -> bytes:
        """Encrypt with AES-256-GCM."""
        if not self._shared_key:
            raise RuntimeError("Channel not established")
        iv = os.urandom(IV_SIZE)
        aesgcm = AESGCM(self._shared_key)
        ct = aesgcm.encrypt(iv, data, None)
        return iv + ct

    def decrypt(self, data: bytes) -> bytes:
        """Decrypt AES-256-GCM."""
        if not self._shared_key:
            raise RuntimeError("Channel not established")
        iv = data[:IV_SIZE]
        ct = data[IV_SIZE:]
        aesgcm = AESGCM(self._shared_key)
        return aesgcm.decrypt(iv, ct, None)

    async def send(self, message: dict) -> None:
        """Send encrypted JSON message."""
        if not self._ws or not self._connected:
            raise RuntimeError("Not connected")
        plaintext = json.dumps(message).encode()
        encrypted = self.encrypt(plaintext)
        await self._ws.send(encrypted)  # type: ignore[union-attr]

    async def receive(self) -> dict:
        """Receive and decrypt JSON message."""
        if not self._ws or not self._connected:
            raise RuntimeError("Not connected")
        data = await self._ws.recv()  # type: ignore[union-attr]
        if isinstance(data, str):
            data = data.encode()
        plaintext = self.decrypt(data)
        return json.loads(plaintext)

    async def start_heartbeat(self) -> None:
        self._heartbeat_task = asyncio.create_task(self._heartbeat_loop())

    async def _heartbeat_loop(self) -> None:
        while self._connected:
            try:
                await self.send({"type": "heartbeat", "timestamp": time.time()})
                self._last_heartbeat = time.time()
            except Exception:
                logger.warning("Heartbeat failed")
                self._connected = False
                break
            await asyncio.sleep(HEARTBEAT_INTERVAL)

    async def close(self) -> None:
        self._connected = False
        if self._heartbeat_task:
            self._heartbeat_task.cancel()
        if self._ws:
            await self._ws.close()  # type: ignore[union-attr]

    @property
    def is_connected(self) -> bool:
        return self._connected
