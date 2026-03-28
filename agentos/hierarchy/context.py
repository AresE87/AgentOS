"""Chain context -- shared state between agents in a task chain."""

from __future__ import annotations

from agentos.utils.logging import get_logger

logger = get_logger("hierarchy.context")


class ChainContext:
    """Shared state between agents in a task chain.

    Each subtask can store key-value pairs. The special key ``"output"``
    holds the subtask's main result, which downstream subtasks can read
    via :meth:`get_dependency_outputs`.
    """

    def __init__(self, chain_id: str, max_size_bytes: int = 50_000) -> None:
        self._chain_id = chain_id
        self._max_size = max_size_bytes
        self._data: dict[str, dict[str, str]] = {}  # subtask_id -> {key: value}

    @property
    def chain_id(self) -> str:
        return self._chain_id

    def set(self, subtask_id: str, key: str, value: str) -> None:
        """Store a value for a subtask."""
        if subtask_id not in self._data:
            self._data[subtask_id] = {}
        self._data[subtask_id][key] = value
        # Check size
        total = sum(len(v) for vals in self._data.values() for v in vals.values())
        if total > self._max_size:
            logger.warning("Chain context exceeds max size (%d bytes)", total)

    def get(self, subtask_id: str, key: str) -> str | None:
        """Retrieve a value for a subtask, or None if not set."""
        return self._data.get(subtask_id, {}).get(key)

    def get_output(self, subtask_id: str) -> str | None:
        """Shorthand to get the ``output`` key for a subtask."""
        return self.get(subtask_id, "output")

    def get_dependency_outputs(self, subtask_id: str, depends_on: list[str]) -> str:
        """Collect outputs from dependency subtasks into a single string.

        Long outputs are truncated to 1000 characters each.
        """
        outputs: list[str] = []
        for dep_id in depends_on:
            output = self.get_output(dep_id)
            if output:
                # Truncate long outputs
                if len(output) > 1000:
                    output = output[:1000] + "... [truncated]"
                outputs.append(f"[{dep_id}]: {output}")
        return "\n\n".join(outputs) if outputs else ""

    def to_dict(self) -> dict:
        """Serialize to a plain dict."""
        return {"chain_id": self._chain_id, "data": self._data}

    @classmethod
    def from_dict(cls, data: dict) -> ChainContext:
        """Deserialize from a plain dict."""
        ctx = cls(chain_id=data.get("chain_id", ""))
        ctx._data = data.get("data", {})
        return ctx
