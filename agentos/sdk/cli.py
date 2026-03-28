"""AgentOS CLI tool."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import TYPE_CHECKING, Any

from agentos.utils.logging import get_logger

if TYPE_CHECKING:
    from collections.abc import Callable

logger = get_logger("cli")


def main(args: list[str] | None = None) -> int:
    """CLI entry point."""
    if args is None:
        args = sys.argv[1:]
    if not args:
        _print_help()
        return 0

    command = args[0]
    rest = args[1:]

    commands: dict[str, Callable[[list[str]], int]] = {
        "run": _cmd_run,
        "status": _cmd_status,
        "tasks": _cmd_tasks,
        "playbooks": _cmd_playbooks,
        "health": _cmd_health,
        "help": lambda _: _print_help() or 0,
    }

    handler = commands.get(command)
    if not handler:
        print(f"Unknown command: {command}")
        _print_help()
        return 1

    return handler(rest)


def _get_client() -> Any:
    """Get SDK client from config."""
    from agentos.sdk.client import AgentOS

    config = _load_config()
    return AgentOS(
        api_key=config.get("api_key", ""),
        base_url=config.get("base_url", "http://localhost:8080"),
    )


def _load_config() -> dict:
    config_path = Path.home() / ".agentos" / "config.yaml"
    if config_path.exists():
        import yaml

        with open(config_path) as f:
            return yaml.safe_load(f) or {}
    return {}


def _cmd_run(args: list[str]) -> int:
    if not args:
        print("Usage: agentos run <task description>")
        return 1
    text = " ".join(args)
    try:
        client = _get_client()
        result = client.run_task(text)
        print(f"Task: {result.task_id}")
        print(f"Status: {result.status}")
        if result.output:
            print(f"\n{result.output}")
        return 0
    except Exception as e:
        print(f"Error: {e}")
        return 1


def _cmd_status(args: list[str]) -> int:
    try:
        client = _get_client()
        status = client.get_status()
        print(json.dumps(status, indent=2))
        return 0
    except Exception as e:
        print(f"Error: {e}")
        return 1


def _cmd_tasks(args: list[str]) -> int:
    try:
        client = _get_client()
        tasks = client.list_tasks()
        if not tasks:
            print("No tasks found.")
        else:
            for t in tasks:
                print(f"  {t.get('task_id', '?')} | {t.get('status', '?')}")
        return 0
    except Exception as e:
        print(f"Error: {e}")
        return 1


def _cmd_playbooks(args: list[str]) -> int:
    try:
        client = _get_client()
        playbooks = client.list_playbooks()
        if not playbooks:
            print("No playbooks installed.")
        else:
            for p in playbooks:
                print(f"  {p.get('name', '?')} (tier {p.get('tier', '?')})")
        return 0
    except Exception as e:
        print(f"Error: {e}")
        return 1


def _cmd_health(args: list[str]) -> int:
    try:
        client = _get_client()
        health = client.get_health()
        print(json.dumps(health, indent=2))
        return 0
    except Exception as e:
        print(f"Error: {e}")
        return 1


def _print_help() -> None:
    print(
        """AgentOS CLI

Commands:
  run <text>      Execute a task
  status          Agent status
  tasks           List recent tasks
  playbooks       List playbooks
  health          Health check
  help            Show this help
"""
    )


if __name__ == "__main__":
    sys.exit(main())
