"""Task decomposer -- breaks complex tasks into sub-tasks via LLM."""

from __future__ import annotations

import json
from dataclasses import dataclass, field

from agentos.hierarchy.levels import AgentLevel
from agentos.types import LLMRequest, LLMTier, TaskClassification, TaskInput, TaskType
from agentos.utils.logging import get_logger

logger = get_logger("hierarchy.decomposer")

DECOMPOSE_PROMPT = """You are a task planning agent. Decompose the following task into atomic sub-tasks.

Task: "{task_description}"
Task type: {task_type}
Complexity: {complexity}

Respond ONLY with JSON:
{{
  "subtasks": [
    {{
      "id": "subtask_1",
      "description": "Clear, actionable description",
      "depends_on": [],
      "suggested_level": "junior|specialist|senior",
      "suggested_specialist": "category_name or null",
      "estimated_complexity": 1
    }}
  ],
  "reasoning": "Brief explanation"
}}

Rules:
- Maximum 10 sub-tasks
- Each sub-task should be independently executable
- Specify dependencies if B needs output from A
- Use the simplest agent level possible
- Sub-tasks without dependencies can run in parallel"""


@dataclass
class SubTaskDefinition:
    """Definition of a single sub-task produced by decomposition."""

    id: str
    description: str
    depends_on: list[str] = field(default_factory=list)
    suggested_level: AgentLevel = AgentLevel.JUNIOR
    suggested_specialist: str | None = None
    estimated_complexity: int = 1


@dataclass
class TaskPlan:
    """A plan consisting of ordered sub-tasks for a complex task."""

    original_task: str
    subtasks: list[SubTaskDefinition]
    estimated_total_cost: float = 0.0
    reasoning: str = ""


class TaskDecomposer:
    """Decomposes complex tasks into sub-tasks using an LLM."""

    def __init__(self, gateway=None) -> None:
        self._gateway = gateway

    def should_decompose(self, classification: TaskClassification) -> bool:
        """Return True if the task is complex enough to warrant decomposition."""
        return classification.complexity >= 3

    async def decompose(
        self, task_input: TaskInput, classification: TaskClassification
    ) -> TaskPlan:
        """Decompose a task into sub-tasks via LLM.

        Falls back to a single-task plan when no gateway is available or
        when the LLM response cannot be parsed.
        """
        if not self._gateway:
            return TaskPlan(
                original_task=task_input.text,
                subtasks=[SubTaskDefinition(id="subtask_1", description=task_input.text)],
                reasoning="No gateway -- single task fallback",
            )

        prompt = DECOMPOSE_PROMPT.format(
            task_description=task_input.text,
            task_type=classification.task_type.value,
            complexity=classification.complexity,
        )
        request = LLMRequest(
            prompt=prompt,
            tier=LLMTier.STANDARD,
            task_type=TaskType.TEXT,
            system_prompt="You are a task planner. Respond only with valid JSON.",
        )
        response = await self._gateway.complete(request)
        return self._parse_response(response.content, task_input.text)

    def _parse_response(self, content: str, original_task: str) -> TaskPlan:
        """Parse LLM JSON response into a TaskPlan."""
        try:
            text = content.strip()
            if text.startswith("```"):
                text = text.split("```")[1]
                if text.startswith("json"):
                    text = text[4:]
            data = json.loads(text)
        except (json.JSONDecodeError, IndexError):
            logger.warning("Failed to parse decomposition, using single-task fallback")
            return TaskPlan(
                original_task=original_task,
                subtasks=[SubTaskDefinition(id="subtask_1", description=original_task)],
                reasoning="Parse failure -- single task fallback",
            )

        subtasks: list[SubTaskDefinition] = []
        for i, st in enumerate(data.get("subtasks", [])[:10]):  # Max 10
            level_str = st.get("suggested_level", "junior")
            try:
                level = AgentLevel(level_str)
            except ValueError:
                level = AgentLevel.JUNIOR
            subtasks.append(
                SubTaskDefinition(
                    id=st.get("id", f"subtask_{i + 1}"),
                    description=st.get("description", ""),
                    depends_on=st.get("depends_on", []),
                    suggested_level=level,
                    suggested_specialist=st.get("suggested_specialist"),
                    estimated_complexity=st.get("estimated_complexity", 1),
                )
            )

        if not subtasks:
            subtasks = [SubTaskDefinition(id="subtask_1", description=original_task)]

        return TaskPlan(
            original_task=original_task,
            subtasks=subtasks,
            reasoning=data.get("reasoning", ""),
        )
