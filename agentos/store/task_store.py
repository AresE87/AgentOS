"""SQLite-backed persistence for tasks, execution logs, and LLM usage.

Uses aiosqlite with WAL mode for safe concurrent reads. All timestamps
are stored as ISO 8601 UTC strings. All primary keys are UUID v4.
"""

from __future__ import annotations

import uuid
from datetime import UTC, datetime
from pathlib import Path

import aiosqlite

from agentos.types import (
    ExecutionResult,
    LLMResponse,
    TaskClassification,
    TaskInput,
    TaskStatus,
)
from agentos.utils.logging import get_logger

logger = get_logger("store")

SCHEMA_VERSION = 1
DB_STDOUT_MAX = 10 * 1024  # 10 KB max for stdout/stderr in DB


def _uuid() -> str:
    return str(uuid.uuid4())


def _now_iso() -> str:
    return datetime.now(UTC).isoformat()


class TaskStore:
    """Async SQLite store for task tracking and usage analytics."""

    def __init__(self, db_path: str | Path = "data/agentos.db") -> None:
        self._db_path = str(db_path)
        self._db: aiosqlite.Connection | None = None

    # ── lifecycle ────────────────────────────────────────────────

    async def initialize(self) -> None:
        """Open the database connection, enable WAL, and run migrations."""
        if self._db_path != ":memory:":
            Path(self._db_path).parent.mkdir(parents=True, exist_ok=True)

        self._db = await aiosqlite.connect(self._db_path)
        self._db.row_factory = aiosqlite.Row
        await self._db.execute("PRAGMA journal_mode=WAL")
        await self._create_tables()
        await self._check_schema_version()
        logger.info("TaskStore initialized (db=%s)", self._db_path)

    async def close(self) -> None:
        """Close the database connection."""
        if self._db is not None:
            await self._db.close()
            self._db = None
            logger.info("TaskStore closed")

    # ── schema ───────────────────────────────────────────────────

    async def _create_tables(self) -> None:
        assert self._db is not None
        await self._db.executescript(
            """\
            CREATE TABLE IF NOT EXISTS _schema_version (
                version   INTEGER NOT NULL,
                applied_at TEXT   NOT NULL
            );

            CREATE TABLE IF NOT EXISTS tasks (
                id            TEXT PRIMARY KEY,
                user_id       TEXT    NOT NULL DEFAULT '',
                chat_id       TEXT    NOT NULL DEFAULT '',
                source        TEXT    NOT NULL,
                input_text    TEXT    NOT NULL,
                task_type     TEXT,
                complexity    INTEGER,
                tier          INTEGER,
                model_used    TEXT,
                provider      TEXT,
                tokens_in     INTEGER DEFAULT 0,
                tokens_out    INTEGER DEFAULT 0,
                cost_estimate REAL    DEFAULT 0.0,
                status        TEXT    NOT NULL,
                output_text   TEXT    DEFAULT '',
                error_message TEXT    DEFAULT '',
                created_at    TEXT    NOT NULL,
                started_at    TEXT,
                completed_at  TEXT,
                duration_ms   REAL    DEFAULT 0.0
            );

            CREATE INDEX IF NOT EXISTS idx_tasks_status
                ON tasks(status);
            CREATE INDEX IF NOT EXISTS idx_tasks_created_at
                ON tasks(created_at);
            CREATE INDEX IF NOT EXISTS idx_tasks_user_id
                ON tasks(user_id);
            CREATE INDEX IF NOT EXISTS idx_tasks_chat_id
                ON tasks(chat_id);

            CREATE TABLE IF NOT EXISTS execution_log (
                id            TEXT PRIMARY KEY,
                task_id       TEXT NOT NULL REFERENCES tasks(id),
                executor_type TEXT NOT NULL,
                command       TEXT NOT NULL,
                exit_code     INTEGER NOT NULL,
                success       INTEGER NOT NULL DEFAULT 0,
                stdout        TEXT DEFAULT '',
                stderr        TEXT DEFAULT '',
                duration_ms   REAL DEFAULT 0.0,
                created_at    TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_execution_log_task_id
                ON execution_log(task_id);

            CREATE TABLE IF NOT EXISTS llm_usage (
                id            TEXT PRIMARY KEY,
                task_id       TEXT NOT NULL REFERENCES tasks(id),
                provider      TEXT NOT NULL,
                model         TEXT NOT NULL,
                tokens_in     INTEGER DEFAULT 0,
                tokens_out    INTEGER DEFAULT 0,
                cost_estimate REAL    DEFAULT 0.0,
                latency_ms    REAL    DEFAULT 0.0,
                success       INTEGER DEFAULT 1,
                error_type    TEXT    DEFAULT '',
                fallback_index INTEGER DEFAULT 0,
                created_at    TEXT    NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_llm_usage_task_id
                ON llm_usage(task_id);
            CREATE INDEX IF NOT EXISTS idx_llm_usage_provider
                ON llm_usage(provider);
            CREATE INDEX IF NOT EXISTS idx_llm_usage_created_at
                ON llm_usage(created_at);
            CREATE INDEX IF NOT EXISTS idx_llm_usage_model
                ON llm_usage(model);

            CREATE TABLE IF NOT EXISTS chain_log (
                id          TEXT PRIMARY KEY,
                chain_id    TEXT NOT NULL,
                timestamp   TEXT NOT NULL,
                agent_name  TEXT NOT NULL,
                agent_level TEXT NOT NULL,
                event_type  TEXT NOT NULL,
                message     TEXT NOT NULL,
                metadata    TEXT,
                FOREIGN KEY (chain_id) REFERENCES task_chains(id)
            );

            CREATE INDEX IF NOT EXISTS idx_chainlog_chain
                ON chain_log(chain_id, timestamp);
            """
        )
        await self._db.commit()

    async def _check_schema_version(self) -> None:
        """Insert schema version row if the table is empty."""
        assert self._db is not None
        cursor = await self._db.execute("SELECT COUNT(*) FROM _schema_version")
        row = await cursor.fetchone()
        if row is not None and row[0] == 0:
            await self._db.execute(
                "INSERT INTO _schema_version (version, applied_at) VALUES (?, ?)",
                (SCHEMA_VERSION, _now_iso()),
            )
            await self._db.commit()

    # ── task CRUD ────────────────────────────────────────────────

    async def create_task(self, task_input: TaskInput) -> str:
        """Insert a new task from a TaskInput. Returns the task_id (UUID)."""
        assert self._db is not None

        task_id = _uuid()
        now = _now_iso()
        await self._db.execute(
            """\
            INSERT INTO tasks
                (id, user_id, chat_id, source, input_text, status, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            """,
            (
                task_id,
                getattr(task_input, "user_id", ""),
                task_input.chat_id,
                task_input.source,
                task_input.text,
                TaskStatus.PENDING.value,
                now,
            ),
        )
        await self._db.commit()
        logger.debug("Created task %s (source=%s)", task_id, task_input.source)
        return task_id

    async def update_task_status(self, task_id: str, status: TaskStatus) -> None:
        """Update the status of a task. Sets started_at when moving to RUNNING."""
        assert self._db is not None

        if status == TaskStatus.RUNNING:
            await self._db.execute(
                "UPDATE tasks SET status = ?, started_at = ? WHERE id = ?",
                (status.value, _now_iso(), task_id),
            )
        else:
            await self._db.execute(
                "UPDATE tasks SET status = ? WHERE id = ?",
                (status.value, task_id),
            )
        await self._db.commit()
        logger.debug("Updated task %s -> %s", task_id, status.value)

    async def update_task_classification(
        self, task_id: str, classification: TaskClassification
    ) -> None:
        """Update the task_type, complexity, and tier from a classification."""
        assert self._db is not None

        await self._db.execute(
            """\
            UPDATE tasks
               SET task_type   = ?,
                   complexity  = ?,
                   tier        = ?
             WHERE id = ?
            """,
            (
                classification.task_type.value,
                classification.complexity,
                classification.tier.value,
                task_id,
            ),
        )
        await self._db.commit()
        logger.debug("Classified task %s as %s", task_id, classification.task_type.value)

    async def complete_task(
        self,
        task_id: str,
        output: str,
        llm_response: LLMResponse | None = None,
    ) -> None:
        """Mark a task as completed with output and optional LLM metadata."""
        assert self._db is not None

        now = _now_iso()
        # Calculate duration from started_at if available
        cursor = await self._db.execute(
            "SELECT started_at, created_at FROM tasks WHERE id = ?", (task_id,)
        )
        row = await cursor.fetchone()
        duration_ms = 0.0
        if row is not None:
            ref = row["started_at"] or row["created_at"]
            start = datetime.fromisoformat(ref)
            duration_ms = (datetime.now(UTC) - start).total_seconds() * 1000

        if llm_response is not None:
            await self._db.execute(
                """\
                UPDATE tasks
                   SET status        = ?,
                       output_text   = ?,
                       completed_at  = ?,
                       duration_ms   = ?,
                       model_used    = ?,
                       provider      = ?,
                       tokens_in     = ?,
                       tokens_out    = ?,
                       cost_estimate = ?
                 WHERE id = ?
                """,
                (
                    TaskStatus.COMPLETED.value,
                    output,
                    now,
                    duration_ms,
                    llm_response.model,
                    llm_response.provider,
                    llm_response.tokens_in,
                    llm_response.tokens_out,
                    llm_response.cost_estimate,
                    task_id,
                ),
            )
        else:
            await self._db.execute(
                """\
                UPDATE tasks
                   SET status       = ?,
                       output_text  = ?,
                       completed_at = ?,
                       duration_ms  = ?
                 WHERE id = ?
                """,
                (TaskStatus.COMPLETED.value, output, now, duration_ms, task_id),
            )
        await self._db.commit()
        logger.debug("Completed task %s", task_id)

    async def fail_task(self, task_id: str, error: str) -> None:
        """Mark a task as failed with an error message."""
        assert self._db is not None

        now = _now_iso()
        await self._db.execute(
            """\
            UPDATE tasks
               SET status        = ?,
                   error_message = ?,
                   completed_at  = ?
             WHERE id = ?
            """,
            (TaskStatus.FAILED.value, error, now, task_id),
        )
        await self._db.commit()
        logger.debug("Failed task %s: %s", task_id, error[:80])

    async def get_task(self, task_id: str) -> dict | None:
        """Fetch a single task by ID as a dict, or None if not found."""
        assert self._db is not None

        cursor = await self._db.execute("SELECT * FROM tasks WHERE id = ?", (task_id,))
        row = await cursor.fetchone()
        if row is None:
            return None
        return dict(row)

    async def get_recent_tasks(self, limit: int = 10) -> list[dict]:
        """Return the most recent tasks ordered by created_at descending."""
        assert self._db is not None

        cursor = await self._db.execute(
            "SELECT * FROM tasks ORDER BY created_at DESC LIMIT ?", (limit,)
        )
        rows = await cursor.fetchall()
        return [dict(r) for r in rows]

    # ── execution log ────────────────────────────────────────────

    async def save_execution(self, task_id: str, result: ExecutionResult) -> None:
        """Record a CLI execution event linked to a task. Truncates stdout/stderr to 10KB."""
        assert self._db is not None

        stdout = result.stdout[:DB_STDOUT_MAX] if result.stdout else ""
        stderr = result.stderr[:DB_STDOUT_MAX] if result.stderr else ""

        await self._db.execute(
            """\
            INSERT INTO execution_log
                (id, task_id, executor_type, command, exit_code, success,
                 stdout, stderr, duration_ms, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                _uuid(),
                task_id,
                "subprocess",
                result.command,
                result.exit_code,
                int(result.exit_code == 0),
                stdout,
                stderr,
                result.duration_ms,
                _now_iso(),
            ),
        )
        await self._db.commit()
        logger.debug("Logged execution for task %s (exit=%d)", task_id, result.exit_code)

    # ── LLM usage ────────────────────────────────────────────────

    async def save_llm_usage(
        self,
        task_id: str,
        response: LLMResponse,
        fallback_index: int = 0,
        error_type: str = "",
    ) -> None:
        """Record an LLM call linked to a task."""
        assert self._db is not None

        await self._db.execute(
            """\
            INSERT INTO llm_usage
                (id, task_id, provider, model, tokens_in, tokens_out,
                 cost_estimate, latency_ms, success, error_type,
                 fallback_index, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                _uuid(),
                task_id,
                response.provider,
                response.model,
                response.tokens_in,
                response.tokens_out,
                response.cost_estimate,
                response.latency_ms,
                1 if not error_type else 0,
                error_type,
                fallback_index,
                _now_iso(),
            ),
        )
        await self._db.commit()
        logger.debug(
            "Logged LLM usage for task %s (%s/%s fb=%d)",
            task_id,
            response.provider,
            response.model,
            fallback_index,
        )

    # ── analytics queries ────────────────────────────────────────

    async def get_cost_by_period(self, start: str, end: str) -> dict:
        """Aggregate LLM costs for a date range."""
        assert self._db is not None

        cursor = await self._db.execute(
            """\
            SELECT
                COALESCE(SUM(cost_estimate), 0.0) AS total_cost,
                COALESCE(SUM(tokens_in), 0)        AS total_tokens_in,
                COALESCE(SUM(tokens_out), 0)       AS total_tokens_out,
                COUNT(*)                            AS total_calls
            FROM llm_usage
            WHERE created_at >= ? AND created_at <= ?
            """,
            (start, end),
        )
        row = await cursor.fetchone()
        assert row is not None
        return {
            "total_cost": row["total_cost"],
            "total_tokens_in": row["total_tokens_in"],
            "total_tokens_out": row["total_tokens_out"],
            "total_calls": row["total_calls"],
        }

    async def get_cost_by_provider(self, start: str, end: str) -> list[dict]:
        """Aggregate LLM costs grouped by provider for a date range."""
        assert self._db is not None

        cursor = await self._db.execute(
            """\
            SELECT
                provider,
                COALESCE(SUM(cost_estimate), 0.0) AS total_cost,
                COUNT(*)                            AS call_count,
                COALESCE(AVG(latency_ms), 0.0)     AS avg_latency
            FROM llm_usage
            WHERE created_at >= ? AND created_at <= ?
            GROUP BY provider
            """,
            (start, end),
        )
        rows = await cursor.fetchall()
        return [dict(r) for r in rows]

    async def get_cost_by_model(self, start: str, end: str) -> list[dict]:
        """Aggregate LLM costs grouped by model for a date range."""
        assert self._db is not None

        cursor = await self._db.execute(
            """\
            SELECT
                model,
                COALESCE(SUM(cost_estimate), 0.0) AS total_cost,
                COUNT(*)                            AS call_count,
                SUM(success)                        AS success_count
            FROM llm_usage
            WHERE created_at >= ? AND created_at <= ?
            GROUP BY model
            """,
            (start, end),
        )
        rows = await cursor.fetchall()
        return [dict(r) for r in rows]

    async def get_success_rate(self, start: str, end: str) -> dict:
        """Task success rate for a date range."""
        assert self._db is not None

        cursor = await self._db.execute(
            """\
            SELECT
                COUNT(*)                                              AS total,
                SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) AS completed,
                SUM(CASE WHEN status = 'failed'    THEN 1 ELSE 0 END) AS failed
            FROM tasks
            WHERE created_at >= ? AND created_at <= ?
            """,
            (start, end),
        )
        row = await cursor.fetchone()
        assert row is not None
        return {
            "total": row["total"],
            "completed": row["completed"],
            "failed": row["failed"],
        }

    async def get_task_executions(self, task_id: str) -> list[dict]:
        """All execution_log entries for a task."""
        assert self._db is not None

        cursor = await self._db.execute(
            "SELECT * FROM execution_log WHERE task_id = ? ORDER BY created_at",
            (task_id,),
        )
        rows = await cursor.fetchall()
        return [dict(r) for r in rows]

    async def get_task_llm_usage(self, task_id: str) -> list[dict]:
        """All llm_usage entries for a task, ordered by fallback_index."""
        assert self._db is not None

        cursor = await self._db.execute(
            "SELECT * FROM llm_usage WHERE task_id = ? ORDER BY fallback_index",
            (task_id,),
        )
        rows = await cursor.fetchall()
        return [dict(r) for r in rows]

    # ── chain log ─────────────────────────────────────────────────

    async def save_chain_log(
        self,
        chain_id: str,
        agent_name: str,
        agent_level: str,
        event_type: str,
        message: str,
        metadata: str | None = None,
    ) -> str:
        """Insert a chain log entry. Returns the log entry id."""
        assert self._db is not None

        entry_id = _uuid()
        now = _now_iso()
        await self._db.execute(
            """\
            INSERT INTO chain_log
                (id, chain_id, timestamp, agent_name, agent_level,
                 event_type, message, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (entry_id, chain_id, now, agent_name, agent_level, event_type, message, metadata),
        )
        await self._db.commit()
        logger.debug("Saved chain log %s for chain %s", entry_id, chain_id)
        return entry_id

    async def get_chain_log(self, chain_id: str, limit: int = 100) -> list[dict]:
        """Return log entries for a chain, ordered by timestamp ascending."""
        assert self._db is not None

        cursor = await self._db.execute(
            "SELECT * FROM chain_log WHERE chain_id = ? ORDER BY timestamp ASC LIMIT ?",
            (chain_id, limit),
        )
        rows = await cursor.fetchall()
        return [dict(r) for r in rows]

    async def get_chain_history(self, limit: int = 20) -> list[dict]:
        """Return recent chains with status and log entry count.

        Groups chain_log entries by chain_id and returns summary info.
        """
        assert self._db is not None

        cursor = await self._db.execute(
            """\
            SELECT
                chain_id,
                MIN(timestamp) AS created_at,
                COUNT(*)       AS log_count,
                MAX(CASE WHEN event_type = 'status' THEN message ELSE NULL END) AS last_status
            FROM chain_log
            GROUP BY chain_id
            ORDER BY MIN(timestamp) DESC
            LIMIT ?
            """,
            (limit,),
        )
        rows = await cursor.fetchall()
        return [dict(r) for r in rows]
