"""Tests for AOS-061 Node Identity."""

from __future__ import annotations

from agentos.mesh.identity import NodeCapabilities, NodeIdentity, NodeProfile


def test_generate_keypair() -> None:
    """node_id starts with 'node-' after keypair generation."""
    identity = NodeIdentity()
    identity.generate_keypair()
    assert identity.node_id.startswith("node-")
    assert len(identity.node_id) == len("node-") + 6


def test_node_id_deterministic() -> None:
    """Same key produces the same node_id."""
    identity = NodeIdentity()
    identity.generate_keypair()
    pem = identity.export_private_key()

    restored = NodeIdentity()
    restored.import_private_key(pem)
    assert restored.node_id == identity.node_id


def test_shared_secret() -> None:
    """Two identities derive the same shared secret via ECDH."""
    alice = NodeIdentity()
    alice.generate_keypair()

    bob = NodeIdentity()
    bob.generate_keypair()

    secret_ab = alice.compute_shared_secret(bob.get_public_key_bytes())
    secret_ba = bob.compute_shared_secret(alice.get_public_key_bytes())
    assert secret_ab == secret_ba
    assert len(secret_ab) == 32  # AES-256


def test_export_import_roundtrip() -> None:
    """Export PEM then import produces the same node_id and public key."""
    identity = NodeIdentity()
    identity.generate_keypair()
    original_id = identity.node_id
    original_pub = identity.get_public_key_bytes()
    pem = identity.export_private_key()

    restored = NodeIdentity()
    restored.import_private_key(pem)
    assert restored.node_id == original_id
    assert restored.get_public_key_bytes() == original_pub


def test_get_profile() -> None:
    """get_profile returns a NodeProfile with all fields populated."""
    identity = NodeIdentity()
    identity.generate_keypair()
    identity.display_name = "Test Node"
    identity.capabilities = NodeCapabilities(os_type="linux", has_gpu=True, cpu_cores=8)

    profile = identity.get_profile()
    assert isinstance(profile, NodeProfile)
    assert profile.node_id == identity.node_id
    assert profile.display_name == "Test Node"
    assert profile.public_key  # non-empty PEM bytes
    assert profile.capabilities is not None
    assert profile.capabilities.os_type == "linux"
    assert profile.capabilities.has_gpu is True
    assert profile.capabilities.cpu_cores == 8
