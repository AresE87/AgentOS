"""Logging utilities with secret redaction.

Uses rich for formatted console output. The redact() function
strips API keys and sensitive strings from log messages.
"""

from __future__ import annotations

import logging
import re

from rich.console import Console
from rich.logging import RichHandler

console = Console()

# Patterns that look like API keys
_SECRET_PATTERNS = [
    re.compile(r"sk-[a-zA-Z0-9\-_]{20,}"),  # OpenAI / Anthropic style
    re.compile(r"AIza[a-zA-Z0-9\-_]{30,}"),  # Google style
    re.compile(r"\b[0-9]+:[A-Za-z0-9_\-]{30,}"),  # Telegram bot token
]


def redact(text: str) -> str:
    """Replace API keys and secrets in text with [REDACTED].

    Args:
        text: String that may contain secrets.

    Returns:
        String with secrets replaced.
    """
    result = text
    for pattern in _SECRET_PATTERNS:
        result = pattern.sub("[REDACTED]", result)
    return result


def setup_logging(level: str = "INFO") -> logging.Logger:
    """Configure the root logger with rich handler and redaction.

    Args:
        level: Log level string (DEBUG, INFO, WARNING, ERROR).

    Returns:
        Configured root logger.
    """
    log_level = getattr(logging, level.upper(), logging.INFO)

    logging.basicConfig(
        level=log_level,
        format="%(message)s",
        datefmt="[%X]",
        handlers=[
            RichHandler(
                console=console,
                rich_tracebacks=True,
                tracebacks_show_locals=False,
                show_path=False,
            )
        ],
    )

    logger = logging.getLogger("agentos")
    logger.setLevel(log_level)
    return logger


def get_logger(name: str) -> logging.Logger:
    """Get a named logger under the agentos namespace.

    Args:
        name: Logger name (e.g., 'gateway', 'executor').

    Returns:
        Logger instance.
    """
    return logging.getLogger(f"agentos.{name}")
