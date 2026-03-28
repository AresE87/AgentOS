"""Tests to validate example playbook structure."""

from pathlib import Path

import yaml

PROJECT_ROOT = Path(__file__).parent.parent
PLAYBOOKS_DIR = PROJECT_ROOT / "examples" / "playbooks"

# Only valid playbooks (those expected to have proper name field)
VALID_PLAYBOOKS = ["hello_world", "system_monitor", "code_reviewer"]


def test_hello_world_playbook_exists():
    playbook = PLAYBOOKS_DIR / "hello_world" / "playbook.md"
    assert playbook.exists()
    content = playbook.read_text()
    assert "Hello World" in content


def test_hello_world_config_exists():
    config = PLAYBOOKS_DIR / "hello_world" / "config.yaml"
    assert config.exists()
    data = yaml.safe_load(config.read_text())
    assert data["name"] == "Hello World"
    assert data["tier"] == 1
    assert "permissions" in data


def test_system_monitor_playbook_exists():
    playbook = PLAYBOOKS_DIR / "system_monitor" / "playbook.md"
    assert playbook.exists()
    content = playbook.read_text()
    assert "System Monitor" in content


def test_system_monitor_config_exists():
    config = PLAYBOOKS_DIR / "system_monitor" / "config.yaml"
    assert config.exists()
    data = yaml.safe_load(config.read_text())
    assert data["name"] == "System Monitor"
    assert "permissions" in data
    assert "cli" in data["permissions"]


def test_code_reviewer_config_has_name():
    config = PLAYBOOKS_DIR / "code_reviewer" / "config.yaml"
    assert config.exists()
    data = yaml.safe_load(config.read_text())
    assert data["name"] == "Code Reviewer"
    assert data["tier"] == 3


def test_all_valid_playbooks_have_required_files():
    for name in VALID_PLAYBOOKS:
        playbook_dir = PLAYBOOKS_DIR / name
        assert (playbook_dir / "playbook.md").exists(), f"{name} missing playbook.md"
        assert (playbook_dir / "config.yaml").exists(), f"{name} missing config.yaml"


def test_all_valid_playbooks_have_name_in_config():
    for name in VALID_PLAYBOOKS:
        config_path = PLAYBOOKS_DIR / name / "config.yaml"
        data = yaml.safe_load(config_path.read_text())
        assert "name" in data, f"{name}/config.yaml missing 'name' field"
        assert data["name"], f"{name}/config.yaml has empty 'name' field"
