use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    pub size: u64,
    pub modified_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalLLMStatus {
    pub available: bool,
    pub url: String,
    pub models: Vec<OllamaModel>,
    pub selected_model: Option<String>,
}

// ── Raw API shapes ──────────────────────────────────────────────

#[derive(Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModelRaw>,
}

#[derive(Deserialize)]
struct OllamaModelRaw {
    name: String,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    modified_at: String,
}

#[derive(Serialize)]
struct OllamaGenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    system: &'a str,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaGenerateResponse {
    response: String,
}

#[derive(Serialize)]
struct OllamaPullRequest<'a> {
    name: &'a str,
    stream: bool,
}

// ── Provider ────────────────────────────────────────────────────

pub struct LocalLLMProvider {
    client: Client,
    pub base_url: String,
}

impl LocalLLMProvider {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
            base_url: base_url.to_string(),
        }
    }

    /// Returns true if Ollama is responding at the configured URL.
    pub async fn is_available(&self) -> bool {
        let url = format!("{}/api/tags", self.base_url);
        match self.client.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Lists models that Ollama has downloaded.
    pub async fn list_models(&self) -> Result<Vec<OllamaModel>, String> {
        let url = format!("{}/api/tags", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Ollama request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Ollama returned status {}", resp.status()));
        }

        let body: OllamaTagsResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse Ollama response: {e}"))?;

        Ok(body
            .models
            .into_iter()
            .map(|m| OllamaModel {
                name: m.name,
                size: m.size,
                modified_at: m.modified_at,
            })
            .collect())
    }

    /// Runs a non-streaming completion via `/api/generate`.
    pub async fn complete(
        &self,
        model: &str,
        prompt: &str,
        system: &str,
    ) -> Result<String, String> {
        let url = format!("{}/api/generate", self.base_url);
        let body = OllamaGenerateRequest {
            model,
            prompt,
            system,
            stream: false,
        };

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Ollama generate request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Ollama generate error {status}: {text}"));
        }

        let gen: OllamaGenerateResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse Ollama generate response: {e}"))?;

        info!(model = %model, "Local LLM completion succeeded");
        Ok(gen.response)
    }

    /// Kicks off a non-blocking model pull (`/api/pull`, `stream: false`).
    pub async fn pull_model(&self, model: &str) -> Result<(), String> {
        let url = format!("{}/api/pull", self.base_url);
        let body = OllamaPullRequest {
            name: model,
            stream: false,
        };

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Ollama pull request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Ollama pull error {status}: {text}"));
        }

        info!(model = %model, "Ollama pull started");
        Ok(())
    }

    /// Checks availability and returns a full status snapshot.
    pub async fn get_status(&self) -> LocalLLMStatus {
        let available = self.is_available().await;
        let models = if available {
            match self.list_models().await {
                Ok(m) => m,
                Err(e) => {
                    warn!("Failed to list Ollama models: {}", e);
                    vec![]
                }
            }
        } else {
            vec![]
        };

        LocalLLMStatus {
            available,
            url: self.base_url.clone(),
            models,
            selected_model: None,
        }
    }
}
