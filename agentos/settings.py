"""Centralized configuration loaded from environment variables.

All configuration is read from .env via python-dotenv into an immutable
Settings dataclass. No module reads os.environ directly — everything
goes through Settings.
"""

from __future__ import annotations

import json
import os
from dataclasses import dataclass
from pathlib import Path

from dotenv import load_dotenv

CONFIG_FILENAME = "agentos_config.json"


@dataclass(frozen=True)
class Settings:
    """Immutable application settings loaded from environment."""

    # LLM Provider API Keys
    anthropic_api_key: str = ""
    openai_api_key: str = ""
    google_api_key: str = ""

    # Telegram
    telegram_bot_token: str = ""

    # Agent config
    log_level: str = "INFO"
    max_cost_per_task: float = 1.00
    cli_timeout: int = 300
    db_path: str = "data/agentos.db"

    # Paths
    config_dir: str = "config"
    playbooks_dir: str = "examples/playbooks"

    def available_providers(self) -> dict[str, str]:
        """Return dict of provider_name -> api_key for configured providers."""
        providers: dict[str, str] = {}
        if self.anthropic_api_key:
            providers["anthropic"] = self.anthropic_api_key
        if self.openai_api_key:
            providers["openai"] = self.openai_api_key
        if self.google_api_key:
            providers["google"] = self.google_api_key
        return providers

    def __repr__(self) -> str:
        """Redact API keys in repr to prevent leaks."""
        return (
            "Settings("
            f"anthropic_api_key={'***' if self.anthropic_api_key else '<not set>'}, "
            f"openai_api_key={'***' if self.openai_api_key else '<not set>'}, "
            f"google_api_key={'***' if self.google_api_key else '<not set>'}, "
            f"telegram_bot_token={'***' if self.telegram_bot_token else '<not set>'}, "
            f"log_level={self.log_level!r}, "
            f"max_cost_per_task={self.max_cost_per_task}, "
            f"cli_timeout={self.cli_timeout}, "
            f"db_path={self.db_path!r}"
            ")"
        )


def save_settings_to_file(updates: dict, config_path: Path | None = None) -> None:
    """Save settings updates to a JSON config file."""
    path = config_path or Path("data/agentos_config.json")
    path.parent.mkdir(parents=True, exist_ok=True)

    # Load existing
    existing: dict = {}
    if path.exists():
        with open(path) as f:  # noqa: PTH123
            existing = json.load(f)

    # Merge updates
    existing.update(updates)

    # Write
    with open(path, "w") as f:  # noqa: PTH123
        json.dump(existing, f, indent=2)


def load_settings(env_path: str | Path | None = None) -> Settings:
    """Load settings from env vars AND JSON config file (config overrides env).

    Args:
        env_path: Optional path to .env file. If None, searches default locations.

    Returns:
        Immutable Settings instance.
    """
    load_dotenv(dotenv_path=env_path)

    # Check for JSON config file
    config_path = Path(os.environ.get("AGENTOS_CONFIG_PATH", "data/agentos_config.json"))
    config_overrides: dict = {}
    if config_path.exists():
        with open(config_path) as f:  # noqa: PTH123
            config_overrides = json.load(f)

    return Settings(
        anthropic_api_key=config_overrides.get(
            "anthropic_api_key", os.environ.get("ANTHROPIC_API_KEY", "")
        ),
        openai_api_key=config_overrides.get(
            "openai_api_key", os.environ.get("OPENAI_API_KEY", "")
        ),
        google_api_key=config_overrides.get(
            "google_api_key", os.environ.get("GOOGLE_API_KEY", "")
        ),
        telegram_bot_token=config_overrides.get(
            "telegram_bot_token", os.environ.get("TELEGRAM_BOT_TOKEN", "")
        ),
        log_level=config_overrides.get(
            "log_level", os.environ.get("AGENTOS_LOG_LEVEL", "INFO")
        ),
        max_cost_per_task=float(
            config_overrides.get(
                "max_cost_per_task", os.environ.get("AGENTOS_MAX_COST_PER_TASK", "1.00")
            )
        ),
        cli_timeout=int(
            config_overrides.get(
                "cli_timeout", os.environ.get("AGENTOS_CLI_TIMEOUT", "300")
            )
        ),
        db_path=os.environ.get("AGENTOS_DB_PATH", "data/agentos.db"),
        config_dir=os.environ.get("AGENTOS_CONFIG_DIR", "config"),
        playbooks_dir=os.environ.get("AGENTOS_PLAYBOOKS_DIR", "examples/playbooks"),
    )
