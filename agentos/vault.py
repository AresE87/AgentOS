"""Encrypted vault for API keys and secrets.

Uses AES-256-GCM encryption with master key stored in OS keychain.
Fallback to PBKDF2-derived key from password when keychain unavailable.
"""

from __future__ import annotations

import base64
import json
import os
from pathlib import Path

from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.ciphers.aead import AESGCM
from cryptography.hazmat.primitives.kdf.pbkdf2 import PBKDF2HMAC

from agentos.utils.logging import get_logger

logger = get_logger("vault")

VAULT_VERSION = 1
SERVICE_NAME = "agentos"
KEY_NAME = "master_key"
PBKDF2_ITERATIONS = 100_000
SALT_SIZE = 16
IV_SIZE = 12  # 96 bits for AES-GCM
KEY_SIZE = 32  # 256 bits


class VaultError(Exception):
    """Error with vault operations."""


class Vault:
    """AES-256-GCM encrypted key-value store."""

    def __init__(
        self,
        vault_path: str | Path = "data/vault.enc",
        use_keyring: bool = True,
    ) -> None:
        self._path = Path(vault_path)
        self._use_keyring = use_keyring
        self._master_key: bytes | None = None
        self._data: dict[str, dict[str, str]] = {}

    async def initialize(self, password: str | None = None) -> None:
        """Initialize vault. Loads or creates master key."""
        self._path.parent.mkdir(parents=True, exist_ok=True)

        # Get or create master key
        self._master_key = self._get_or_create_master_key(password)

        # Load existing vault data
        if self._path.exists():
            with open(self._path) as f:
                vault_data = json.load(f)
            if vault_data.get("version") != VAULT_VERSION:
                raise VaultError(f"Unsupported vault version: {vault_data.get('version')}")
            self._data = vault_data.get("entries", {})

        logger.info("Vault initialized (%d entries)", len(self._data))

    async def store(self, key: str, value: str) -> None:
        """Encrypt and store a value."""
        if not self._master_key:
            raise VaultError("Vault not initialized")

        iv = os.urandom(IV_SIZE)
        aesgcm = AESGCM(self._master_key)
        ciphertext = aesgcm.encrypt(iv, value.encode(), key.encode())  # key as AAD

        # Split ciphertext and tag (last 16 bytes is tag)
        ct = ciphertext[:-16]
        tag = ciphertext[-16:]

        self._data[key] = {
            "iv": base64.b64encode(iv).decode(),
            "ciphertext": base64.b64encode(ct).decode(),
            "tag": base64.b64encode(tag).decode(),
        }

        self._save()
        logger.debug("Stored key: %s", key)

    async def retrieve(self, key: str) -> str | None:
        """Decrypt and retrieve a value."""
        if not self._master_key:
            raise VaultError("Vault not initialized")

        entry = self._data.get(key)
        if not entry:
            return None

        iv = base64.b64decode(entry["iv"])
        ct = base64.b64decode(entry["ciphertext"])
        tag = base64.b64decode(entry["tag"])

        aesgcm = AESGCM(self._master_key)
        plaintext = aesgcm.decrypt(iv, ct + tag, key.encode())
        return plaintext.decode()

    async def delete(self, key: str) -> None:
        """Delete a key from the vault."""
        if key in self._data:
            del self._data[key]
            self._save()

    async def list_keys(self) -> list[str]:
        """List all stored keys."""
        return list(self._data.keys())

    async def migrate_from_env(self, env_path: Path) -> int:
        """Import secrets from .env file into vault.

        Returns number of keys migrated.
        """
        if not env_path.exists():
            return 0

        count = 0
        secret_keys = {
            "ANTHROPIC_API_KEY",
            "OPENAI_API_KEY",
            "GOOGLE_API_KEY",
            "TELEGRAM_BOT_TOKEN",
        }

        with open(env_path) as f:
            for line in f:
                line = line.strip()
                if not line or line.startswith("#"):
                    continue
                if "=" not in line:
                    continue
                key, _, value = line.partition("=")
                key = key.strip()
                value = value.strip().strip("\"'")
                if key in secret_keys and value:
                    await self.store(key, value)
                    count += 1

        logger.info("Migrated %d keys from %s", count, env_path)
        return count

    def _get_or_create_master_key(self, password: str | None = None) -> bytes:
        """Get master key from keyring or derive from password."""
        if self._use_keyring:
            try:
                import keyring as kr

                stored = kr.get_password(SERVICE_NAME, KEY_NAME)
                if stored:
                    return bytes.fromhex(stored)
                # Generate new key
                new_key = os.urandom(KEY_SIZE)
                kr.set_password(SERVICE_NAME, KEY_NAME, new_key.hex())
                logger.info("Generated new master key in OS keychain")
                return new_key
            except Exception:
                logger.warning("Keychain unavailable, falling back to password")

        # Fallback: password-derived key
        if password:
            salt_path = self._path.with_suffix(".salt")
            if salt_path.exists():
                salt = salt_path.read_bytes()
            else:
                salt = os.urandom(SALT_SIZE)
                salt_path.parent.mkdir(parents=True, exist_ok=True)
                salt_path.write_bytes(salt)

            kdf = PBKDF2HMAC(
                algorithm=hashes.SHA256(),
                length=KEY_SIZE,
                salt=salt,
                iterations=PBKDF2_ITERATIONS,
            )
            return kdf.derive(password.encode())

        # Last resort: generate ephemeral key (for testing)
        logger.warning(
            "No keychain or password — using ephemeral key (data won't persist across restarts)"
        )
        return os.urandom(KEY_SIZE)

    def _save(self) -> None:
        """Save vault to disk."""
        vault_data = {"version": VAULT_VERSION, "entries": self._data}
        self._path.parent.mkdir(parents=True, exist_ok=True)
        with open(self._path, "w") as f:
            json.dump(vault_data, f, indent=2)
