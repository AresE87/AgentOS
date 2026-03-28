"""Specialist profiles registry -- loads and selects domain experts."""

from __future__ import annotations

from typing import TYPE_CHECKING

import yaml

if TYPE_CHECKING:
    from pathlib import Path

from agentos.hierarchy.levels import AgentLevel, AgentProfile
from agentos.types import LLMTier, TaskType
from agentos.utils.logging import get_logger

logger = get_logger("hierarchy.specialists")

# Keywords for matching tasks to specialist categories
CATEGORY_KEYWORDS: dict[str, list[str]] = {
    "software_development": [
        "code",
        "program",
        "develop",
        "software",
        "api",
        "debug",
        "architecture",
        "design pattern",
        "refactor",
    ],
    "design_creative": [
        "design",
        "ui",
        "ux",
        "wireframe",
        "mockup",
        "layout",
        "creative",
        "visual",
        "brand",
    ],
    "business_finance": [
        "finance",
        "budget",
        "revenue",
        "profit",
        "accounting",
        "invoice",
        "tax",
        "financial",
        "forecast",
    ],
    "marketing_growth": [
        "marketing",
        "content",
        "seo",
        "campaign",
        "social media",
        "growth",
        "engagement",
        "audience",
    ],
    "legal_compliance": [
        "legal",
        "contract",
        "compliance",
        "regulation",
        "terms",
        "privacy",
        "gdpr",
        "policy",
    ],
    "data_analytics": [
        "data",
        "analytics",
        "statistics",
        "csv",
        "chart",
        "graph",
        "metrics",
        "kpi",
        "dashboard",
        "report",
    ],
    "operations": [
        "project",
        "timeline",
        "milestone",
        "sprint",
        "task",
        "plan",
        "schedule",
        "coordinate",
        "manage",
    ],
    "sales": [
        "sales",
        "lead",
        "prospect",
        "pipeline",
        "crm",
        "outreach",
        "pitch",
        "proposal",
        "client",
    ],
}


class SpecialistRegistry:
    """Registry that loads specialist YAML profiles and selects the best match."""

    def __init__(self, config_dir: Path | None = None) -> None:
        self._specialists: dict[str, AgentProfile] = {}
        self._config_dir = config_dir

    def load_all(self) -> list[AgentProfile]:
        """Load all specialist profiles from config/specialists/."""
        if not self._config_dir or not self._config_dir.exists():
            logger.warning("Specialists config dir not found: %s", self._config_dir)
            return []

        for yaml_file in sorted(self._config_dir.glob("*.yaml")):
            try:
                with open(yaml_file) as f:
                    data = yaml.safe_load(f) or {}
                profile = self._parse_profile(data)
                self._specialists[profile.name] = profile
                logger.debug("Loaded specialist: %s", profile.name)
            except Exception:
                logger.warning("Failed to load specialist from %s", yaml_file)

        logger.info("Loaded %d specialists", len(self._specialists))
        return list(self._specialists.values())

    def _parse_profile(self, data: dict) -> AgentProfile:  # type: ignore[type-arg]
        tier_val = data.get("tier", 2)
        return AgentProfile(
            name=data["name"],
            level=AgentLevel(data.get("level", "specialist")),
            system_prompt=data.get("system_prompt", ""),
            tier=LLMTier(tier_val) if isinstance(tier_val, int) else LLMTier.STANDARD,
            allowed_tools=data.get("tools", ["cli"]),
            max_tokens=data.get("max_tokens", 4096),
            temperature=data.get("temperature", 0.7),
            category=data.get("category", ""),
            description=data.get("description", ""),
        )

    def get_by_name(self, name: str) -> AgentProfile | None:
        """Look up a specialist by exact name."""
        return self._specialists.get(name)

    def get_by_category(self, category: str) -> list[AgentProfile]:
        """Return all specialists belonging to a category."""
        return [p for p in self._specialists.values() if p.category == category]

    def select_best(self, task_type: TaskType, task_description: str) -> AgentProfile | None:
        """Select the best specialist using keyword matching against the task description."""
        text_lower = task_description.lower()
        best_match: AgentProfile | None = None
        best_score = 0

        for profile in self._specialists.values():
            if not profile.category:
                continue
            keywords = CATEGORY_KEYWORDS.get(profile.category, [])
            score = sum(1 for kw in keywords if kw in text_lower)
            if score > best_score:
                best_score = score
                best_match = profile

        if best_score > 0:
            logger.info(
                "Selected specialist: %s (score=%d)",
                best_match.name if best_match else "none",
                best_score,
            )
        return best_match

    def all_specialists(self) -> list[AgentProfile]:
        """Return all loaded specialist profiles."""
        return list(self._specialists.values())
