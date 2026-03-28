"""Safety guard for CLI command validation.

Validates commands against security rules loaded from cli_safety.yaml.
Handles command chaining, subshell extraction, and environment sanitisation.
"""

from __future__ import annotations

import os
import re
from pathlib import Path

import yaml

from agentos.utils.logging import get_logger

logger = get_logger("executor.safety")

_DEFAULT_SAFETY_CONFIG = (
    Path(__file__).resolve().parent.parent.parent / "config" / "cli_safety.yaml"
)

# ── Blocked patterns by category ──────────────────────────────────────

_BLOCKED_PATTERNS: dict[str, list[str]] = {
    "destructive": [
        r"rm\s+(-[a-zA-Z]*f[a-zA-Z]*\s+)?/",
        r"rm\s+-[a-zA-Z]*r[a-zA-Z]*\s+/",
        r"rm\s+-[a-zA-Z]*r[a-zA-Z]*\s+~",
        r"rm\s+-[a-zA-Z]*r[a-zA-Z]*\s+\.\s",
        r"mkfs\.",
        r"dd\s+.*of=/dev/",
        r"shred\s",
        r"wipefs\s",
        r":>\s*/",
        r">\s*/dev/sd",
    ],
    "system": [
        r"\bshutdown\b",
        r"\breboot\b",
        r"\bhalt\b",
        r"\bpoweroff\b",
        r"\binit\s+[06]\b",
        r"\bsystemctl\s+(stop|disable|mask)\b",
    ],
    "privilege_escalation": [
        r"\bsudo\b",
        r"\bsu\s",
        r"\bchmod\s+[0-7]*777\s+/",
        r"\bchown\s+root\b",
        r"\bpasswd\b",
        r"\bvisudo\b",
    ],
    "network_abuse": [
        r"\bnmap\b",
        r"\bnetcat\b|\bnc\s+-[a-zA-Z]*l",
        r"bash\s+-i\s+>&\s+/dev/tcp",
        r"\biptables\b",
        r"\bufw\b",
    ],
    "resource_exhaustion": [
        r":\(\)\s*\{.*\|.*\}",
        r"\byes\s*\|",
        r"while\s+true.*do",
        r"\bstress\b|\bstress-ng\b",
    ],
    "crypto_malware": [
        r"\bcryptominer\b",
        r"\bxmrig\b",
        r"\bcpuminer\b",
    ],
    "pipe_to_shell": [
        r"curl.*\|.*sh",
        r"wget.*\|.*sh",
    ],
}

# Pre-compile a flat list: (compiled_pattern, category, raw_pattern)
_COMPILED_PATTERNS: list[tuple[re.Pattern[str], str, str]] = []
for _cat, _pats in _BLOCKED_PATTERNS.items():
    for _p in _pats:
        _COMPILED_PATTERNS.append((re.compile(_p), _cat, _p))


class SafetyGuard:
    """Validates commands against security rules from cli_safety.yaml."""

    def __init__(self, config_path: Path | None = None) -> None:
        path = config_path or _DEFAULT_SAFETY_CONFIG

        config: dict = {}
        if path.exists():
            with open(path, encoding="utf-8") as fh:
                config = yaml.safe_load(fh) or {}

        # Additional patterns from config file (on top of hardcoded ones)
        extra_patterns: list[str] = config.get("blocked_patterns", [])
        self._extra_compiled: list[tuple[re.Pattern[str], str, str]] = [
            (re.compile(p), "config", p) for p in extra_patterns
        ]

        self._blocked_commands: list[str] = config.get("blocked_commands", [])

        limits = config.get("limits", {})
        self._max_timeout: int = limits.get("max_timeout", 300)
        self._max_output_bytes: int = limits.get("max_output_bytes", 1_048_576)

        self._strip_env_vars: list[str] = config.get("strip_env_vars", [])

    # ── public API ────────────────────────────────────────────────────

    @property
    def max_timeout(self) -> int:
        return self._max_timeout

    @property
    def max_output_bytes(self) -> int:
        return self._max_output_bytes

    def validate(self, command: str) -> tuple[bool, str]:
        """Validate command safety.

        Returns:
            ``(True, "")`` when safe, ``(False, reason)`` when blocked.
        """
        # Check exact-match blocklist first
        stripped = command.strip()
        for blocked in self._blocked_commands:
            if stripped == blocked or stripped.startswith(blocked):
                return False, f"Command matches blocked command: {blocked}"

        # Check the FULL command first (catches cross-segment patterns
        # like "curl ... | sh" where the pipe is part of the pattern).
        safe, reason = self._check_pattern(command)
        if not safe:
            return False, reason

        # Split into sub-commands (chaining + subshells) and check each
        sub_commands = self._split_command_chain(command)
        for sub_cmd in sub_commands:
            safe, reason = self._check_pattern(sub_cmd)
            if not safe:
                return False, reason

        return True, ""

    def sanitize_env(
        self,
        extra_env: dict[str, str] | None = None,
    ) -> dict[str, str]:
        """Copy os.environ, remove blocked vars, add *extra_env*.

        NEVER modifies ``os.environ`` directly.
        """
        env = os.environ.copy()
        for var in self._strip_env_vars:
            env.pop(var, None)
        if extra_env:
            env.update(extra_env)
        return env

    # ── private helpers ───────────────────────────────────────────────

    def _split_command_chain(self, command: str) -> list[str]:
        """Split on ``; && || |`` operators and extract ``$()`` / backtick content."""
        parts: list[str] = []

        # Split on shell chaining operators (;  &&  ||  |)
        # Use regex that splits on these operators but keeps each segment
        segments = re.split(r"\s*(?:;|&&|\|\||(?<!\|)\|(?!\|))\s*", command)
        for seg in segments:
            seg = seg.strip()
            if seg:
                parts.append(seg)

        # Extract content from $(...) subshells
        for match in re.finditer(r"\$\(([^)]+)\)", command):
            inner = match.group(1).strip()
            if inner:
                parts.append(inner)

        # Extract content from backtick subshells
        for match in re.finditer(r"`([^`]+)`", command):
            inner = match.group(1).strip()
            if inner:
                parts.append(inner)

        return parts if parts else [command]

    def _check_pattern(self, command: str) -> tuple[bool, str]:
        """Check a single command against all blocked patterns."""
        # Hardcoded patterns
        for compiled, category, raw in _COMPILED_PATTERNS:
            if compiled.search(command):
                return False, (f"Blocked by {category} rule: {raw}")

        # Extra patterns from config
        for compiled, category, raw in self._extra_compiled:
            if compiled.search(command):
                return False, (f"Blocked by {category} rule: {raw}")

        return True, ""
