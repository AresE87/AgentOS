use reqwest::Client;

use super::spec::AAPMessage;

pub struct AAPClient {
    client: Client,
}

impl AAPClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Send a message to another AAP agent
    pub async fn send_message(
        &self,
        host: &str,
        port: u16,
        message: &AAPMessage,
    ) -> Result<serde_json::Value, String> {
        let url = format!("http://{}:{}/aap/v1/message", host, port);
        let response = self
            .client
            .post(&url)
            .json(message)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| format!("AAP send error: {}", e))?;

        response.json().await.map_err(|e| e.to_string())
    }

    /// Query capabilities of another agent
    pub async fn query_capabilities(
        &self,
        host: &str,
        port: u16,
    ) -> Result<serde_json::Value, String> {
        let url = format!("http://{}:{}/aap/v1/capabilities", host, port);
        let response = self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| format!("AAP query error: {}", e))?;

        response.json().await.map_err(|e| e.to_string())
    }

    /// Check if an agent is alive
    pub async fn health_check(&self, host: &str, port: u16) -> bool {
        let url = format!("http://{}:{}/aap/health", host, port);
        self.client
            .get(&url)
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Send task request to another agent
    pub async fn send_task(
        &self,
        host: &str,
        port: u16,
        sender_id: &str,
        sender_name: &str,
        task: &str,
    ) -> Result<serde_json::Value, String> {
        let msg = AAPMessage::task_request(sender_id, sender_name, task);
        self.send_message(host, port, &msg).await
    }
}
