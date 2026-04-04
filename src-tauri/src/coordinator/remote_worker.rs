use serde::{Deserialize, Serialize};
use reqwest::Client;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteWorkerHost {
    pub node_id: String,
    pub address: String,
    pub docker_available: bool,
    pub active_workers: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteWorkerResult {
    pub worker_id: String,
    pub container_id: String,
    pub ollama_port: u16,
}

pub struct RemoteWorkerManager {
    client: Client,
}

impl RemoteWorkerManager {
    pub fn new() -> Self {
        Self { client: Client::new() }
    }

    /// Deploy a worker container on a remote AgentOS node
    pub async fn deploy(
        &self,
        node_address: &str,
        image: &str,
        memory_mb: u32,
        cpu: f64,
    ) -> Result<RemoteWorkerResult, String> {
        let url = format!("http://{}/workers/deploy", node_address);
        let body = serde_json::json!({
            "image": image,
            "memory_mb": memory_mb,
            "cpu": cpu,
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("Deploy failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Remote deploy failed: HTTP {}", response.status()));
        }

        response.json().await.map_err(|e| e.to_string())
    }

    /// Execute a command in a remote worker container
    pub async fn exec(
        &self,
        node_address: &str,
        worker_id: &str,
        command: &str,
    ) -> Result<(String, String, i32), String> {
        let url = format!("http://{}/workers/{}/exec", node_address, worker_id);
        let body = serde_json::json!({ "command": command });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

        Ok((
            json.get("stdout")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            json.get("stderr")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            json.get("exit_code")
                .and_then(|v| v.as_i64())
                .unwrap_or(-1) as i32,
        ))
    }

    /// Get status of a remote worker
    pub async fn status(
        &self,
        node_address: &str,
        worker_id: &str,
    ) -> Result<serde_json::Value, String> {
        let url = format!("http://{}/workers/{}/status", node_address, worker_id);
        let response = self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        response.json().await.map_err(|e| e.to_string())
    }

    /// Stop a remote worker
    pub async fn stop(
        &self,
        node_address: &str,
        worker_id: &str,
    ) -> Result<(), String> {
        let url = format!("http://{}/workers/{}", node_address, worker_id);
        self.client
            .delete(&url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Check if a remote node has Docker available
    pub async fn check_node(
        &self,
        node_address: &str,
    ) -> Result<RemoteWorkerHost, String> {
        let url = format!("http://{}/workers/status", node_address);
        let response = self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        response.json().await.map_err(|e| e.to_string())
    }
}
