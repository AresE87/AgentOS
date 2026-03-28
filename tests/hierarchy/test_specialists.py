"""Tests for the SpecialistRegistry."""

from __future__ import annotations

from pathlib import Path

import pytest

from agentos.hierarchy.specialists import SpecialistRegistry
from agentos.types import TaskType

PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent
SPECIALISTS_DIR = PROJECT_ROOT / "config" / "specialists"


@pytest.fixture()
def registry() -> SpecialistRegistry:
    """Load all specialists from the real config directory."""
    reg = SpecialistRegistry(config_dir=SPECIALISTS_DIR)
    reg.load_all()
    return reg


class TestLoadAll:
    def test_load_all(self, registry: SpecialistRegistry) -> None:
        """All 8 specialist YAML files are loaded."""
        specialists = registry.all_specialists()
        assert len(specialists) == 8


class TestGetByName:
    def test_get_by_name(self, registry: SpecialistRegistry) -> None:
        """Find 'Software Architect' by exact name."""
        profile = registry.get_by_name("Software Architect")
        assert profile is not None
        assert profile.name == "Software Architect"

    def test_get_by_name_missing(self, registry: SpecialistRegistry) -> None:
        """Non-existent name returns None."""
        assert registry.get_by_name("Nonexistent Specialist") is None


class TestGetByCategory:
    def test_get_by_category(self, registry: SpecialistRegistry) -> None:
        """Find specialists in the software_development category."""
        results = registry.get_by_category("software_development")
        assert len(results) >= 1
        assert any(p.name == "Software Architect" for p in results)


class TestSelectBest:
    def test_select_best_code_task(self, registry: SpecialistRegistry) -> None:
        """'write a REST API' should select the Software Architect."""
        profile = registry.select_best(TaskType.CODE, "write a REST API with proper architecture")
        assert profile is not None
        assert profile.name == "Software Architect"

    def test_select_best_data_task(self, registry: SpecialistRegistry) -> None:
        """'analyze CSV data' should select the Data Analyst."""
        profile = registry.select_best(TaskType.DATA, "analyze CSV data and build a dashboard")
        assert profile is not None
        assert profile.name == "Data Analyst"

    def test_select_best_marketing(self, registry: SpecialistRegistry) -> None:
        """'create social media campaign' should select the Content Marketer."""
        profile = registry.select_best(TaskType.TEXT, "create social media campaign for engagement")
        assert profile is not None
        assert profile.name == "Content Marketer"

    def test_select_best_no_match(self, registry: SpecialistRegistry) -> None:
        """Random gibberish should return None."""
        profile = registry.select_best(TaskType.TEXT, "xyzzy plugh qwerty asdf")
        assert profile is None


class TestRequiredFields:
    def test_all_specialists_have_required_fields(self, registry: SpecialistRegistry) -> None:
        """Every specialist must have name, category, system_prompt, and tier."""
        for profile in registry.all_specialists():
            assert profile.name, f"Missing name on {profile}"
            assert profile.category, f"Missing category on {profile.name}"
            assert profile.system_prompt, f"Missing system_prompt on {profile.name}"
            assert profile.tier is not None, f"Missing tier on {profile.name}"
