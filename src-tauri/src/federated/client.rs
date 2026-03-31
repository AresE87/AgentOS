use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedSignal {
    pub key: String,
    pub value: f64,
    pub sample_count: u64,
    pub privacy_applied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedPayload {
    pub node_id: String,
    pub model_name: String,
    pub generated_at: String,
    pub round: u64,
    pub signals: Vec<FederatedSignal>,
    pub excluded_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedConfig {
    pub server_url: String,
    pub model_name: String,
    pub privacy_budget: f64,
    pub min_samples: u32,
    pub node_id: String,
}

impl Default for FederatedConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:9090".into(),
            model_name: "task-routing".into(),
            privacy_budget: 1.0,
            min_samples: 25,
            node_id: format!("node-{}", uuid::Uuid::new_v4().simple()),
        }
    }
}

pub struct FederatedClient {
    client: Client,
    config: FederatedConfig,
    last_round: u64,
    status: String,
    last_payload: Option<FederatedPayload>,
}

impl FederatedClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("reqwest client"),
            config: FederatedConfig::default(),
            last_round: 0,
            status: "idle".into(),
            last_payload: None,
        }
    }

    pub fn configure(&mut self, config: FederatedConfig) {
        self.config = config;
    }

    pub fn get_config(&self) -> &FederatedConfig {
        &self.config
    }

    pub fn build_payload(&mut self, metrics: &[(&str, f64, u64)]) -> FederatedPayload {
        self.status = "aggregating".into();
        self.last_round += 1;
        let epsilon = self.config.privacy_budget.max(0.1);
        let noise = 1.0 / epsilon * 0.001;
        let signals = metrics
            .iter()
            .filter(|(_, _, sample_count)| *sample_count as u32 >= self.config.min_samples)
            .map(|(key, value, sample_count)| FederatedSignal {
                key: (*key).to_string(),
                value: value + noise,
                sample_count: *sample_count,
                privacy_applied: true,
            })
            .collect::<Vec<_>>();

        let payload = FederatedPayload {
            node_id: self.config.node_id.clone(),
            model_name: self.config.model_name.clone(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            round: self.last_round,
            signals,
            excluded_fields: vec![
                "input_text".to_string(),
                "output_text".to_string(),
                "email_body".to_string(),
                "raw_prompt".to_string(),
            ],
        };
        self.last_payload = Some(payload.clone());
        self.status = "ready_to_submit".into();
        payload
    }

    pub async fn submit_payload(
        &mut self,
        payload: &FederatedPayload,
    ) -> Result<serde_json::Value, String> {
        self.status = "submitting".into();
        let url = format!("{}/api/v1/federated/submit", self.config.server_url.trim_end_matches('/'));
        let response = self
            .client
            .post(url)
            .json(payload)
            .send()
            .await
            .map_err(|e| format!("Federated submit error: {}", e))?;

        let status = response.status();
        let parsed = response
            .json::<serde_json::Value>()
            .await
            .unwrap_or_else(|_| serde_json::json!({ "ok": status.is_success() }));
        if !status.is_success() {
            self.status = "submit_failed".into();
            return Err(format!("Federated submit failed with status {}", status));
        }
        self.status = "submitted".into();
        Ok(parsed)
    }

    pub fn get_status(&self) -> serde_json::Value {
        serde_json::json!({
            "status": self.status,
            "last_round": self.last_round,
            "config": self.config,
            "last_payload": self.last_payload,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn start_mock_server() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                let body = r#"{"ok":true,"accepted":2}"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        format!("http://{}", addr)
    }

    #[test]
    fn build_payload_excludes_sensitive_fields_and_applies_privacy() {
        let mut client = FederatedClient::new();
        client.configure(FederatedConfig {
            min_samples: 10,
            ..FederatedConfig::default()
        });

        let payload = client.build_payload(&[
            ("task.success_rate", 0.92, 48),
            ("task.avg_cost", 0.04, 48),
            ("raw_prompt", 99.0, 48),
            ("tiny_sample", 1.0, 2),
        ]);

        assert_eq!(payload.signals.len(), 3);
        assert!(payload.signals.iter().all(|signal| signal.privacy_applied));
        assert!(payload
            .excluded_fields
            .iter()
            .any(|field| field == "input_text"));
    }

    #[tokio::test]
    async fn submit_payload_syncs_to_server() {
        let mut client = FederatedClient::new();
        client.configure(FederatedConfig {
            server_url: start_mock_server(),
            min_samples: 1,
            ..FederatedConfig::default()
        });
        let payload = client.build_payload(&[("task.success_rate", 0.88, 12)]);
        let result = client.submit_payload(&payload).await.unwrap();

        assert_eq!(result["ok"], true);
        assert_eq!(result["accepted"], 2);
        assert_eq!(client.get_status()["status"], "submitted");
    }
}
