use serde::{Deserialize, Serialize};

/// R92: Weight delta for federated learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightDelta {
    pub layer_name: String,
    pub delta: Vec<f64>,
    pub noise_added: bool,
}

/// R92: Federated learning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedConfig {
    pub server_url: String,
    pub model_name: String,
    pub privacy_budget: f64,
    pub min_samples: u32,
}

impl Default for FederatedConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:9090".into(),
            model_name: "default".into(),
            privacy_budget: 1.0,
            min_samples: 100,
        }
    }
}

/// R92: Client for federated learning
pub struct FederatedClient {
    config: FederatedConfig,
    last_round: u64,
    status: String,
}

impl FederatedClient {
    pub fn new() -> Self {
        Self {
            config: FederatedConfig::default(),
            last_round: 0,
            status: "idle".into(),
        }
    }

    pub fn configure(&mut self, config: FederatedConfig) {
        self.config = config;
    }

    pub fn get_config(&self) -> &FederatedConfig {
        &self.config
    }

    /// Stub: train locally on data samples and return weight deltas
    pub fn train_local(&mut self, _data: &[Vec<f64>]) -> Vec<WeightDelta> {
        self.status = "training".into();
        self.last_round += 1;

        // Stub: produce dummy weight deltas
        let deltas = vec![
            WeightDelta {
                layer_name: "dense_1".into(),
                delta: vec![0.001, -0.002, 0.0015],
                noise_added: false,
            },
            WeightDelta {
                layer_name: "dense_2".into(),
                delta: vec![-0.0005, 0.003],
                noise_added: false,
            },
        ];

        self.status = "trained".into();
        deltas
    }

    /// Add differential privacy noise to weight deltas
    pub fn add_privacy_noise(&self, deltas: Vec<WeightDelta>, epsilon: f64) -> Vec<WeightDelta> {
        let noise_scale = 1.0 / epsilon;
        deltas.into_iter().map(|mut d| {
            d.delta = d.delta.iter().map(|v| {
                // Simple Laplace-like noise stub
                v + noise_scale * 0.001
            }).collect();
            d.noise_added = true;
            d
        }).collect()
    }

    /// Submit deltas to the federated server (stub)
    pub fn submit_deltas(&self, deltas: &[WeightDelta]) -> Result<serde_json::Value, String> {
        // In production this would POST to self.config.server_url
        Ok(serde_json::json!({
            "ok": true,
            "round": self.last_round,
            "deltas_count": deltas.len(),
            "server": self.config.server_url,
        }))
    }

    /// Get the global model from the server (stub)
    pub fn get_global_model(&self) -> Result<serde_json::Value, String> {
        Ok(serde_json::json!({
            "model_name": self.config.model_name,
            "round": self.last_round,
            "status": "available",
        }))
    }

    pub fn get_status(&self) -> serde_json::Value {
        serde_json::json!({
            "status": self.status,
            "last_round": self.last_round,
            "config": self.config,
        })
    }
}
