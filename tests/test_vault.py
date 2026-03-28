"""Tests for the encrypted vault (AOS-043)."""

from __future__ import annotations

import logging
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from pathlib import Path

from agentos.vault import Vault


@pytest.fixture
def vault_path(tmp_path: Path) -> Path:
    """Return a temporary vault file path."""
    return tmp_path / "vault.enc"


@pytest.mark.asyncio
async def test_store_and_retrieve(vault_path: Path) -> None:
    """Store a value and retrieve it back."""
    vault = Vault(vault_path=vault_path, use_keyring=False)
    await vault.initialize(password="test-password")

    await vault.store("my_key", "my_secret_value")
    result = await vault.retrieve("my_key")
    assert result == "my_secret_value"


@pytest.mark.asyncio
async def test_retrieve_nonexistent(vault_path: Path) -> None:
    """Retrieving a missing key returns None."""
    vault = Vault(vault_path=vault_path, use_keyring=False)
    await vault.initialize(password="test-password")

    result = await vault.retrieve("nonexistent")
    assert result is None


@pytest.mark.asyncio
async def test_delete_key(vault_path: Path) -> None:
    """Store then delete then retrieve returns None."""
    vault = Vault(vault_path=vault_path, use_keyring=False)
    await vault.initialize(password="test-password")

    await vault.store("delete_me", "value")
    await vault.delete("delete_me")
    result = await vault.retrieve("delete_me")
    assert result is None


@pytest.mark.asyncio
async def test_list_keys(vault_path: Path) -> None:
    """Storing 3 keys makes list_keys return 3 items."""
    vault = Vault(vault_path=vault_path, use_keyring=False)
    await vault.initialize(password="test-password")

    await vault.store("key1", "val1")
    await vault.store("key2", "val2")
    await vault.store("key3", "val3")

    keys = await vault.list_keys()
    assert sorted(keys) == ["key1", "key2", "key3"]


@pytest.mark.asyncio
async def test_migrate_from_env(vault_path: Path, tmp_path: Path) -> None:
    """Migrate secrets from a .env file into the vault."""
    env_path = tmp_path / ".env"
    env_path.write_text(
        "# Comment line\n"
        'ANTHROPIC_API_KEY="sk-ant-test-key"\n'
        "OPENAI_API_KEY=sk-test-openai\n"
        "SOME_OTHER_VAR=not-a-secret\n"
        "TELEGRAM_BOT_TOKEN='123456:ABC-TOKEN'\n"
    )

    vault = Vault(vault_path=vault_path, use_keyring=False)
    await vault.initialize(password="test-password")

    count = await vault.migrate_from_env(env_path)
    assert count == 3

    assert await vault.retrieve("ANTHROPIC_API_KEY") == "sk-ant-test-key"
    assert await vault.retrieve("OPENAI_API_KEY") == "sk-test-openai"
    assert await vault.retrieve("TELEGRAM_BOT_TOKEN") == "123456:ABC-TOKEN"
    assert await vault.retrieve("SOME_OTHER_VAR") is None


@pytest.mark.asyncio
async def test_vault_persistence(vault_path: Path) -> None:
    """Store, then re-initialize from disk and retrieve."""
    vault1 = Vault(vault_path=vault_path, use_keyring=False)
    await vault1.initialize(password="persist-password")
    await vault1.store("persist_key", "persist_value")

    # Create a new vault instance pointing to the same file
    vault2 = Vault(vault_path=vault_path, use_keyring=False)
    await vault2.initialize(password="persist-password")

    result = await vault2.retrieve("persist_key")
    assert result == "persist_value"


@pytest.mark.asyncio
async def test_password_derived_key(vault_path: Path) -> None:
    """Initialize with password (no keyring) works for encrypt/decrypt."""
    vault = Vault(vault_path=vault_path, use_keyring=False)
    await vault.initialize(password="strong-password-123")

    await vault.store("pwd_key", "pwd_value")
    assert await vault.retrieve("pwd_key") == "pwd_value"

    # Verify salt file was created
    salt_path = vault_path.with_suffix(".salt")
    assert salt_path.exists()
    assert len(salt_path.read_bytes()) == 16


@pytest.mark.asyncio
async def test_ephemeral_key_warning(vault_path: Path, caplog: pytest.LogCaptureFixture) -> None:
    """No keyring and no password logs a warning about ephemeral key."""
    vault = Vault(vault_path=vault_path, use_keyring=False)

    with caplog.at_level(logging.WARNING, logger="agentos.vault"):
        await vault.initialize(password=None)

    assert any("ephemeral" in msg.lower() for msg in caplog.messages)

    # Should still work for the current session
    await vault.store("ephemeral_key", "ephemeral_value")
    assert await vault.retrieve("ephemeral_key") == "ephemeral_value"
