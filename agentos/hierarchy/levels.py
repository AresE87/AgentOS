"""Agent levels and profiles for the multi-agent hierarchy."""

from __future__ import annotations

import enum
from dataclasses import dataclass, field
from typing import TYPE_CHECKING

import yaml

from agentos.types import LLMTier
from agentos.utils.logging import get_logger

if TYPE_CHECKING:
    from pathlib import Path

logger = get_logger("hierarchy.levels")


class AgentLevel(enum.StrEnum):
    JUNIOR = "junior"
    SPECIALIST = "specialist"
    SENIOR = "senior"
    MANAGER = "manager"
    ORCHESTRATOR = "orchestrator"


@dataclass(frozen=True)
class AgentProfile:
    """Profile that configures agent behavior."""

    name: str
    level: AgentLevel
    system_prompt: str
    tier: LLMTier
    allowed_tools: list[str] = field(default_factory=lambda: ["cli"])
    max_tokens: int = 4096
    temperature: float = 0.7
    category: str | None = None
    description: str = ""


DEFAULT_PROFILES: dict[AgentLevel, AgentProfile] = {
    AgentLevel.JUNIOR: AgentProfile(
        name="Junior Agent",
        level=AgentLevel.JUNIOR,
        system_prompt=(
            "You are a helpful assistant. Answer questions directly and concisely. "
            "For simple tasks, provide the answer. For commands, suggest the exact "
            "command to run in a ```bash code block."
        ),
        tier=LLMTier.CHEAP,
        allowed_tools=["cli"],
        max_tokens=2048,
        temperature=0.5,
        description="Handles simple, repetitive tasks efficiently.",
    ),
    AgentLevel.SPECIALIST: AgentProfile(
        name="Specialist Agent",
        level=AgentLevel.SPECIALIST,
        system_prompt=(
            "You are a domain specialist. Apply your expertise to solve the task thoroughly. "
            "Provide detailed, well-reasoned answers within your area of expertise. "
            "If a task is outside your domain, say so clearly."
        ),
        tier=LLMTier.STANDARD,
        allowed_tools=["cli", "files"],
        max_tokens=4096,
        temperature=0.7,
        description="Domain expert with specialized knowledge.",
    ),
    AgentLevel.SENIOR: AgentProfile(
        name="Senior Agent",
        level=AgentLevel.SENIOR,
        system_prompt=(
            "You are an experienced AI agent. Break complex tasks into clear steps. "
            "Think before acting. Verify your work. Provide detailed, well-structured "
            "responses. Consider edge cases and potential issues."
        ),
        tier=LLMTier.STANDARD,
        allowed_tools=["cli", "screen", "files"],
        max_tokens=8192,
        temperature=0.7,
        description="Handles complex, multi-step tasks with depth.",
    ),
    AgentLevel.MANAGER: AgentProfile(
        name="Manager Agent",
        level=AgentLevel.MANAGER,
        system_prompt=(
            "You are a task manager coordinating a team of AI agents. Your job is to "
            "review sub-task outputs, ensure quality, handle coordination between tasks, "
            "and compile final results. Focus on synthesis and quality control."
        ),
        tier=LLMTier.STANDARD,
        allowed_tools=["cli", "screen", "files", "network"],
        max_tokens=8192,
        temperature=0.5,
        description="Coordinates sub-tasks and ensures quality.",
    ),
    AgentLevel.ORCHESTRATOR: AgentProfile(
        name="Orchestrator Agent",
        level=AgentLevel.ORCHESTRATOR,
        system_prompt=(
            "You are the meta-planner. Analyze incoming tasks, determine complexity, "
            "decide whether to handle directly or decompose into sub-tasks. "
            "Select the right agent level and specialist for each piece. "
            "Monitor execution and compile results."
        ),
        tier=LLMTier.STANDARD,
        allowed_tools=["cli", "screen", "files", "network"],
        max_tokens=4096,
        temperature=0.3,
        description="Plans and delegates complex workflows.",
    ),
}


def get_profile(level: AgentLevel) -> AgentProfile:
    """Get the default profile for a level."""
    return DEFAULT_PROFILES[level]


def load_profiles_from_yaml(config_path: Path) -> dict[str, AgentProfile]:
    """Load custom profiles from a YAML config file."""
    if not config_path.exists():
        return {}
    with open(config_path) as f:
        data = yaml.safe_load(f) or {}
    profiles: dict[str, AgentProfile] = {}
    for name, cfg in data.items():
        level = AgentLevel(cfg.get("level", "junior"))
        tier_val = cfg.get("tier", 1)
        tier = LLMTier(tier_val) if isinstance(tier_val, int) else LLMTier.CHEAP
        profiles[name] = AgentProfile(
            name=name,
            level=level,
            system_prompt=cfg.get("system_prompt", ""),
            tier=tier,
            allowed_tools=cfg.get("tools", ["cli"]),
            max_tokens=cfg.get("max_tokens", 4096),
            temperature=cfg.get("temperature", 0.7),
            category=cfg.get("category"),
            description=cfg.get("description", ""),
        )
    return profiles
