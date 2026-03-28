"""Tests for the AgentOS REST API server (AOS-071)."""

from __future__ import annotations

import pytest
from fastapi.testclient import TestClient

from agentos.api.auth import get_key_manager
from agentos.api.server import app


@pytest.fixture()
def client() -> TestClient:
    return TestClient(app)


@pytest.fixture()
def api_key() -> str:
    km = get_key_manager()
    return km.generate_key("test_user", plan="pro")


@pytest.fixture()
def auth_headers(api_key: str) -> dict[str, str]:
    return {"Authorization": f"Bearer {api_key}"}


# ------------------------------------------------------------------
# Health (no auth)
# ------------------------------------------------------------------


def test_health_no_auth(client: TestClient) -> None:
    resp = client.get("/api/v1/health")
    assert resp.status_code == 200
    body = resp.json()
    assert body["data"]["healthy"] is True
    assert body["error"] is None


# ------------------------------------------------------------------
# Tasks
# ------------------------------------------------------------------


def test_create_task(client: TestClient, auth_headers: dict[str, str]) -> None:
    resp = client.post(
        "/api/v1/tasks",
        json={"text": "hello world"},
        headers=auth_headers,
    )
    assert resp.status_code == 200
    data = resp.json()["data"]
    assert "task_id" in data
    assert data["status"] == "pending"


def test_list_tasks(client: TestClient, auth_headers: dict[str, str]) -> None:
    resp = client.get("/api/v1/tasks", headers=auth_headers)
    assert resp.status_code == 200
    data = resp.json()["data"]
    assert isinstance(data["tasks"], list)
    assert data["total"] == 0


def test_get_task(client: TestClient, auth_headers: dict[str, str]) -> None:
    resp = client.get("/api/v1/tasks/task_123", headers=auth_headers)
    assert resp.status_code == 200
    assert resp.json()["data"]["task_id"] == "task_123"


# ------------------------------------------------------------------
# Playbooks
# ------------------------------------------------------------------


def test_list_playbooks(client: TestClient, auth_headers: dict[str, str]) -> None:
    resp = client.get("/api/v1/playbooks", headers=auth_headers)
    assert resp.status_code == 200
    assert isinstance(resp.json()["data"]["playbooks"], list)


# ------------------------------------------------------------------
# Status
# ------------------------------------------------------------------


def test_get_status(client: TestClient, auth_headers: dict[str, str]) -> None:
    resp = client.get("/api/v1/status", headers=auth_headers)
    assert resp.status_code == 200
    assert resp.json()["data"]["state"] == "running"


# ------------------------------------------------------------------
# Auth rejection
# ------------------------------------------------------------------


def test_no_auth_401(client: TestClient) -> None:
    resp = client.get("/api/v1/tasks")
    assert resp.status_code == 401


def test_invalid_key_401(client: TestClient) -> None:
    resp = client.get(
        "/api/v1/tasks",
        headers={"Authorization": "Bearer bad_key_xyz"},
    )
    assert resp.status_code == 401


# ------------------------------------------------------------------
# Mesh
# ------------------------------------------------------------------


def test_mesh_nodes(client: TestClient, auth_headers: dict[str, str]) -> None:
    resp = client.get("/api/v1/mesh/nodes", headers=auth_headers)
    assert resp.status_code == 200
    assert isinstance(resp.json()["data"]["nodes"], list)
