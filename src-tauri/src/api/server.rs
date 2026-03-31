use axum::{
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};

use super::routes::{self, TaskEntry};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiTask {
    pub task_id: String,
    pub text: String,
}

pub type TaskStore = Arc<RwLock<HashMap<String, TaskEntry>>>;

#[derive(Clone)]
pub struct ApiState {
    pub db_path: String,
    pub task_sender: tokio::sync::mpsc::Sender<ApiTask>,
    pub task_store: TaskStore,
    /// Stripe webhook signing secret (from settings)
    pub stripe_webhook_secret: Option<String>,
    /// Path to settings config file so webhooks can update plan_type
    pub settings_path: Option<String>,
}

pub async fn start_api_server(
    db_path: String,
    port: u16,
) -> Result<(tokio::sync::mpsc::Receiver<ApiTask>, TaskStore), String> {
    start_api_server_with_stripe(db_path, port, None, None).await
}

pub async fn start_api_server_with_stripe(
    db_path: String,
    port: u16,
    stripe_webhook_secret: Option<String>,
    settings_path: Option<String>,
) -> Result<(tokio::sync::mpsc::Receiver<ApiTask>, TaskStore), String> {
    let (tx, rx) = tokio::sync::mpsc::channel::<ApiTask>(128);
    let task_store: TaskStore = Arc::new(RwLock::new(HashMap::new()));

    let state = ApiState {
        db_path,
        task_sender: tx,
        task_store: task_store.clone(),
        stripe_webhook_secret,
        settings_path,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(routes::health))
        .route("/v1/status", get(routes::get_status))
        .route("/v1/message", post(routes::post_message))
        .route("/v1/tasks", get(routes::list_tasks))
        .route("/v1/task/:id", get(routes::get_task))
        .route("/webhooks/stripe", post(routes::stripe_webhook))
        .layer(cors)
        .with_state(state);

    let addr: SocketAddr = format!("0.0.0.0:{}", port)
        .parse()
        .map_err(|e: std::net::AddrParseError| e.to_string())?;

    tracing::info!("Starting public API server on {}", addr);

    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr).await;
        match listener {
            Ok(l) => {
                if let Err(e) = axum::serve(l, app).await {
                    tracing::error!("API server error: {}", e);
                }
            }
            Err(e) => {
                tracing::error!("Failed to bind API server to {}: {}", addr, e);
            }
        }
    });

    Ok((rx, task_store))
}
