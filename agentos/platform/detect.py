"""Platform detection and configuration."""

from __future__ import annotations

import enum
import os
import platform
import sys
from dataclasses import dataclass

from agentos.utils.logging import get_logger

logger = get_logger("platform")


class OSType(enum.StrEnum):
    WINDOWS = "windows"
    MACOS = "macos"
    LINUX = "linux"
    UNKNOWN = "unknown"


@dataclass(frozen=True)
class PlatformInfo:
    os_type: OSType
    os_version: str
    python_version: str
    architecture: str
    has_display: bool
    shell: str  # cmd.exe, /bin/bash, /bin/zsh
    keychain_backend: str  # "windows_credential", "macos_keychain", "secret_service"


def detect_platform() -> PlatformInfo:
    """Detect current platform and capabilities."""
    system = platform.system().lower()

    if system == "windows":
        os_type = OSType.WINDOWS
        shell = "cmd.exe"
        keychain = "windows_credential"
    elif system == "darwin":
        os_type = OSType.MACOS
        shell = "/bin/zsh"
        keychain = "macos_keychain"
    elif system == "linux":
        os_type = OSType.LINUX
        shell = "/bin/bash"
        keychain = "secret_service"
    else:
        os_type = OSType.UNKNOWN
        shell = "/bin/sh"
        keychain = "none"

    has_display = _check_display()

    return PlatformInfo(
        os_type=os_type,
        os_version=platform.version(),
        python_version=platform.python_version(),
        architecture=platform.machine(),
        has_display=has_display,
        shell=shell,
        keychain_backend=keychain,
    )


def _check_display() -> bool:
    """Check if a display is available."""
    if sys.platform == "win32":
        return True  # Windows always has a display (unless headless server)
    if sys.platform == "darwin":
        return True  # macOS always has a display
    # Linux: check DISPLAY or WAYLAND_DISPLAY
    return bool(os.environ.get("DISPLAY") or os.environ.get("WAYLAND_DISPLAY"))


def get_safety_patterns_for_os(os_type: OSType) -> list[str]:
    """Return OS-specific blocked command patterns."""
    base = [
        r"\bshutdown\b",
        r"\breboot\b",
        r"\bhalt\b",
        r"\bpoweroff\b",
    ]
    if os_type == OSType.WINDOWS:
        return [
            *base,
            r"\bdel\s+/[fs]",  # del /f /s
            r"\bformat\s+[a-zA-Z]:",  # format C:
            r"\brd\s+/s",  # rd /s /q
            r"\breg\s+delete\b",  # registry delete
            r"\bnet\s+stop\b",  # stop services
        ]
    if os_type == OSType.LINUX:
        return [
            *base,
            r"rm\s+-[a-zA-Z]*r[a-zA-Z]*\s+/",
            r"\bsystemctl\s+(stop|disable)\b",
            r"\bkillall\b",
        ]
    if os_type == OSType.MACOS:
        return [
            *base,
            r"rm\s+-[a-zA-Z]*r[a-zA-Z]*\s+/",
            r"\bkillall\b",
            r"\blaunchctl\s+unload\b",
        ]
    return base
