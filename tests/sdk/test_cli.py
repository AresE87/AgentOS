"""Tests for AgentOS CLI (AOS-078)."""

from __future__ import annotations

from agentos.sdk.cli import main


def test_help() -> None:
    assert main(["help"]) == 0


def test_unknown_command() -> None:
    assert main(["xyz"]) == 1


def test_no_args() -> None:
    assert main([]) == 0


def test_run_no_text() -> None:
    assert main(["run"]) == 1
