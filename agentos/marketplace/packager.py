"""Playbook packaging -- creates and extracts .aosp archives."""

from __future__ import annotations

import hashlib
import zipfile
from pathlib import Path

import yaml

from agentos.utils.logging import get_logger

logger = get_logger("marketplace.packager")

EXCLUDED_PATHS = {"credentials.vault", "state", "__pycache__", ".git", ".env"}
REQUIRED_FILES = {"playbook.md", "config.yaml"}
METADATA_REQUIRED_FIELDS = ["name", "version", "author", "description"]


class PackagingError(Exception):
    """Error during packaging/unpackaging."""


class PlaybookPackager:
    """Creates and extracts .aosp playbook packages."""

    async def pack(self, folder_path: Path, output_path: Path | None = None) -> Path:
        """Package a Context Folder into .aosp file.

        1. Validate folder has required files
        2. Generate/validate metadata.yaml (create from config.yaml if missing)
        3. Collect files (excluding prohibited paths)
        4. Generate checksum.sha256
        5. Create ZIP
        """
        folder = Path(folder_path)
        if not folder.is_dir():
            raise PackagingError(f"Not a directory: {folder}")

        # Check required files
        for req in REQUIRED_FILES:
            if not (folder / req).exists():
                raise PackagingError(f"Missing required file: {req}")

        # Load/create metadata
        metadata = self._load_or_create_metadata(folder)
        errors = self.validate_metadata(metadata)
        if errors:
            raise PackagingError(f"Invalid metadata: {'; '.join(errors)}")

        # Determine output path
        if output_path is None:
            name = metadata.get("name", folder.name).replace(" ", "_").lower()
            version = metadata.get("version", "1.0.0")
            output_path = folder.parent / f"{name}-{version}.aosp"

        # Collect files
        files_to_pack = self._collect_files(folder)

        # Generate checksums
        checksums: dict[str, str] = {}
        for rel_path, abs_path in files_to_pack:
            checksums[rel_path] = self._file_hash(abs_path)

        # Write ZIP
        with zipfile.ZipFile(output_path, "w", zipfile.ZIP_DEFLATED) as zf:
            # Write metadata
            zf.writestr("metadata.yaml", yaml.dump(metadata, default_flow_style=False))

            # Write all collected files
            for rel_path, abs_path in files_to_pack:
                zf.write(abs_path, rel_path)

            # Write checksums
            checksum_content = "\n".join(f"{h}  {p}" for p, h in sorted(checksums.items()))
            zf.writestr("checksum.sha256", checksum_content)

        logger.info("Packed %s -> %s (%d files)", folder.name, output_path.name, len(files_to_pack))
        return output_path

    async def unpack(self, aosp_path: Path, target_dir: Path) -> Path:
        """Unpack .aosp to target directory. Verifies checksums."""
        if not aosp_path.exists():
            raise PackagingError(f"File not found: {aosp_path}")

        target_dir.mkdir(parents=True, exist_ok=True)

        with zipfile.ZipFile(aosp_path, "r") as zf:
            # Verify checksums if present
            if "checksum.sha256" in zf.namelist():
                self._verify_checksums(zf)

            # Extract all
            zf.extractall(target_dir)

        logger.info("Unpacked %s -> %s", aosp_path.name, target_dir)
        return target_dir

    def validate_metadata(self, metadata: dict) -> list[str]:
        """Validate metadata.yaml. Returns list of error strings."""
        errors: list[str] = []
        for field in METADATA_REQUIRED_FIELDS:
            if not metadata.get(field):
                errors.append(f"Missing required field: {field}")

        version = metadata.get("version", "")
        if version and not all(c in "0123456789." for c in version):
            errors.append(f"Invalid version format: {version}")

        price = metadata.get("price", 0)
        if isinstance(price, (int, float)) and price < 0:
            errors.append("Price cannot be negative")

        license_val = metadata.get("license", "free")
        if license_val not in ("free", "commercial", "subscription"):
            errors.append(f"Invalid license: {license_val}")

        return errors

    def _load_or_create_metadata(self, folder: Path) -> dict:
        meta_path = folder / "metadata.yaml"
        if meta_path.exists():
            with open(meta_path) as f:
                return yaml.safe_load(f) or {}

        # Create from config.yaml
        config_path = folder / "config.yaml"
        with open(config_path) as f:
            config = yaml.safe_load(f) or {}

        return {
            "name": config.get("name", folder.name),
            "version": "1.0.0",
            "author": "local",
            "description": config.get("description", ""),
            "tags": [],
            "category": "",
            "license": "free",
            "price": 0,
            "permissions_required": config.get("permissions", []),
        }

    def _collect_files(self, folder: Path) -> list[tuple[str, Path]]:
        """Collect files, excluding prohibited paths."""
        files: list[tuple[str, Path]] = []
        for path in sorted(folder.rglob("*")):
            if path.is_dir():
                continue
            rel = path.relative_to(folder)
            # Check exclusions
            parts = set(rel.parts)
            if parts & EXCLUDED_PATHS:
                continue
            if rel.name in EXCLUDED_PATHS:
                continue
            if rel.name == "metadata.yaml":  # We write our own
                continue
            files.append((str(rel).replace("\\", "/"), path))
        return files

    def _file_hash(self, path: Path) -> str:
        h = hashlib.sha256()
        with open(path, "rb") as f:
            for chunk in iter(lambda: f.read(8192), b""):
                h.update(chunk)
        return h.hexdigest()

    def _verify_checksums(self, zf: zipfile.ZipFile) -> None:
        checksum_data = zf.read("checksum.sha256").decode()
        for line in checksum_data.strip().split("\n"):
            if not line.strip():
                continue
            parts = line.split("  ", 1)
            if len(parts) != 2:
                continue
            expected_hash, filename = parts
            if filename not in zf.namelist():
                raise PackagingError(f"Checksum references missing file: {filename}")
            actual_hash = hashlib.sha256(zf.read(filename)).hexdigest()
            if actual_hash != expected_hash:
                raise PackagingError(f"Checksum mismatch for {filename}")
