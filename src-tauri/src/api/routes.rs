use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::auth;
use super::server::ApiState;

type ApiResult = Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)>;

fn api_error(
    status: StatusCode,
    code: &str,
    message: impl Into<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        status,
        Json(serde_json::json!({
            "error": code,
            "message": message.into(),
        })),
    )
}

fn extract_bearer(headers: &HeaderMap) -> Option<String> {
    let auth = headers.get("authorization")?.to_str().ok()?;
    let token = auth.strip_prefix("Bearer ")?;
    Some(token.to_string())
}

fn validate_auth(
    state: &ApiState,
    headers: &HeaderMap,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let token = extract_bearer(headers).ok_or_else(|| {
        api_error(
            StatusCode::UNAUTHORIZED,
            "missing_authorization",
            "Missing Authorization header",
        )
    })?;

    let conn = rusqlite::Connection::open(&state.db_path).map_err(|e| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "db_open_failed",
            e.to_string(),
        )
    })?;

    let valid = auth::validate_api_key(&conn, &token).map_err(|e| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "api_key_validation_failed",
            e,
        )
    })?;

    if valid {
        Ok(())
    } else {
        Err(api_error(
            StatusCode::UNAUTHORIZED,
            "invalid_api_key",
            "Invalid or revoked API key",
        ))
    }
}

pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "name": "AgentOS Public API",
        "api_version": "v1",
    }))
}

pub async fn get_status(headers: HeaderMap, State(state): State<ApiState>) -> ApiResult {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEntry {
    pub status: String,
    pub result: Option<String>,
    pub text: String,
    pub created_at: String,
}

pub type TaskStore = Arc<RwLock<HashMap<String, TaskEntry>>>;

pub async fn post_message(
    headers: HeaderMap,
    State(state): State<ApiState>,
    Json(body): Json<MessageRequest>,
) -> ApiResult {
    validate_auth(&state, &headers)?;

    if body.text.trim().is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "text must not be empty",
        ));
    }

    let task_id = uuid::Uuid::new_v4().to_string();
    {
        let mut store = state.task_store.write().await;
        store.insert(
            task_id.clone(),
            TaskEntry {
                status: "queued".to_string(),
                result: None,
                text: body.text.clone(),
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        );
    }

    let api_task = super::server::ApiTask {
        task_id: task_id.clone(),
        text: body.text.clone(),
    };

    state.task_sender.send(api_task).await.map_err(|e| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "task_queue_failed",
            e.to_string(),
        )
    })?;

    Ok(Json(serde_json::json!({
        "task_id": task_id,
        "status": "queued",
    })))
}

#[derive(Debug, Deserialize)]
pub struct ListTasksQuery {
    pub limit: Option<usize>,
    pub status: Option<String>,
}

pub async fn list_tasks(
    headers: HeaderMap,
    State(state): State<ApiState>,
    Query(query): Query<ListTasksQuery>,
) -> ApiResult {
    validate_auth(&state, &headers)?;

    let store = state.task_store.read().await;
    let limit = query.limit.unwrap_or(20);
    let mut tasks: Vec<serde_json::Value> = store
        .iter()
        .filter(|(_, entry)| {
            query
                .status
                .as_ref()
                .map(|status| status == &entry.status)
                .unwrap_or(true)
        })
        .map(|(task_id, entry)| {
            serde_json::json!({
                "task_id": task_id,
                "status": entry.status,
                "text": entry.text,
                "created_at": entry.created_at,
                "has_result": entry.result.is_some(),
            })
        })
        .collect();
    tasks.sort_by(|a, b| b["created_at"].as_str().cmp(&a["created_at"].as_str()));
    tasks.truncate(limit);

    Ok(Json(serde_json::json!({
        "tasks": tasks,
        "total": store.len(),
    })))
}

pub async fn get_task(
    headers: HeaderMap,
    State(state): State<ApiState>,
    axum::extract::Path(task_id): axum::extract::Path<String>,
) -> ApiResult {
    validate_auth(&state, &headers)?;

    let store = state.task_store.read().await;
    if let Some(entry) = store.get(&task_id) {
        Ok(Json(serde_json::json!({
            "task_id": task_id,
            "status": entry.status,
            "text": entry.text,
            "created_at": entry.created_at,
            "result": entry.result,
        })))
    } else {
        Err(api_error(
            StatusCode::NOT_FOUND,
            "task_not_found",
            "Task not found",
        ))
    }
}

