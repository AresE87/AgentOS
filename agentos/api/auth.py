"""API authentication — keys, rate limiting, scopes."""

from __future__ import annotations

import hashlib
import secrets
import time
from collections import defaultdict
from dataclasses import dataclass, field

from fastapi import Header, HTTPException

from agentos.utils.logging import get_logger

logger = get_logger("api.auth")

RATE_LIMITS = {"free": 100, "pro": 1000, "enterprise": 10000}
VALID_SCOPES = {"tasks:read", "tasks:write", "playbooks:read", "playbooks:write", "admin"}


@dataclass
class APIKeyInfo:
    key_id: str
    user_id: str
    plan: str = "free"
    scopes: list[str] = field(default_factory=lambda: ["tasks:read", "tasks:write"])


class APIKeyManager:
    """Manages API keys."""

    def __init__(self) -> None:
        self._keys: dict[str, APIKeyInfo] = {}  # key_hash -> info
        self._raw_to_hash: dict[str, str] = {}  # For dev/testing only

    def generate_key(
        self,
        user_id: str,
        plan: str = "free",
        scopes: list[str] | None = None,
    ) -> str:
        """Generate a new API key. Returns the raw key (only shown once)."""
        raw_key = f"aos_key_{secrets.token_hex(24)}"
        key_hash = hashlib.sha256(raw_key.encode()).hexdigest()
        self._keys[key_hash] = APIKeyInfo(
            key_id=key_hash[:12],
            user_id=user_id,
            plan=plan,
            scopes=scopes or ["tasks:read", "tasks:write"],
        )
        self._raw_to_hash[raw_key] = key_hash
        return raw_key

    def verify(self, raw_key: str) -> APIKeyInfo | None:
        key_hash = hashlib.sha256(raw_key.encode()).hexdigest()
        return self._keys.get(key_hash)

    def revoke(self, key_id: str) -> bool:
        for h, info in list(self._keys.items()):
            if info.key_id == key_id:
                del self._keys[h]
                return True
        return False


class RateLimiter:
    """Token bucket rate limiter."""

    def __init__(self) -> None:
        self._requests: dict[str, list[float]] = defaultdict(list)

    def check(self, key_id: str, plan: str = "free") -> tuple[bool, dict[str, str]]:
        limit = RATE_LIMITS.get(plan, 100)
        now = time.time()
        window_start = now - 60

        # Clean old entries
        self._requests[key_id] = [t for t in self._requests[key_id] if t > window_start]

        remaining = limit - len(self._requests[key_id])
        headers = {
            "X-RateLimit-Limit": str(limit),
            "X-RateLimit-Remaining": str(max(0, remaining)),
            "X-RateLimit-Reset": str(int(window_start + 60)),
        }

        if remaining <= 0:
            return False, headers

        self._requests[key_id].append(now)
        return True, headers


# Global instances (would be injected in production)
_key_manager = APIKeyManager()
_rate_limiter = RateLimiter()


def get_key_manager() -> APIKeyManager:
    return _key_manager


def get_rate_limiter() -> RateLimiter:
    return _rate_limiter


async def verify_api_key(authorization: str = Header(None)) -> APIKeyInfo:
    """FastAPI dependency for auth."""
    if not authorization:
        raise HTTPException(status_code=401, detail="Missing Authorization header")

    # Extract token from "Bearer xxx"
    parts = authorization.split(" ", 1)
    token = parts[1] if len(parts) == 2 else parts[0]

    info = _key_manager.verify(token)
    if not info:
        raise HTTPException(status_code=401, detail="Invalid API key")

    # Rate limit check
    allowed, headers = _rate_limiter.check(info.key_id, info.plan)
    if not allowed:
        raise HTTPException(status_code=429, detail="Rate limit exceeded", headers=headers)

    return info
