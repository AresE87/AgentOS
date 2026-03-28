"""Tests for enterprise audit logging."""

from __future__ import annotations

import json
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from pathlib import Path

from agentos.enterprise.audit import AuditLog


@pytest.fixture()
async def audit_log() -> AuditLog:
    log = AuditLog(db_path=":memory:")
    await log.initialize()
    return log


@pytest.mark.asyncio()
async def test_log_and_query(audit_log: AuditLog) -> None:
    """Log an entry and retrieve it via query."""
    entry_id = await audit_log.log(
        user_id="user1",
        action="task_created",
        resource_type="task",
        resource_id="t-001",
        details={"text": "hello"},
    )
    assert entry_id

    results = await audit_log.query()
    assert len(results) == 1
    assert results[0]["id"] == entry_id
    assert results[0]["user_id"] == "user1"
    assert results[0]["action"] == "task_created"


@pytest.mark.asyncio()
async def test_query_by_user(audit_log: AuditLog) -> None:
    """Filter audit entries by user_id."""
    await audit_log.log("alice", "login", "session", "s-1")
    await audit_log.log("bob", "login", "session", "s-2")

    results = await audit_log.query(user_id="alice")
    assert len(results) == 1
    assert results[0]["user_id"] == "alice"


@pytest.mark.asyncio()
async def test_query_by_action(audit_log: AuditLog) -> None:
    """Filter audit entries by action."""
    await audit_log.log("u1", "login", "session", "s-1")
    await audit_log.log("u1", "task_created", "task", "t-1")

    results = await audit_log.query(action="task_created")
    assert len(results) == 1
    assert results[0]["action"] == "task_created"


@pytest.mark.asyncio()
async def test_query_by_date_range(audit_log: AuditLog) -> None:
    """Filter by start and end timestamps."""
    await audit_log.log("u1", "login", "session", "s-1")

    # Query with a future start should return nothing
    results = await audit_log.query(start="2099-01-01T00:00:00")
    assert len(results) == 0

    # Query with a past start should return the entry
    results = await audit_log.query(start="2000-01-01T00:00:00")
    assert len(results) == 1


@pytest.mark.asyncio()
async def test_export_json(audit_log: AuditLog, tmp_path: Path) -> None:
    """Export to JSON file produces valid JSON."""
    await audit_log.log("u1", "login", "session", "s-1")
    await audit_log.log("u1", "task_created", "task", "t-1")

    out = tmp_path / "audit_export.json"
    count = await audit_log.export_json(out)
    assert count == 2

    data = json.loads(out.read_text())
    assert len(data) == 2


@pytest.mark.asyncio()
async def test_immutable(audit_log: AuditLog) -> None:
    """Audit log has no UPDATE or DELETE operations exposed."""
    assert not hasattr(audit_log, "update")
    assert not hasattr(audit_log, "delete")
    assert not hasattr(audit_log, "remove")
