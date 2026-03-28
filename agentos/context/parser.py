"""Context Folder parser for AgentOS playbooks.

Reads playbook.md and config.yaml from context folders, validates their
contents, and returns structured ContextFolder objects.
"""

from __future__ import annotations

from pathlib import Path

import yaml

from agentos.types import ContextFolder, LLMTier, PlaybookConfig
from agentos.utils.logging import get_logger

logger = get_logger("context")

# ─── Valid values ─────────────────────────────────────────────────

_TIER_MAP: dict[int, LLMTier] = {
    1: LLMTier.CHEAP,
    2: LLMTier.STANDARD,
    3: LLMTier.PREMIUM,
}

_VALID_PERMISSIONS = {"cli", "screen", "files", "network"}

# ─── Errors ──────────────────────────────────────────────────────


class ContextFolderError(Exception):
    """Base error for context folder operations."""

    def __init__(self, path: Path, message: str) -> None:
        self.path = path
        super().__init__(f"{path}: {message}")


class PlaybookNotFoundError(ContextFolderError):
    """Raised when playbook.md is missing from a context folder."""

    def __init__(self, path: Path) -> None:
        super().__init__(path, "playbook.md not found")


class ConfigNotFoundError(ContextFolderError):
    """Raised when config.yaml is missing from a context folder."""

    def __init__(self, path: Path) -> None:
        super().__init__(path, "config.yaml not found")


class ConfigValidationError(ContextFolderError):
    """Raised when config.yaml fails validation."""

    def __init__(self, path: Path, errors: list[str]) -> None:
        self.errors = errors
        msg = "config validation failed: " + "; ".join(errors)
        super().__init__(path, msg)


# ─── Parser ──────────────────────────────────────────────────────


class ContextFolderParser:
    """Parses context folders containing playbook.md and config.yaml."""

    # ── public API ──────────────────────────────────────────────

    async def parse(self, path: Path) -> ContextFolder:
        """Parse a single context folder into a ``ContextFolder``.

        Args:
            path: Directory containing playbook.md and config.yaml.

        Returns:
            Parsed ``ContextFolder`` dataclass.

        Raises:
            ContextFolderError: If *path* does not exist or is not a directory.
            PlaybookNotFoundError: If playbook.md is missing.
            ConfigNotFoundError: If config.yaml is missing.
            ConfigValidationError: If config.yaml fails validation.
        """
        path = Path(path)

        if not path.exists():
            raise ContextFolderError(path, "path does not exist")
        if not path.is_dir():
            raise ContextFolderError(path, "path is not a directory")

        # 1. Read playbook.md (required)
        playbook_path = path / "playbook.md"
        if not playbook_path.exists():
            raise PlaybookNotFoundError(path)

        raw_content = playbook_path.read_text(encoding="utf-8")

        # 2. Read config.yaml (required)
        config_path = path / "config.yaml"
        if not config_path.exists():
            raise ConfigNotFoundError(path)

        config_text = config_path.read_text(encoding="utf-8")
        config_data: dict = yaml.safe_load(config_text) or {}

        # 3. Validate config
        errors = self.validate_config(config_data)
        if errors:
            raise ConfigValidationError(path, errors)

        # 4. Build PlaybookConfig
        config = self._build_config(config_data)

        # 5. Extract title / instructions from playbook.md
        title, instructions = self._parse_playbook(raw_content, path)

        # If no H1 heading found, use directory name
        if not title:
            title = path.name

        # Use playbook title as config description fallback
        # (config.name comes from config.yaml, title from playbook.md)

        logger.debug("Parsed playbook '%s' from %s", config.name, path)

        return ContextFolder(
            path=str(path),
            config=config,
            instructions=instructions,
        )

    async def parse_many(self, base_dir: Path) -> list[ContextFolder]:
        """Scan *base_dir* for subdirectories containing playbook.md.

        Invalid folders are skipped with a warning.

        Args:
            base_dir: Parent directory to scan.

        Returns:
            List of successfully parsed ``ContextFolder`` objects.
        """
        base_dir = Path(base_dir)
        if not base_dir.is_dir():
            logger.warning("Playbooks directory does not exist: %s", base_dir)
            return []

        results: list[ContextFolder] = []
        for child in sorted(base_dir.iterdir()):
            if not child.is_dir():
                continue
            if not (child / "playbook.md").exists():
                continue
            try:
                folder = await self.parse(child)
                results.append(folder)
            except ContextFolderError as exc:
                logger.warning("Skipping invalid playbook %s: %s", child, exc)

        logger.info("Loaded %d playbook(s) from %s", len(results), base_dir)
        return results

    @staticmethod
    def validate_config(config: dict) -> list[str]:
        """Validate a config dictionary and return a list of error strings.

        Returns:
            Empty list if valid, otherwise list of human-readable errors.
        """
        errors: list[str] = []

        # name: required, non-empty, max 100 chars
        name = config.get("name")
        if name is None or (isinstance(name, str) and not name.strip()):
            errors.append("name is required and must be non-empty")
        elif not isinstance(name, str):
            errors.append("name must be a string")
        elif len(name) > 100:
            errors.append("name must be at most 100 characters")

        # tier: must be 1, 2, or 3
        tier = config.get("tier")
        if tier is not None and tier not in (1, 2, 3):
            errors.append("tier must be 1, 2, or 3")

        # timeout: must be > 0 and <= 600
        timeout = config.get("timeout")
        if timeout is not None:
            try:
                t = int(timeout)
                if t <= 0 or t > 600:
                    errors.append("timeout must be > 0 and <= 600")
            except (TypeError, ValueError):
                errors.append("timeout must be an integer")

        # permissions: each must be one of the valid set
        permissions = config.get("permissions", [])
        if isinstance(permissions, list):
            for perm in permissions:
                if perm not in _VALID_PERMISSIONS:
                    errors.append(
                        f"invalid permission '{perm}'; "
                        f"allowed: {', '.join(sorted(_VALID_PERMISSIONS))}"
                    )

        return errors

    # ── internal helpers ────────────────────────────────────────

    @staticmethod
    def _parse_playbook(content: str, path: Path) -> tuple[str, str]:
        """Extract title and full instructions from playbook text.

        Returns:
            ``(title, instructions)`` tuple.  Title may be empty if
            no H1 heading is found (caller should fall back to dir name).
        """
        instructions = content.strip()

        lines = content.splitlines()

        # Title: first line starting with "# "
        title = ""
        for line in lines:
            stripped = line.strip()
            if stripped.startswith("# "):
                title = stripped.lstrip("#").strip()
                break

        return title, instructions

    @staticmethod
    def _build_config(data: dict) -> PlaybookConfig:
        """Build a ``PlaybookConfig`` from a validated config dict."""
        name = str(data.get("name", ""))
        description = str(data.get("description", ""))

        tier_value = data.get("tier", 1)
        tier = _TIER_MAP.get(int(tier_value), LLMTier.CHEAP)

        timeout = int(data.get("timeout", 300))
        permissions = list(data.get("permissions", []))
        allowed_commands = list(data.get("allowed_commands", []))
        blocked_commands = list(data.get("blocked_commands", []))

        return PlaybookConfig(
            name=name,
            description=description,
            tier=tier,
            timeout=timeout,
            permissions=permissions,
            allowed_commands=allowed_commands,
            blocked_commands=blocked_commands,
        )
