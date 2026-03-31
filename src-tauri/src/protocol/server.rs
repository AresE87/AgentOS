use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tokio::sync::mpsc;

use super::spec::{AAPMessage, AAPMessageType, AAP_VERSION};

#[derive(Clone)]
pub struct AAPServerState {
    pub node_id: String,
    pub node_name: String,
    pub capabilities: serde_json::Value,
    pub task_tx: mpsc::Sender<AAPMessage>,
}

pub struct AAPServer;

impl AAPServer {
    pub async fn start(
        port: u16,
        state: AAPServerState,
    ) -> Result<mpsc::Receiver<AAPMessage>, String> {
        let (tx, rx) = mpsc::channel(100);
        let state = Arc::new(AAPServerState {
            task_tx: tx,
            ..state
        });

        let app = Router::new()
            .route("/aap/health", get(health))
            .route("/aap/v1/message", post(handle_message))
            .route("/aap/v1/capabilities", get(get_capabilities))
            .with_state(state);

        let addr = format!("0.0.0.0:{}", port);
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .map_err(|e| e.to_string())?;

        tokio::spawn(async move {
            axum::serve(listener, app).await.ok();
        });

        Ok(rx)
    }
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "protocol": "AAP",
        "version": AAP_VERSION,
        "status": "ok"
    }))
}

async fn handle_message(
    State(state): State<Arc<AAPServerState>>,
    Json(msg): Json<AAPMessage>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match msg.msg_type {
        AAPMessageType::TaskRequest => {
            state
                .task_tx
                .send(msg.clone())
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            Ok(Json(serde_json::json!({
                "status": "accepted",
                "trace_id": msg.trace_id
            })))
        }
        AAPMessageType::CapabilityQuery => {
            let response = AAPMessage::capability_response(
                &state.node_id,
                &state.node_name,
                state.capabilities.clone(),
            );
            Ok(Json(serde_json::to_value(response).unwrap()))
        }
        AAPMessageType::Heartbeat => Ok(Json(serde_json::json!({"status": "ok"}))),
        _ => Ok(Json(serde_json::json!({"status": "received"}))),
    }
}

async fn get_capabilities(State(state): State<Arc<AAPServerState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "node_id": state.node_id,
        "node_name": state.node_name,
        "protocol_version": AAP_VERSION,
        "capabilities": state.capabilities
    }))
}
