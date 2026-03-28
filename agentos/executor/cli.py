"""CLI command executor with safety validation.

Runs shell commands via asyncio subprocess with timeout enforcement,
output truncation, and safety checks delegated to :class:`SafetyGuard`.
"""

from __future__ import annotations

import asyncio
import contextlib
import time
from typing import TYPE_CHECKING

from agentos.types import ExecutionResult
from agentos.utils.logging import get_logger

if TYPE_CHECKING:
    from agentos.executor.safety import SafetyGuard

logger = get_logger("executor")

# ── Custom exceptions ─────────────────────────────────────────────────

_OUTPUT_TRUNCATION_MARKER = "[output truncated at 1MB]"


class CommandBlockedError(Exception):
    """Raised when a command fails safety validation."""


class CommandTimeoutError(Exception):
    """Raised when a command exceeds its timeout."""


# ── Executor ──────────────────────────────────────────────────────────


class CLIExecutor:
    """Execute CLI commands with safety validation and resource limits.

    Args:
        safety: A :class:`SafetyGuard` instance (dependency injection).
        default_timeout: Default command timeout in seconds.
    """

    def __init__(
        self,
        safety: SafetyGuard,
        default_timeout: int = 300,
    ) -> None:
        self._safety = safety
        self._default_timeout = default_timeout

    # ── public API ────────────────────────────────────────────────────

    async def execute(
        self,
        command: str,
        timeout: int | None = None,
        cwd: str | None = None,
        extra_env: dict[str, str] | None = None,
    ) -> ExecutionResult:
        """Run *command* in a subprocess and return the result.

        Args:
            command: Shell command string.
            timeout: Per-command timeout in seconds (capped by config max).
            cwd: Working directory for the command.
            extra_env: Additional environment variables for the child process.

        Returns:
            :class:`ExecutionResult` with captured output.

        Raises:
            CommandBlockedError: If the command fails safety validation.
            CommandTimeoutError: If the command exceeds the timeout.
        """
        safe, reason = self._safety.validate(command)
        if not safe:
            raise CommandBlockedError(f"Blocked command: {reason}")

        effective_timeout = min(
            timeout if timeout is not None else self._default_timeout,
            self._safety.max_timeout,
        )

        env = self._safety.sanitize_env(extra_env)
        cmd_preview = command[:200]
        logger.info("Executing command: %s (timeout=%ds)", cmd_preview, effective_timeout)

        start = time.perf_counter()
        timed_out = False

        try:
            proc = await asyncio.create_subprocess_shell(
                command,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
                cwd=cwd,
                env=env,
            )

            try:
                stdout_bytes, stderr_bytes = await asyncio.wait_for(
                    proc.communicate(),
                    timeout=effective_timeout,
                )
            except TimeoutError:
                timed_out = True
                # Graceful terminate
                with contextlib.suppress(OSError):
                    proc.terminate()
                # Give 5 seconds then force-kill
                try:
                    await asyncio.wait_for(proc.wait(), timeout=5)
                except TimeoutError:
                    with contextlib.suppress(OSError):
                        proc.kill()
                    await proc.wait()

                stdout_bytes = b""
                stderr_bytes = b""
                if proc.stdout:
                    with contextlib.suppress(Exception):
                        stdout_bytes = await asyncio.wait_for(
                            proc.stdout.read(),
                            timeout=1,
                        )
                if proc.stderr:
                    with contextlib.suppress(Exception):
                        stderr_bytes = await asyncio.wait_for(
                            proc.stderr.read(),
                            timeout=1,
                        )

        except Exception as exc:
            duration_ms = (time.perf_counter() - start) * 1000
            logger.error("Subprocess creation failed: %s", exc)
            return ExecutionResult(
                command=command,
                exit_code=-1,
                stdout="",
                stderr=str(exc),
                duration_ms=duration_ms,
                timed_out=False,
            )

        duration_ms = (time.perf_counter() - start) * 1000

        stdout = self._truncate(stdout_bytes)
        stderr = self._truncate(stderr_bytes)
        exit_code = proc.returncode if proc.returncode is not None else -1

        # Log command metadata only; NEVER log stdout/stderr at INFO+
        logger.info(
            "Command finished: exit_code=%d duration=%.0fms timed_out=%s",
            exit_code,
            duration_ms,
            timed_out,
        )
        logger.debug("stdout=%s", stdout[:200])
        logger.debug("stderr=%s", stderr[:200])

        if timed_out:
            raise CommandTimeoutError(
                f"Command timed out after {effective_timeout}s",
            )

        return ExecutionResult(
            command=command,
            exit_code=exit_code,
            stdout=stdout,
            stderr=stderr,
            duration_ms=duration_ms,
            timed_out=timed_out,
        )

    # ── private helpers ───────────────────────────────────────────────

    def _truncate(self, data: bytes) -> str:
        """Decode and truncate output to configured max bytes."""
        max_bytes = self._safety.max_output_bytes
        if len(data) > max_bytes:
            truncated = data[:max_bytes]
            return truncated.decode("utf-8", errors="replace") + _OUTPUT_TRUNCATION_MARKER
        return data.decode("utf-8", errors="replace")
