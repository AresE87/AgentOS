"""Node identity — X25519 keypair for mesh authentication."""

from __future__ import annotations

import hashlib
from dataclasses import dataclass, field

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.x25519 import X25519PrivateKey, X25519PublicKey

from agentos.utils.logging import get_logger

logger = get_logger("mesh.identity")


@dataclass(frozen=True)
class NodeCapabilities:
    os_type: str
    has_gpu: bool = False
    specialists: list[str] = field(default_factory=list)
    cpu_cores: int = 1
    memory_gb: float = 0.0


@dataclass
class NodeProfile:
    node_id: str  # Short hash of public key, e.g. "node-a3f7b2"
    display_name: str
    public_key: bytes  # PEM encoded
    capabilities: NodeCapabilities | None
    address: str = ""  # ip:port
    is_online: bool = False


class NodeIdentity:
    """Manages this node's cryptographic identity."""

    def __init__(self) -> None:
        self._private_key: X25519PrivateKey | None = None
        self._public_key: X25519PublicKey | None = None
        self._node_id: str = ""
        self._display_name: str = "My PC"
        self._capabilities = NodeCapabilities(os_type="unknown")

    def generate_keypair(self) -> None:
        """Generate new X25519 keypair."""
        self._private_key = X25519PrivateKey.generate()
        self._public_key = self._private_key.public_key()
        pub_bytes = self._public_key.public_bytes(
            encoding=serialization.Encoding.Raw,
            format=serialization.PublicFormat.Raw,
        )
        self._node_id = "node-" + hashlib.sha256(pub_bytes).hexdigest()[:6]
        logger.info("Generated node identity: %s", self._node_id)

    @property
    def node_id(self) -> str:
        return self._node_id

    @property
    def display_name(self) -> str:
        return self._display_name

    @display_name.setter
    def display_name(self, value: str) -> None:
        self._display_name = value

    @property
    def capabilities(self) -> NodeCapabilities:
        return self._capabilities

    @capabilities.setter
    def capabilities(self, value: NodeCapabilities) -> None:
        self._capabilities = value

    def get_public_key_bytes(self) -> bytes:
        if not self._public_key:
            raise RuntimeError("Keypair not generated")
        return self._public_key.public_bytes(
            encoding=serialization.Encoding.Raw,
            format=serialization.PublicFormat.Raw,
        )

    def get_public_key_pem(self) -> bytes:
        if not self._public_key:
            raise RuntimeError("Keypair not generated")
        return self._public_key.public_bytes(
            encoding=serialization.Encoding.PEM,
            format=serialization.PublicFormat.SubjectPublicKeyInfo,
        )

    def compute_shared_secret(self, peer_public_key_bytes: bytes) -> bytes:
        """X25519 ECDH key exchange."""
        if not self._private_key:
            raise RuntimeError("Keypair not generated")
        peer_key = X25519PublicKey.from_public_bytes(peer_public_key_bytes)
        shared = self._private_key.exchange(peer_key)
        # Derive AES key from shared secret
        return hashlib.sha256(shared).digest()  # 32 bytes = AES-256

    def get_profile(self) -> NodeProfile:
        return NodeProfile(
            node_id=self._node_id,
            display_name=self._display_name,
            public_key=self.get_public_key_pem(),
            capabilities=self._capabilities,
        )

    def export_private_key(self) -> bytes:
        """Export private key PEM for vault storage."""
        if not self._private_key:
            raise RuntimeError("Keypair not generated")
        return self._private_key.private_bytes(
            encoding=serialization.Encoding.PEM,
            format=serialization.PrivateFormat.PKCS8,
            encryption_algorithm=serialization.NoEncryption(),
        )

    def import_private_key(self, pem_data: bytes) -> None:
        """Import private key from vault."""
        self._private_key = serialization.load_pem_private_key(pem_data, password=None)  # type: ignore[assignment]
        self._public_key = self._private_key.public_key()  # type: ignore[union-attr]
        pub_bytes = self._public_key.public_bytes(
            encoding=serialization.Encoding.Raw,
            format=serialization.PublicFormat.Raw,
        )
        self._node_id = "node-" + hashlib.sha256(pub_bytes).hexdigest()[:6]
