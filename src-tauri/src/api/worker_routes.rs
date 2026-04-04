use axum::{
    extract::Path,
    http::StatusCode,
    Json,
};

use super::routes::api_error;

type ApiResult = Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)>;

/// POST /workers/deploy -- deploy a container on this machine
pub async fn deploy_worker(Json(body): Json<serde_json::Value>) -> ApiResult {
    let _image = body
        .get("image")
        .and_then(|v| v.as_str())
        .unwrap_or("agentos-worker:latest");
    let _memory_mb = body
        .get("memory_mb")
        .and_then(|v| v.as_u64())
        .unwrap_or(512) as u32;

    let worker_id = uuid::Uuid::new_v4().to_string();
    let existing = crate::sandbox::WorkerContainer::list_all()
        .await
        .unwrap_or_default();
    let used_ports: Vec<u16> = existing
        .iter()
        .filter_map(|(_, name, _)| {
            // Parse port from container name pattern if available
            let _ = name;
            None
        })
        .collect();
    let port = crate::sandbox::WorkerContainer::next_available_port(&used_ports);

    let container = crate::sandbox::WorkerContainer::start(&worker_id, None, port)
        .await
        .map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "deploy_failed",
                e,
            )
        })?;

    Ok(Json(serde_json::json!({
        "worker_id": worker_id,
        "container_id": container.container_id,
        "ollama_port": port,
    })))
}

/// POST /workers/:id/exec -- execute command in a worker container
pub async fn exec_in_worker(
    Path(worker_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult {
    let command = body
        .get("command")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let (stdout, stderr, exit_code) =
        crate::sandbox::WorkerContainer::exec_command(&worker_id, command)
            .await
            .map_err(|e| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "exec_failed",
                    e,
                )
            })?;

    Ok(Json(serde_json::json!({
        "stdout": stdout,
        "stderr": stderr,
        "exit_code": exit_code,
    })))
}

/// DELETE /workers/:id -- stop a worker container
pub async fn stop_worker(Path(worker_id): Path<String>) -> ApiResult {
    crate::sandbox::WorkerContainer::stop(&worker_id)
        .await
        .map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "stop_failed",
                e,
            )
        })?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// GET /workers/:id/status -- get worker container status
pub async fn get_worker_status(Path(worker_id): Path<String>) -> ApiResult {
    let running = crate::sandbox::WorkerContainer::is_running(&worker_id).await;
    let logs = crate::sandbox::WorkerContainer::get_logs(&worker_id, 20)
        .await
        .unwrap_or_default();

    Ok(Json(serde_json::json!({
        "worker_id": worker_id,
        "running": running,
        "recent_logs": logs,
    })))
}

/// GET /workers/status -- get this node's Docker availability and active workers
pub async fn get_node_status() -> ApiResult {
    let docker_available = crate::sandbox::SandboxManager::is_docker_available().await;
    let workers = crate::sandbox::WorkerContainer::list_all()
        .await
        .unwrap_or_default();

    let node_id = uuid::Uuid::new_v4().to_string();

    Ok(Json(serde_json::json!({
        "node_id": node_id,
        "address": "",
        "docker_available": docker_available,
        "active_workers": workers.len(),
    })))
}
