"""Tests for playbook signing (AOS-042)."""

from __future__ import annotations

import zipfile
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from pathlib import Path

from agentos.marketplace.signer import PlaybookSigner, SignatureError


def _create_aosp(path: Path, files: dict[str, str] | None = None) -> Path:
    """Create a minimal .aosp (ZIP) package for testing."""
    if files is None:
        files = {
            "manifest.yaml": "name: test-playbook\nversion: 1.0.0\n",
            "main.py": 'print("hello")\n',
        }
    aosp = path / "test.aosp"
    with zipfile.ZipFile(aosp, "w") as zf:
        for name, content in files.items():
            zf.writestr(name, content)
    return aosp


class TestPlaybookSigner:
    """Tests for PlaybookSigner."""

    def test_generate_keypair(self) -> None:
        """generate_keypair returns (private, public) PEM bytes."""
        private, public = PlaybookSigner.generate_keypair()
        assert isinstance(private, bytes)
        assert isinstance(public, bytes)
        assert b"BEGIN PRIVATE KEY" in private
        assert b"BEGIN PUBLIC KEY" in public

    def test_sign_creates_signature_file(self, tmp_path: Path) -> None:
        """After signing, the ZIP contains signature.sig."""
        aosp = _create_aosp(tmp_path)
        private, _ = PlaybookSigner.generate_keypair()

        signer = PlaybookSigner()
        signer.sign(aosp, private)

        with zipfile.ZipFile(aosp, "r") as zf:
            assert "signature.sig" in zf.namelist()

    def test_verify_valid_signature(self, tmp_path: Path) -> None:
        """Sign then verify returns True."""
        aosp = _create_aosp(tmp_path)
        private, public = PlaybookSigner.generate_keypair()

        signer = PlaybookSigner()
        signer.sign(aosp, private)
        assert signer.verify(aosp, public) is True

    def test_verify_invalid_signature(self, tmp_path: Path) -> None:
        """Modifying file content after signing raises SignatureError."""
        aosp = _create_aosp(tmp_path)
        private, public = PlaybookSigner.generate_keypair()

        signer = PlaybookSigner()
        signer.sign(aosp, private)

        # Tamper with the archive: add a new file
        with zipfile.ZipFile(aosp, "a") as zf:
            zf.writestr("evil.py", "import os; os.system('rm -rf /')\n")

        with pytest.raises(SignatureError):
            signer.verify(aosp, public)

    def test_verify_no_signature(self, tmp_path: Path) -> None:
        """ZIP without signature.sig returns False."""
        aosp = _create_aosp(tmp_path)
        _, public = PlaybookSigner.generate_keypair()

        signer = PlaybookSigner()
        assert signer.verify(aosp, public) is False

    def test_roundtrip(self, tmp_path: Path) -> None:
        """Full roundtrip: generate_keypair -> sign -> verify -> success."""
        files = {
            "manifest.yaml": "name: my-playbook\nversion: 2.0.0\n",
            "steps/step1.py": "print('step 1')\n",
            "steps/step2.py": "print('step 2')\n",
            "README.md": "# My Playbook\n",
        }
        aosp = _create_aosp(tmp_path, files)

        private, public = PlaybookSigner.generate_keypair()
        signer = PlaybookSigner()

        signer.sign(aosp, private)
        assert signer.verify(aosp, public) is True
