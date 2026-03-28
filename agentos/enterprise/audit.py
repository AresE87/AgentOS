"""Enhanced audit logging for enterprise deployments."""

from __future__ import annotations

import json
import uuid
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path

import aiosqlite

from agentos.utils.logging import get_logger

logger = get_logger("enterprise.audit")


@dataclass(frozen=True)
class AuditEntry:
    id: str
    timestamp: str
    user_id: str
    action: str  # "task_created", "task_completed", "setting_changed", "login", etc.
    resource_type: str  # "task", "playbook", "setting", "session"
    resource_id: str
    details: dict
    ip_address: str = ""
    organization_id: str = ""


class AuditLog:
    """Immutable, exportable audit log for enterprise compliance."""

    def __init__(self, db_path: str | Path = "data/audit.db") -> None:
        self._db_path = str(db_path)
        self._db: aiosqlite.Connection | None = None

    async def initialize(self) -> None:
        if self._db_path != ":memory:":
            Path(self._db_path).parent.mkdir(parents=True, exist_ok=True)
        self._db = await aiosqlite.connect(self._db_path)
        await self._db.execute("PRAGMA journal_mode=WAL")
        await self._db.execute("""
            CREATE TABLE IF NOT EXISTS audit_log (
                id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                user_id TEXT NOT NULL,
                action TEXT NOT NULL,
                resource_type TEXT NOT NULL,
                resource_id TEXT NOT NULL,
                details TEXT NOT NULL,
                ip_address TEXT DEFAULT '',
                organization_id TEXT DEFAULT ''
            )
        """)
        await self._db.execute("CREATE INDEX IF NOT EXISTS idx_audit_ts ON audit_log(timestamp)")
        await self._db.execute("CREATE INDEX IF NOT EXISTS idx_audit_user ON audit_log(user_id)")
        await self._db.execute("CREATE INDEX IF NOT EXISTS idx_audit_action ON audit_log(action)")
        await self._db.execute(
            "CREATE INDEX IF NOT EXISTS idx_audit_org ON audit_log(organization_id)"
        )
        await self._db.commit()

    async def log(
        self,
        user_id: str,
        action: str,
        resource_type: str,
        resource_id: str,
        details: dict | None = None,
        ip_address: str = "",
        organization_id: str = "",
    ) -> str:
        """Log an audit entry. Returns entry ID."""
        entry_id = uuid.uuid4().hex[:12]
        assert self._db is not None, "Call initialize() first"
        await self._db.execute(
            "INSERT INTO audit_log VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                entry_id,
                datetime.now(UTC).isoformat(),
                user_id,
                action,
                resource_type,
                resource_id,
                json.dumps(details or {}),
                ip_address,
                organization_id,
            ),
        )
        await self._db.commit()
        return entry_id

    async def query(
        self,
        user_id: str | None = None,
        action: str | None = None,
        start: str | None = None,
        end: str | None = None,
        organization_id: str | None = None,
        limit: int = 100,
    ) -> list[dict]:
        """Query audit entries with filters."""
        assert self._db is not None, "Call initialize() first"
        sql = "SELECT * FROM audit_log WHERE 1=1"
        params: list[str | int] = []
        if user_id:
            sql += " AND user_id = ?"
            params.append(user_id)
        if action:
            sql += " AND action = ?"
            params.append(action)
        if start:
            sql += " AND timestamp >= ?"
            params.append(start)
        if end:
            sql += " AND timestamp < ?"
            params.append(end)
        if organization_id:
            sql += " AND organization_id = ?"
            params.append(organization_id)
        sql += " ORDER BY timestamp DESC LIMIT ?"
        params.append(limit)

        async with self._db.execute(sql, params) as cursor:
            rows = await cursor.fetchall()
            cols = [d[0] for d in cursor.description]
            return [dict(zip(cols, row, strict=True)) for row in rows]

    async def export_json(self, output_path: Path, **filters: object) -> int:
        """Export audit entries to JSON file. Returns count."""
        entries = await self.query(limit=100000, **filters)  # type: ignore[arg-type]
        with open(output_path, "w") as f:
            json.dump(entries, f, indent=2)
        return len(entries)

    async def close(self) -> None:
        if self._db:
            await self._db.close()
