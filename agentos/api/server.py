"""AgentOS Public REST API."""

from __future__ import annotations

import time
from typing import Any

from fastapi import Depends, FastAPI, Query
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel

from agentos.api.auth import APIKeyInfo, verify_api_key
from agentos.utils.logging import get_logger

logger = get_logger("api.server")

app = FastAPI(
    title="AgentOS API",
    version="1.0.0",
    docs_url="/docs",
    redoc_url="/redoc",
)
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)


# ---------------------------------------------------------------------------
# Response models
# ---------------------------------------------------------------------------


class APIResponse(BaseModel):
    data: Any = None
    error: str | None = None
    meta: dict[str, Any] = {}


class TaskCreate(BaseModel):
    text: str
    source: str = "api"
    playbook: str | None = None


# ---------------------------------------------------------------------------
# Tasks
# ---------------------------------------------------------------------------


@app.post("/api/v1/tasks", response_model=APIResponse)
async def create_task(
    body: TaskCreate,
    key: APIKeyInfo = Depends(verify_api_key),
) -> APIResponse:
    """Create and execute a task. Returns task_id immediately."""
    # TODO(AOS-071): Wire to AgentCore
    task_id = f"task_{int(time.time())}"
    return APIResponse(
        data={"task_id": task_id, "status": "pending"},
        meta={"async": True},
    )


@app.get("/api/v1/tasks", response_model=APIResponse)
async def list_tasks(
    page: int = 1,
    per_page: int = 20,
    status: str | None = None,
    key: APIKeyInfo = Depends(verify_api_key),
) -> APIResponse:
    return APIResponse(
        data={"tasks": [], "total": 0},
        meta={"page": page, "per_page": per_page},
    )


@app.get("/api/v1/tasks/{task_id}", response_model=APIResponse)
async def get_task(
    task_id: str,
    key: APIKeyInfo = Depends(verify_api_key),
) -> APIResponse:
    return APIResponse(data={"task_id": task_id, "status": "pending"})


@app.get("/api/v1/tasks/{task_id}/chain", response_model=APIResponse)
async def get_task_chain(
    task_id: str,
    key: APIKeyInfo = Depends(verify_api_key),
) -> APIResponse:
    return APIResponse(data={"task_id": task_id, "subtasks": []})


@app.delete("/api/v1/tasks/{task_id}", response_model=APIResponse)
async def cancel_task(
    task_id: str,
    key: APIKeyInfo = Depends(verify_api_key),
) -> APIResponse:
    return APIResponse(data={"task_id": task_id, "cancelled": True})


# ---------------------------------------------------------------------------
# Playbooks
# ---------------------------------------------------------------------------


@app.get("/api/v1/playbooks", response_model=APIResponse)
async def list_playbooks(
    key: APIKeyInfo = Depends(verify_api_key),
) -> APIResponse:
    return APIResponse(data={"playbooks": []})


@app.post("/api/v1/playbooks/activate", response_model=APIResponse)
async def activate_playbook(
    name: str = Query(...),
    key: APIKeyInfo = Depends(verify_api_key),
) -> APIResponse:
    return APIResponse(data={"activated": name})


@app.get("/api/v1/playbooks/{name}", response_model=APIResponse)
async def get_playbook(
    name: str,
    key: APIKeyInfo = Depends(verify_api_key),
) -> APIResponse:
    return APIResponse(data={"name": name})


# ---------------------------------------------------------------------------
# Agent
# ---------------------------------------------------------------------------


@app.get("/api/v1/status", response_model=APIResponse)
async def get_status(
    key: APIKeyInfo = Depends(verify_api_key),
) -> APIResponse:
    return APIResponse(data={"state": "running", "version": "0.1.0"})


@app.get("/api/v1/health", response_model=APIResponse)
async def health_check() -> APIResponse:
    """Public health check (no auth required)."""
    return APIResponse(data={"healthy": True})


@app.get("/api/v1/usage", response_model=APIResponse)
async def get_usage(
    key: APIKeyInfo = Depends(verify_api_key),
) -> APIResponse:
    return APIResponse(data={"tasks_today": 0, "cost_today": 0.0})


# ---------------------------------------------------------------------------
# Mesh
# ---------------------------------------------------------------------------


@app.get("/api/v1/mesh/nodes", response_model=APIResponse)
async def list_mesh_nodes(
    key: APIKeyInfo = Depends(verify_api_key),
) -> APIResponse:
    return APIResponse(data={"nodes": []})


@app.get("/api/v1/mesh/nodes/{node_id}", response_model=APIResponse)
async def get_mesh_node(
    node_id: str,
    key: APIKeyInfo = Depends(verify_api_key),
) -> APIResponse:
    return APIResponse(data={"node_id": node_id})
