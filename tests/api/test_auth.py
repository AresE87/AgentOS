"""Tests for API authentication module (AOS-072)."""

from __future__ import annotations

from agentos.api.auth import APIKeyManager, RateLimiter

# ------------------------------------------------------------------
# APIKeyManager
# ------------------------------------------------------------------


def test_generate_key() -> None:
    km = APIKeyManager()
    key = km.generate_key("user1")
    assert key.startswith("aos_key_")
    assert len(key) > 20


def test_verify_valid() -> None:
    km = APIKeyManager()
    key = km.generate_key("user1")
    info = km.verify(key)
    assert info is not None
    assert info.user_id == "user1"


def test_verify_invalid() -> None:
    km = APIKeyManager()
    assert km.verify("random_string_not_a_key") is None


def test_revoke() -> None:
    km = APIKeyManager()
    key = km.generate_key("user1")
    info = km.verify(key)
    assert info is not None
    revoked = km.revoke(info.key_id)
    assert revoked is True
    assert km.verify(key) is None


# ------------------------------------------------------------------
# RateLimiter
# ------------------------------------------------------------------


def test_rate_limiter_allows() -> None:
    rl = RateLimiter()
    allowed, _ = rl.check("k1", plan="free")
    assert allowed is True


def test_rate_limiter_blocks() -> None:
    rl = RateLimiter()
    # Free plan = 100 per minute
    for _ in range(100):
        rl.check("k1", plan="free")
    allowed, _ = rl.check("k1", plan="free")
    assert allowed is False


def test_rate_limit_headers() -> None:
    rl = RateLimiter()
    _, headers = rl.check("k2", plan="pro")
    assert "X-RateLimit-Limit" in headers
    assert "X-RateLimit-Remaining" in headers
    assert "X-RateLimit-Reset" in headers
    assert headers["X-RateLimit-Limit"] == "1000"


def test_scopes() -> None:
    km = APIKeyManager()
    key = km.generate_key("user1", scopes=["tasks:read", "admin"])
    info = km.verify(key)
    assert info is not None
    assert "tasks:read" in info.scopes
    assert "admin" in info.scopes
    assert "tasks:write" not in info.scopes
