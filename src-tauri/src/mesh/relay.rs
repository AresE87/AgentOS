use serde::{Deserialize, Serialize};
use reqwest::Client;
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
