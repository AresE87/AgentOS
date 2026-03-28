"""Tests for logging utilities."""

from agentos.utils.logging import get_logger, redact, setup_logging


def test_redact_openai_key():
    text = "Using key sk-abc123def456ghi789jkl012mno345"
    result = redact(text)
    assert "sk-abc123" not in result
    assert "[REDACTED]" in result


def test_redact_google_key():
    text = "Google key: AIzaSyAbCdEfGhIjKlMnOpQrStUvWxYz1234567"
    result = redact(text)
    assert "AIzaSy" not in result
    assert "[REDACTED]" in result


def test_redact_telegram_token():
    text = "Token: 123456789:ABCDefGhIJKlmNOPQRstUVWxyz_1234567"
    result = redact(text)
    assert "ABCDef" not in result
    assert "[REDACTED]" in result


def test_redact_no_secrets():
    text = "This is a normal log message"
    assert redact(text) == text


def test_get_logger():
    logger = get_logger("test")
    assert logger.name == "agentos.test"


def test_setup_logging():
    logger = setup_logging("DEBUG")
    assert logger is not None
