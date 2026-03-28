"""Recovery strategies for failed sub-tasks."""

from __future__ import annotations

from agentos.utils.logging import get_logger

logger = get_logger("hierarchy.recovery")


class RecoveryStrategy:
    """Retry, tier upgrade, and specialist swap strategies."""

    def __init__(self, max_retries: int = 2) -> None:
        self._max_retries = max_retries
        self._retry_counts: dict[str, int] = {}

    async def attempt(self, subtask_id: str, execute_fn) -> bool:
        """Attempt recovery for a failed subtask.

        Returns True if recovered successfully.
        """
        count = self._retry_counts.get(subtask_id, 0)
        if count >= self._max_retries:
            logger.warning(
                "Subtask %s exceeded max retries (%d)",
                subtask_id,
                self._max_retries,
            )
            return False

        self._retry_counts[subtask_id] = count + 1
        logger.info(
            "Retrying subtask %s (attempt %d/%d)",
            subtask_id,
            count + 1,
            self._max_retries,
        )

        try:
            result = await execute_fn()
            return result is not None and result.status.value == "completed"
        except Exception:
            logger.warning("Recovery attempt %d failed for %s", count + 1, subtask_id)
            return False

    def reset(self) -> None:
        """Clear all retry counters."""
        self._retry_counts.clear()
