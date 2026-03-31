use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::auth;
use super::server::ApiState;

fn extract_bearer(headers: &HeaderMap) -> Option<String> {
    let auth = headers.get("authorization")?.to_str().ok()?;
    let token = auth.strip_prefix("Bearer ")?;
    Some(token.to_string())
}

fn validate_auth(state: &ApiState, headers: &HeaderMap) -> Result<(), (StatusCode, String)> {
    let token = extract_bearer(headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            "Missing Authorization header".to_string(),
        )
    })?;

    let conn = rusqlite::Connection::open(&state.db_path)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let valid = auth::validate_api_key(&conn, &token)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    if valid {
        Ok(())
    } else {
        Err((
            StatusCode::UNAUTHORIZED,
            "Invalid or revoked API key".to_string(),
        ))
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
        "version": env!("CARGO_PKG_VERSION"),
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
        return Err((
            StatusCode::BAD_REQUEST,
            "text must not be empty".to_string(),
        ));
    }

    let task_id = uuid::Uuid::new_v4().to_string();

    // Insert task as pending
    {
        let mut store = state.task_store.write().await;
        store.insert(
            task_id.clone(),
            TaskEntry {
                status: "queued".to_string(),
                result: None,
            },
        );
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

// ── C1: Stripe Webhook ────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct WebhookQuery {
    #[serde(default)]
    pub secret: String,
}

/// POST /webhooks/stripe — receives Stripe webhook events.
/// Updates plan_type in the database when checkout completes or subscription is canceled.
pub async fn stripe_webhook(
    headers: HeaderMap,
    State(state): State<ApiState>,
    body: String,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let signature = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Read webhook secret from the db_path-adjacent config
    // For now we rely on the secret being passed via the ApiState
    let webhook_secret = state.stripe_webhook_secret.as_deref().unwrap_or("");

    if !webhook_secret.is_empty() {
        let valid = crate::billing::stripe::StripeClient::verify_webhook_signature(
            &body,
            signature,
            webhook_secret,
        );
        if !valid {
            return Err((
                StatusCode::BAD_REQUEST,
                "Invalid webhook signature".to_string(),
            ));
        }
    }

    // Parse the event and determine plan change
    let plan_change = crate::billing::stripe::StripeClient::parse_webhook_event(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    if let Some(new_plan) = plan_change {
        tracing::info!("Stripe webhook: updating plan to '{}'", new_plan);

        // Update plan_type in the settings file via database
        let conn = rusqlite::Connection::open(&state.db_path)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        // Update plan_type in daily_usage, preserving existing counters
        conn.execute(
            "INSERT INTO daily_usage (date, tasks_count, tokens_used, plan_type) VALUES (date('now'), 0, 0, ?1)
             ON CONFLICT(date) DO UPDATE SET plan_type = ?1",
            rusqlite::params![new_plan],
        )
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        // Also update the settings config file if we have a settings_path
        if let Some(ref settings_path) = state.settings_path {
            if let Ok(content) = std::fs::read_to_string(settings_path) {
                if let Ok(mut settings) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(obj) = settings.as_object_mut() {
                        obj.insert(
                            "plan_type".to_string(),
                            serde_json::Value::String(new_plan.clone()),
                        );
                        if let Ok(json) = serde_json::to_string_pretty(&settings) {
                            let _ = std::fs::write(settings_path, json);
                        }
                    }
                }
            }
        }

        Ok(Json(serde_json::json!({
            "received": true,
            "plan_updated": new_plan,
        })))
    } else {
        Ok(Json(serde_json::json!({
            "received": true,
            "plan_updated": null,
        })))
    }
}
