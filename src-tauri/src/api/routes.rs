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

    let webhook_secret = state.stripe_webhook_secret.as_deref().unwrap_or("");
    if webhook_secret.is_empty() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "Stripe webhook secret is not configured".to_string(),
        ));
    }

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

    let config_settings = load_api_settings(state.settings_path.as_deref());
    let plan_change = crate::billing::stripe::StripeClient::parse_webhook_event(
        &body,
        config_settings
            .as_ref()
            .map(|settings| settings.stripe_price_id_pro.as_str())
            .filter(|value| !value.is_empty()),
        config_settings
            .as_ref()
            .map(|settings| settings.stripe_price_id_team.as_str())
            .filter(|value| !value.is_empty()),
    )
    .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    if let Some(change) = plan_change {
        tracing::info!(
            plan = %change.plan_type,
            customer_id = ?change.customer_id,
            "Stripe webhook: updating billing state"
        );

        let db = crate::memory::Database::new(std::path::Path::new(&state.db_path))
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        db.set_billing_state(&change.plan_type, change.customer_id.as_deref())
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if let Some(mut settings) = config_settings {
            settings.set("plan_type", &change.plan_type);
            if let Some(customer_id) = change.customer_id.as_deref() {
                settings.set("stripe_customer_id", customer_id);
            }
            settings
                .save()
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }

        Ok(Json(serde_json::json!({
            "received": true,
            "plan_updated": change.plan_type,
            "customer_updated": change.customer_id,
        })))
    } else {
        Ok(Json(serde_json::json!({
            "received": true,
            "plan_updated": null,
        })))
    }
}

fn load_api_settings(settings_path: Option<&str>) -> Option<crate::config::Settings> {
    let path = settings_path?;
    let app_dir = std::path::Path::new(path).parent()?;
    Some(crate::config::Settings::load(app_dir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn compute_signature(secret: &str, timestamp: i64, body: &str) -> String {
        let payload = format!("{}.{}", timestamp, body);
        let digest = compute_hmac_sha256(secret.as_bytes(), payload.as_bytes());
        let hex = digest
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect::<String>();
        format!("t={},v1={}", timestamp, hex)
    }

    fn compute_hmac_sha256(secret: &[u8], payload: &[u8]) -> Vec<u8> {
        const BLOCK_SIZE: usize = 64;
        let mut key = [0_u8; BLOCK_SIZE];

        if secret.len() > BLOCK_SIZE {
            let digest = Sha256::digest(secret);
            key[..digest.len()].copy_from_slice(&digest);
        } else {
            key[..secret.len()].copy_from_slice(secret);
        }

        let mut inner_pad = [0_u8; BLOCK_SIZE];
        let mut outer_pad = [0_u8; BLOCK_SIZE];
        for (index, byte) in key.iter().enumerate() {
            inner_pad[index] = byte ^ 0x36;
            outer_pad[index] = byte ^ 0x5c;
        }

        let mut inner = Sha256::new();
        inner.update(inner_pad);
        inner.update(payload);
        let inner_hash = inner.finalize();

        let mut outer = Sha256::new();
        outer.update(outer_pad);
        outer.update(inner_hash);
        outer.finalize().to_vec()
    }

    fn test_state(secret: &str, price_id_pro: &str) -> (tempfile::TempDir, ApiState) {
        let dir = tempfile::tempdir().unwrap();
        let mut settings = crate::config::Settings::load(dir.path());
        settings.set("stripe_webhook_secret", secret);
        settings.set("stripe_price_id_pro", price_id_pro);
        settings.save().unwrap();

        let db_path = dir.path().join("agentos.db");
        crate::memory::Database::new(&db_path).unwrap();

        let (tx, _rx) = tokio::sync::mpsc::channel(4);
        let state = ApiState {
            db_path: db_path.to_string_lossy().to_string(),
            task_sender: tx,
            task_store: Arc::new(RwLock::new(HashMap::new())),
            stripe_webhook_secret: Some(secret.to_string()),
            settings_path: Some(dir.path().join("config.json").to_string_lossy().to_string()),
        };

        (dir, state)
    }

    #[tokio::test]
    async fn stripe_webhook_persists_plan_and_customer() {
        let secret = "whsec_test";
        let (_dir, state) = test_state(secret, "price_pro");
        let body = r#"{
            "id":"evt_1",
            "type":"customer.subscription.updated",
            "data":{"object":{
                "customer":"cus_123",
                "status":"active",
                "items":{"data":[{"price":{"id":"price_pro"}}]}
            }}
        }"#;
        let timestamp = chrono::Utc::now().timestamp();

        let mut headers = HeaderMap::new();
        headers.insert(
            "stripe-signature",
            compute_signature(secret, timestamp, body).parse().unwrap(),
        );

        let response = stripe_webhook(headers, State(state.clone()), body.to_string())
            .await
            .unwrap();

        assert_eq!(response.0["plan_updated"], "pro");
        assert_eq!(response.0["customer_updated"], "cus_123");

        let db = crate::memory::Database::new(std::path::Path::new(&state.db_path)).unwrap();
        let billing = db.get_billing_state().unwrap();
        assert_eq!(billing.plan_type, "pro");
        assert_eq!(billing.stripe_customer_id, "cus_123");

        let settings = load_api_settings(state.settings_path.as_deref()).unwrap();
        assert_eq!(settings.plan_type, "pro");
        assert_eq!(settings.stripe_customer_id, "cus_123");
    }

    #[tokio::test]
    async fn stripe_webhook_rejects_invalid_signature() {
        let secret = "whsec_test";
        let (_dir, state) = test_state(secret, "price_pro");
        let body = r#"{"id":"evt_1","type":"checkout.session.completed","data":{"object":{"metadata":{"plan":"pro"}}}}"#;

        let mut headers = HeaderMap::new();
        headers.insert("stripe-signature", "t=1,v1=deadbeef".parse().unwrap());

        let err = stripe_webhook(headers, State(state), body.to_string())
            .await
            .unwrap_err();

        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        assert_eq!(err.1, "Invalid webhook signature");
    }
}
