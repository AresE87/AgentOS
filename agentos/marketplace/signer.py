"""Playbook signing with Ed25519."""

from __future__ import annotations

import hashlib
import zipfile
from typing import TYPE_CHECKING

from cryptography.exceptions import InvalidSignature
from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from agentos.utils.logging import get_logger

if TYPE_CHECKING:
    from pathlib import Path

logger = get_logger("marketplace.signer")


class SignatureError(Exception):
    """Signature verification failed."""


class PlaybookSigner:
    """Signs and verifies .aosp playbook packages using Ed25519."""

    @staticmethod
    def generate_keypair() -> tuple[bytes, bytes]:
        """Generate an Ed25519 keypair.

        Returns:
            (private_key_bytes, public_key_bytes) in PEM format.
        """
        private_key = Ed25519PrivateKey.generate()
        private_bytes = private_key.private_bytes(
            encoding=serialization.Encoding.PEM,
            format=serialization.PrivateFormat.PKCS8,
            encryption_algorithm=serialization.NoEncryption(),
        )
        public_bytes = private_key.public_key().public_bytes(
            encoding=serialization.Encoding.PEM,
            format=serialization.PublicFormat.SubjectPublicKeyInfo,
        )
        return private_bytes, public_bytes

    def sign(self, aosp_path: Path, private_key_pem: bytes) -> None:
        """Sign an .aosp file. Adds signature.sig to the ZIP.

        Signs the SHA-256 hash of all files except signature.sig.
        """
        content_hash = self._compute_content_hash(aosp_path)

        private_key = serialization.load_pem_private_key(private_key_pem, password=None)
        signature = private_key.sign(content_hash)

        # Add signature to ZIP
        with zipfile.ZipFile(aosp_path, "a") as zf:
            zf.writestr("signature.sig", signature)

        logger.info("Signed %s", aosp_path.name)

    def verify(self, aosp_path: Path, public_key_pem: bytes) -> bool:
        """Verify an .aosp signature.

        Returns True if valid, raises SignatureError if invalid.
        """
        with zipfile.ZipFile(aosp_path, "r") as zf:
            if "signature.sig" not in zf.namelist():
                logger.warning("No signature found in %s", aosp_path.name)
                return False

            signature = zf.read("signature.sig")

        content_hash = self._compute_content_hash(aosp_path)

        public_key = serialization.load_pem_public_key(public_key_pem)
        try:
            public_key.verify(signature, content_hash)
        except InvalidSignature as err:
            raise SignatureError(f"Invalid signature for {aosp_path.name}") from err
        return True

    def _compute_content_hash(self, aosp_path: Path) -> bytes:
        """Compute SHA-256 hash of all files in ZIP except signature.sig."""
        h = hashlib.sha256()
        with zipfile.ZipFile(aosp_path, "r") as zf:
            for name in sorted(zf.namelist()):
                if name == "signature.sig":
                    continue
                h.update(name.encode())
                h.update(zf.read(name))
        return h.digest()
