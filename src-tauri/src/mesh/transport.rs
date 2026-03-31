//! Mesh transport layer — TCP + JSON protocol for task exchange between AgentOS nodes.
//!
//! Uses simple newline-delimited JSON over TCP. No WebSocket dependency needed.
//! Each message is a single JSON line terminated by `\n`.

use crate::brain::Gateway;
use crate::config::Settings;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

use super::protocol::MeshMessage;

/// Start the mesh TCP server that accepts task requests from remote nodes.
///
/// Listens on `0.0.0.0:{port}` and handles each connection in a spawned task.
/// The server runs until `kill_switch` is set to true.
pub async fn start_mesh_server(port: u16, settings: Settings, kill_switch: Arc<AtomicBool>) {
    let listener = match TcpListener::bind(format!("0.0.0.0:{}", port)).await {
        Ok(l) => {
            tracing::info!("Mesh server listening on port {}", port);
            l
        }
        Err(e) => {
            tracing::error!("Failed to bind mesh server on port {}: {}", port, e);
            return;
        }
    };

    loop {
        if kill_switch.load(Ordering::Relaxed) {
            tracing::info!("Mesh server shutting down (kill switch)");
            break;
        }

        // Use tokio::select to check kill_switch periodically
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, addr)) => {
                        tracing::info!("Mesh connection from {}", addr);
                        let settings = settings.clone();
                        let ks = kill_switch.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(stream, &settings, &ks).await {
                                tracing::warn!("Mesh connection error from {}: {}", addr, e);
                            }
                        });
                    }
                    Err(e) => {
                        tracing::warn!("Mesh accept error: {}", e);
                    }
                }
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(2)) => {
                // Check kill switch
                continue;
            }
        }
    }
}

/// Handle a single inbound mesh connection.
///
/// Reads newline-delimited JSON messages, processes them, and sends responses.
async fn handle_connection(
    stream: TcpStream,
    settings: &Settings,
    _kill_switch: &Arc<AtomicBool>,
) -> Result<(), String> {
    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        let n = buf_reader
            .read_line(&mut line)
            .await
            .map_err(|e| e.to_string())?;

        if n == 0 {
            // Connection closed
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        match serde_json::from_str::<MeshMessage>(trimmed) {
            Ok(msg) => {
                let response = process_message(msg, settings).await;
                if let Some(resp) = response {
                    let mut json = serde_json::to_string(&resp).map_err(|e| e.to_string())?;
                    json.push('\n');
                    writer
                        .write_all(json.as_bytes())
                        .await
                        .map_err(|e| e.to_string())?;
                    writer.flush().await.map_err(|e| e.to_string())?;
                }
            }
            Err(e) => {
                tracing::warn!("Invalid mesh message: {}", e);
            }
        }
    }

    Ok(())
}

/// Process an inbound mesh message and optionally return a response.
async fn process_message(msg: MeshMessage, settings: &Settings) -> Option<MeshMessage> {
    match msg {
        MeshMessage::TaskRequest {
            task_id,
            description,
            sender_node,
            ..
        } => {
            tracing::info!(
                "Received task {} from node {}: {}",
                task_id,
                sender_node,
                &description[..description.len().min(80)]
            );

            let start = std::time::Instant::now();

            // Execute the task via the LLM gateway
            let gateway = Gateway::new(settings);
            let result = gateway
                .complete_as_agent(
                    &description,
                    "You are an AgentOS node executing a task received from the mesh network. Complete the task and return the result concisely.",
                    settings,
                )
                .await;

            let duration_ms = start.elapsed().as_millis() as u64;

            match result {
                Ok(resp) => Some(MeshMessage::TaskResult {
                    task_id,
                    success: true,
                    output: resp.content,
                    duration_ms,
                }),
                Err(e) => Some(MeshMessage::TaskResult {
                    task_id,
                    success: false,
                    output: format!("Error: {}", e),
                    duration_ms,
                }),
            }
        }
        MeshMessage::Heartbeat { node_id, .. } => {
            tracing::debug!("Heartbeat from {}", node_id);
            // Respond with our own heartbeat
            Some(MeshMessage::Heartbeat {
                node_id: "self".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                active_tasks: 0,
                load: 0.0,
            })
        }
        MeshMessage::TaskResult { .. } => {
            // We received a result — this is handled by the client side
            None
        }
        _ => {
            tracing::debug!("Unhandled mesh message type");
            None
        }
    }
}

/// Send a task to a remote mesh node and wait for the result.
///
/// Connects via TCP, sends a `TaskRequest` as JSON, reads back the `TaskResult`.
/// Returns the output string on success.
pub async fn send_task(ip: &str, port: u16, description: &str) -> Result<String, String> {
    let addr = format!("{}:{}", ip, port);
    tracing::info!(
        "Sending mesh task to {}: {}",
        addr,
        &description[..description.len().min(80)]
    );

    let stream = TcpStream::connect(&addr)
        .await
        .map_err(|e| format!("Failed to connect to mesh node {}: {}", addr, e))?;

    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);

    let task_id = uuid::Uuid::new_v4().to_string();
    let hostname = whoami::fallible::hostname().unwrap_or_else(|_| "unknown".to_string());

    let request = MeshMessage::TaskRequest {
        task_id: task_id.clone(),
        description: description.to_string(),
        sender_node: hostname,
        priority: 5,
    };

    let mut json = serde_json::to_string(&request).map_err(|e| e.to_string())?;
    json.push('\n');
    writer
        .write_all(json.as_bytes())
        .await
        .map_err(|e| format!("Failed to send task: {}", e))?;
    writer
        .flush()
        .await
        .map_err(|e| format!("Flush error: {}", e))?;

    // Wait for response (with timeout)
    let mut response_line = String::new();
    let read_result = tokio::time::timeout(
        tokio::time::Duration::from_secs(120),
        buf_reader.read_line(&mut response_line),
    )
    .await
    .map_err(|_| "Mesh task timed out after 120 seconds".to_string())?
    .map_err(|e| format!("Read error: {}", e))?;

    if read_result == 0 {
        return Err("Remote node closed connection without response".to_string());
    }

    let resp: MeshMessage = serde_json::from_str(response_line.trim())
        .map_err(|e| format!("Invalid response: {}", e))?;

    match resp {
        MeshMessage::TaskResult {
            success,
            output,
            duration_ms,
            ..
        } => {
            tracing::info!(
                "Mesh task completed: success={}, duration={}ms",
                success,
                duration_ms
            );
            if success {
                Ok(output)
            } else {
                Err(format!("Remote node error: {}", output))
            }
        }
        _ => Err("Unexpected response type from mesh node".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_send_task_connection_refused() {
        // Connecting to a port with nothing listening should fail gracefully
        let result = send_task("127.0.0.1", 59999, "test task").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to connect"));
    }
}
