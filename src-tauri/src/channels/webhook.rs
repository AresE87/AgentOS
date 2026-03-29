use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};

use super::whatsapp::{VerifyQuery, WebhookPayload, WhatsAppChannel};

#[derive(Clone)]
pub struct WebhookState {
    pub verify_token: String,
    pub message_tx: mpsc::Sender<(String, String)>, // (from, text)
}

/// GET /webhook/whatsapp -- Meta verification handshake
async fn verify_webhook(
    Query(params): Query<VerifyQuery>,
    State(state): State<Arc<WebhookState>>,
) -> Result<String, StatusCode> {
    info!("WhatsApp webhook verification request");
    WhatsAppChannel::verify_webhook(&params, &state.verify_token)
        .map_err(|e| {
            warn!("WhatsApp webhook verification failed: {}", e);
            StatusCode::FORBIDDEN
        })
}

/// POST /webhook/whatsapp -- Incoming messages from Meta
async fn receive_webhook(
    State(state): State<Arc<WebhookState>>,
    Json(payload): Json<WebhookPayload>,
) -> StatusCode {
    let messages = WhatsAppChannel::extract_messages(&payload);
    for (from, text) in messages {
        info!(from = %from, "WhatsApp message received");
        if let Err(e) = state.message_tx.send((from, text)).await {
            warn!("Failed to forward WhatsApp message: {}", e);
        }
    }
    StatusCode::OK
}

/// Start the WhatsApp webhook HTTP server on the given port
pub async fn start_webhook_server(
    port: u16,
    verify_token: String,
    message_tx: mpsc::Sender<(String, String)>,
) -> Result<(), String> {
    let state = Arc::new(WebhookState {
        verify_token,
        message_tx,
    });

    let app = Router::new()
        .route("/webhook/whatsapp", get(verify_webhook))
        .route("/webhook/whatsapp", post(receive_webhook))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    info!("Starting WhatsApp webhook server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Failed to bind webhook port {}: {}", port, e))?;

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            warn!("WhatsApp webhook server error: {}", e);
        }
    });

    Ok(())
}
