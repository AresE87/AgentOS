use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayConfig {
    pub server_url: String,
    pub auth_token: String,
    pub node_id: String,
}

pub struct RelayClient {
    client: Client,
    config: RelayConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayNode {
    pub node_id: String,
    pub display_name: String,
    pub is_online: bool,
    pub last_seen: String,
    pub is_cloud: bool,
}

impl RelayClient {
    pub fn new(config: RelayConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    pub fn config(&self) -> &RelayConfig {
        &self.config
    }

    /// Register this node with the relay server
    pub async fn register(&self, display_name: &str) -> Result<(), String> {
        let url = format!("{}/api/v1/nodes/register", self.config.server_url);
        self.client
            .post(&url)
            .bearer_auth(&self.config.auth_token)
            .json(&serde_json::json!({
                "node_id": self.config.node_id,
                "display_name": display_name,
                "capabilities": {}
            }))
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| format!("Relay register error: {}", e))?;
        Ok(())
    }

    /// Send heartbeat to relay
    pub async fn heartbeat(&self) -> Result<(), String> {
        let url = format!(
            "{}/api/v1/nodes/{}/heartbeat",
            self.config.server_url, self.config.node_id
        );
        self.client
            .post(&url)
            .bearer_auth(&self.config.auth_token)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| format!("Relay heartbeat error: {}", e))?;
        Ok(())
    }

    /// List all nodes connected to relay
    pub async fn list_nodes(&self) -> Result<Vec<RelayNode>, String> {
        let url = format!("{}/api/v1/nodes", self.config.server_url);
        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.config.auth_token)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| format!("Relay list error: {}", e))?;

        if response.status().is_success() {
            let nodes: Vec<RelayNode> = response.json().await.map_err(|e| e.to_string())?;
            Ok(nodes)
        } else {
            Ok(vec![])
        }
    }

    /// Send task to remote node via relay
    pub async fn send_task(&self, target_node_id: &str, task: &str) -> Result<String, String> {
        let url = format!("{}/api/v1/tasks", self.config.server_url);
        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.config.auth_token)
            .json(&serde_json::json!({
                "from_node": self.config.node_id,
                "to_node": target_node_id,
                "task": task
            }))
            .timeout(Duration::from_secs(15))
            .send()
            .await
            .map_err(|e| format!("Relay send error: {}", e))?;

        let result: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        Ok(result
            .get("task_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string())
    }

    /// Poll for incoming tasks from relay
    pub async fn poll_tasks(&self) -> Result<Vec<serde_json::Value>, String> {
        let url = format!(
            "{}/api/v1/nodes/{}/tasks",
            self.config.server_url, self.config.node_id
        );
        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.config.auth_token)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| format!("Relay poll error: {}", e))?;

        if response.status().is_success() {
            let tasks: Vec<serde_json::Value> = response.json().await.map_err(|e| e.to_string())?;
            Ok(tasks)
        } else {
            Ok(vec![])
        }
    }

    /// Check if relay server is reachable
    pub async fn is_available(&self) -> bool {
        let url = format!("{}/health", self.config.server_url);
        self.client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;

    #[derive(Default, Clone)]
    struct RelayRequests {
        paths: Arc<Mutex<Vec<String>>>,
    }

    fn start_mock_relay() -> (String, RelayRequests) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let requests = RelayRequests::default();
        let requests_clone = requests.clone();

        thread::spawn(move || {
            for _ in 0..6 {
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                let mut buf = [0u8; 4096];
                let read = stream.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..read]).to_string();
                let line = req.lines().next().unwrap_or_default().to_string();
                if !line.is_empty() {
                    requests_clone.paths.lock().unwrap().push(line.clone());
                }

                let (status, body) = if line.contains("POST /api/v1/nodes/register") {
                    ("200 OK", r#"{"ok":true}"#)
                } else if line.contains("POST /api/v1/nodes/test-node/heartbeat") {
                    ("200 OK", r#"{"ok":true}"#)
                } else if line.contains("GET /api/v1/nodes/test-node/tasks") {
                    (
                        "200 OK",
                        r#"[{"task_id":"relay-task-1","task":"classify invoice"}]"#,
                    )
                } else if line.contains("GET /api/v1/nodes ") || line.contains("GET /api/v1/nodes HTTP") {
                    (
                        "200 OK",
                        r#"[
                            {"node_id":"test-node","display_name":"Local AgentOS","is_online":true,"last_seen":"2026-03-31T00:00:00Z","is_cloud":false},
                            {"node_id":"relay-eu","display_name":"Relay EU","is_online":true,"last_seen":"2026-03-31T00:00:00Z","is_cloud":true}
                        ]"#,
                    )
                } else if line.contains("POST /api/v1/tasks") {
                    ("200 OK", r#"{"task_id":"relay-job-42"}"#)
                } else if line.contains("GET /health") {
                    ("200 OK", r#"{"status":"ok"}"#)
                } else {
                    ("404 Not Found", r#"{"error":"not found"}"#)
                };

                let response = format!(
                    "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    status,
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });

        (format!("http://{}", addr), requests)
    }

    fn demo_client(server_url: String) -> RelayClient {
        RelayClient::new(RelayConfig {
            server_url,
            auth_token: "test-token".to_string(),
            node_id: "test-node".to_string(),
        })
    }

    #[tokio::test]
    async fn relay_client_registers_lists_sends_and_polls() {
        let (server_url, requests) = start_mock_relay();
        let client = demo_client(server_url);

        client.register("Local AgentOS").await.unwrap();
        client.heartbeat().await.unwrap();
        let nodes = client.list_nodes().await.unwrap();
        let task_id = client.send_task("relay-eu", "summarize docs").await.unwrap();
        let polled = client.poll_tasks().await.unwrap();
        let available = client.is_available().await;

        assert_eq!(nodes.len(), 2);
        assert!(nodes.iter().any(|node| !node.is_cloud));
        assert!(nodes.iter().any(|node| node.is_cloud));
        assert_eq!(task_id, "relay-job-42");
        assert_eq!(polled[0]["task_id"], "relay-task-1");
        assert!(available);

        let seen = requests.paths.lock().unwrap().join("\n");
        assert!(seen.contains("POST /api/v1/nodes/register"));
        assert!(seen.contains("POST /api/v1/nodes/test-node/heartbeat"));
        assert!(seen.contains("GET /api/v1/nodes HTTP/1.1"));
        assert!(seen.contains("POST /api/v1/tasks"));
        assert!(seen.contains("GET /api/v1/nodes/test-node/tasks"));
        assert!(seen.contains("GET /health HTTP/1.1"));
    }
}
