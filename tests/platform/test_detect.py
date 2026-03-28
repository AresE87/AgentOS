"""Tests for platform detection and OS safety patterns."""

from __future__ import annotations

import re

from agentos.platform.detect import (
    OSType,
    PlatformInfo,
    detect_platform,
    get_safety_patterns_for_os,
)


class TestDetectPlatform:
    def test_detect_platform(self) -> None:
        """detect_platform returns a PlatformInfo with a valid os_type."""
        info = detect_platform()
        assert isinstance(info, PlatformInfo)
        assert info.os_type in list(OSType)

    def test_detect_has_fields(self) -> None:
        """All fields on PlatformInfo are populated (non-empty)."""
        info = detect_platform()
        assert info.os_version  # non-empty string
        assert info.python_version
        assert info.architecture
        assert isinstance(info.has_display, bool)
        assert info.shell
        assert info.keychain_backend


class TestSafetyPatterns:
    def test_safety_patterns_windows(self) -> None:
        """Windows patterns block del /f style commands."""
        patterns = get_safety_patterns_for_os(OSType.WINDOWS)
        joined = " ".join(patterns)
        assert "del" in joined
        # Verify the pattern actually matches dangerous commands
        assert any(re.search(p, "del /f C:\\stuff") for p in patterns)

    def test_safety_patterns_linux(self) -> None:
        """Linux patterns block rm -rf /."""
        patterns = get_safety_patterns_for_os(OSType.LINUX)
        assert any(re.search(p, "rm -rf /") for p in patterns)

    def test_safety_patterns_macos(self) -> None:
        """macOS patterns block killall."""
        patterns = get_safety_patterns_for_os(OSType.MACOS)
        assert any(re.search(p, "killall Finder") for p in patterns)