pub async fn stripe_webhook(
    headers: HeaderMap,
    State(state): State<ApiState>,
    body: String,
) -> ApiResult {
    let signature = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let webhook_secret = state.stripe_webhook_secret.as_deref().unwrap_or("");

    if !webhook_secret.is_empty() {
        let valid = crate::billing::stripe::StripeClient::verify_webhook_signature(
            &body,
            signature,
            webhook_secret,
        );
        if !valid {
            return Err(api_error(
                StatusCode::BAD_REQUEST,
                "invalid_webhook_signature",
                "Invalid webhook signature",
            ));
        }
    }

    let plan_change = crate::billing::stripe::StripeClient::parse_webhook_event(&body)
        .map_err(|e| api_error(StatusCode::BAD_REQUEST, "invalid_webhook_payload", e))?;

    if let Some(new_plan) = plan_change {
        tracing::info!("Stripe webhook: updating plan to '{}'", new_plan);
        let conn = rusqlite::Connection::open(&state.db_path).map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "db_open_failed",
                e.to_string(),
            )
        })?;

        conn.execute(
            "INSERT INTO daily_usage (date, tasks_count, tokens_used, plan_type) VALUES (date('now'), 0, 0, ?1)
             ON CONFLICT(date) DO UPDATE SET plan_type = ?1",
            rusqlite::params![new_plan],
        )
        .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, "usage_update_failed", e.to_string()))?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::server;
    use axum::http::HeaderValue;
    use tempfile::tempdir;

    fn make_headers(token: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(token) = token {
            headers.insert(
                "authorization",
                HeaderValue::from_str(&format!("Bearer {}", token)).unwrap(),
            );
        }
        headers
    }

    fn test_state() -> (
        ApiState,
        String,
        tempfile::TempDir,
        tokio::sync::mpsc::Receiver<server::ApiTask>,
    ) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("api.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let key = auth::create_api_key(&conn, "test").unwrap().key;
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        let state = ApiState {
            db_path: db_path.to_string_lossy().to_string(),
            task_sender: tx,
            task_store: Arc::new(RwLock::new(HashMap::new())),
            stripe_webhook_secret: None,
            settings_path: None,
        };
        (state, key, dir, rx)
    }

    #[tokio::test]
    async fn health_uses_package_version() {
        let payload = health().await.0;
        assert_eq!(payload["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(payload["api_version"], "v1");
    }

    #[tokio::test]
    async fn status_requires_authorization() {
        let (state, _key, _dir, _rx) = test_state();
        let error = get_status(HeaderMap::new(), State(state))
            .await
            .unwrap_err();
        assert_eq!(error.0, StatusCode::UNAUTHORIZED);
        assert_eq!(error.1 .0["error"], "missing_authorization");
    }

    #[tokio::test]
    async fn message_and_task_listing_follow_real_contract() {
        let (state, key, _dir, _rx) = test_state();
        let headers = make_headers(Some(&key));

        let queued = post_message(
            headers.clone(),
            State(state.clone()),
            Json(MessageRequest {
                text: "ping".to_string(),
            }),
        )
        .await
        .unwrap();
        let task_id = queued.0["task_id"].as_str().unwrap().to_string();

        {
            let mut store = state.task_store.write().await;
            let task = store.get_mut(&task_id).unwrap();
            task.status = "completed".to_string();
            task.result = Some("pong".to_string());
        }

        let list = list_tasks(
            headers.clone(),
            State(state.clone()),
            Query(ListTasksQuery {
                limit: Some(10),
                status: Some("completed".to_string()),
            }),
        )
        .await
        .unwrap();
        assert_eq!(list.0["total"], 1);
        assert_eq!(list.0["tasks"][0]["task_id"], task_id);
        assert_eq!(list.0["tasks"][0]["has_result"], true);

        let task = get_task(headers, State(state), axum::extract::Path(task_id))
            .await
            .unwrap();
        assert_eq!(task.0["status"], "completed");
        assert_eq!(task.0["result"], "pong");
    }
}
