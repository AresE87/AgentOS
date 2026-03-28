"""Tests for settings module."""

from agentos.settings import Settings, load_settings


def test_settings_defaults():
    s = Settings()
    assert s.anthropic_api_key == ""
    assert s.log_level == "INFO"
    assert s.max_cost_per_task == 1.00


def test_settings_available_providers_empty():
    s = Settings()
    assert s.available_providers() == {}


def test_settings_available_providers_with_keys():
    s = Settings(anthropic_api_key="sk-test", openai_api_key="sk-test2")
    providers = s.available_providers()
    assert "anthropic" in providers
    assert "openai" in providers
    assert "google" not in providers


def test_settings_repr_redacts_keys():
    s = Settings(
        anthropic_api_key="sk-secret-key-12345",
        openai_api_key="sk-openai-secret",
        telegram_bot_token="123456:ABC-TOKEN",
    )
    repr_str = repr(s)
    assert "sk-secret" not in repr_str
    assert "sk-openai" not in repr_str
    assert "ABC-TOKEN" not in repr_str
    assert "***" in repr_str


def test_settings_is_frozen():
    s = Settings()
    try:
        s.log_level = "DEBUG"  # type: ignore[misc]
        raise AssertionError("Should not allow mutation")
    except AttributeError:
        pass


def test_load_settings_from_env(monkeypatch):
    monkeypatch.setenv("ANTHROPIC_API_KEY", "sk-test-key")
    monkeypatch.setenv("AGENTOS_LOG_LEVEL", "DEBUG")
    monkeypatch.setenv("AGENTOS_MAX_COST_PER_TASK", "2.50")

    s = load_settings()
    assert s.anthropic_api_key == "sk-test-key"
    assert s.log_level == "DEBUG"
    assert s.max_cost_per_task == 2.50
