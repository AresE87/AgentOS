use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::server::ApiState;
use super::auth;

fn extract_bearer(headers: &HeaderMap) -> Option<String> {
    let auth = headers.get("authorization")?.to_str().ok()?;
    let token = auth.strip_prefix("Bearer ")?;
    Some(token.to_string())
}

fn validate_auth(state: &ApiState, headers: &HeaderMap) -> Result<(), (StatusCode, String)> {
    let token = extract_bearer(headers)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Missing Authorization header".to_string()))?;

    let conn = rusqlite::Connection::open(&state.db_path)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let valid = auth::validate_api_key(&conn, &token)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    if valid {
        Ok(())
    } else {
        Err((StatusCode::UNAUTHORIZED, "Invalid or revoked API key".to_string()))
    }
}

// GET /health — no auth required
pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "version": "0.1.0",
        "name": "AgentOS Public API",
    }))
}

// GET /v1/status — auth required
pub async fn get_status(
    headers: HeaderMap,
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    validate_auth(&state, &headers)?;

    Ok(Json(serde_json::json!({
        "status": "running",
        "api_version": "v1",
        "tasks_queued": state.task_store.read().await.len(),
    })))
}

#[derive(Debug, Deserialize)]
pub struct MessageRequest {
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub task_id: String,
    pub status: String,
}

// POST /v1/message — auth required, async (queues task)
pub async fn post_message(
    headers: HeaderMap,
    State(state): State<ApiState>,
    Json(body): Json<MessageRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    validate_auth(&state, &headers)?;

    if body.text.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "text must not be empty".to_string()));
    }

    let task_id = uuid::Uuid::new_v4().to_string();

    // Insert task as pending
    {
        let mut store = state.task_store.write().await;
        store.insert(task_id.clone(), TaskEntry {
            status: "queued".to_string(),
            result: None,
        });
    }

    // Send to task sender channel
    let api_task = super::server::ApiTask {
        task_id: task_id.clone(),
        text: body.text.clone(),
    };

    state
        .task_sender
        .send(api_task)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({
        "task_id": task_id,
        "status": "queued",
    })))
}

// GET /v1/task/:id — auth required
pub async fn get_task(
    headers: HeaderMap,
    State(state): State<ApiState>,
    axum::extract::Path(task_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    validate_auth(&state, &headers)?;

    let store = state.task_store.read().await;
    if let Some(entry) = store.get(&task_id) {
        Ok(Json(serde_json::json!({
            "task_id": task_id,
            "status": entry.status,
            "result": entry.result,
        })))
    } else {
        Err((StatusCode::NOT_FOUND, "Task not found".to_string()))
    }
}

#[derive(Debug, Clone)]
pub struct TaskEntry {
    pub status: String,
    pub result: Option<String>,
}

pub type TaskStore = Arc<RwLock<HashMap<String, TaskEntry>>>;
