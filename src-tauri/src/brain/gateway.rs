use std::time::Instant;
use tracing::{info, warn};
use uuid::Uuid;

use super::providers::Providers;
use super::router::Router;
use super::types::{LLMResponse, Message};
use crate::config::Settings;

pub struct Gateway {
    pub router: Router,
    providers: Providers,
}

impl Gateway {
    pub fn new(_settings: &Settings) -> Self {
        Self {
            router: Router::new(),
            providers: Providers::new(),
        }
    }

    pub async fn complete(
        &self,
        user_text: &str,
        settings: &Settings,
    ) -> Result<LLMResponse, String> {
        self.complete_with_system(user_text, None, settings).await
    }

    pub async fn complete_with_system(
        &self,
        user_text: &str,
        system_prompt: Option<&str>,
        settings: &Settings,
    ) -> Result<LLMResponse, String> {
        let classification = super::classify(user_text);
        let chain = self.router.get_fallback_chain(&classification);

        let mut messages = Vec::new();
        if let Some(sp) = system_prompt {
            messages.push(Message {
                role: "system".to_string(),
                content: sp.to_string(),
            });
        }
        messages.push(Message {
            role: "user".to_string(),
            content: user_text.to_string(),
        });

        for model_entry in &chain {
            let api_key = match model_entry.provider.as_str() {
                "anthropic" if !settings.anthropic_api_key.is_empty() => {
                    &settings.anthropic_api_key
                }
                "openai" if !settings.openai_api_key.is_empty() => &settings.openai_api_key,
                "google" if !settings.google_api_key.is_empty() => &settings.google_api_key,
                _ => continue,
            };

            let start = Instant::now();
            let result = match model_entry.provider.as_str() {
                "anthropic" => {
                    self.providers
                        .call_anthropic(&model_entry.model, &messages, 4096, api_key)
                        .await
                }
                "openai" => {
                    self.providers
                        .call_openai(&model_entry.model, &messages, 4096, api_key)
                        .await
                }
                "google" => {
                    self.providers
                        .call_google(&model_entry.model, &messages, api_key)
                        .await
                }
                _ => continue,
            };

            match result {
                Ok((content, tokens_in, tokens_out)) => {
                    let duration = start.elapsed().as_millis() as u64;
                    let cost = (tokens_in as f64 * model_entry.cost_per_1k_input / 1000.0)
                        + (tokens_out as f64 * model_entry.cost_per_1k_output / 1000.0);

                    info!(
                        model = %model_entry.id,
                        tokens_in,
                        tokens_out,
                        cost,
                        duration_ms = duration,
                        "LLM call succeeded"
                    );

                    return Ok(LLMResponse {
                        task_id: Uuid::new_v4().to_string(),
                        content,
                        model: model_entry.id.clone(),
                        provider: model_entry.provider.clone(),
                        tokens_in,
                        tokens_out,
                        cost,
                        duration_ms: duration,
                    });
                }
                Err(e) => {
                    warn!(model = %model_entry.id, error = %e, "LLM call failed, trying next");
                    continue;
                }
            }
        }

        Err("All LLM providers failed. Check your API keys in Settings.".to_string())
    }

    /// Call LLM with a screenshot (vision/multimodal)
    pub async fn complete_with_vision(
        &self,
        user_text: &str,
        image_b64: &str,
        settings: &Settings,
    ) -> Result<LLMResponse, String> {
        // For vision, prefer models that support it well
        let vision_models = [
            ("anthropic", "claude-sonnet-4-20250514", "anthropic/sonnet"),
            ("openai", "gpt-4o", "openai/gpt4o"),
            ("google", "gemini-2.0-flash", "google/flash"),
        ];

        for (provider, model, model_id) in &vision_models {
            let api_key = match *provider {
                "anthropic" if !settings.anthropic_api_key.is_empty() => {
                    &settings.anthropic_api_key
                }
                "openai" if !settings.openai_api_key.is_empty() => &settings.openai_api_key,
                "google" if !settings.google_api_key.is_empty() => &settings.google_api_key,
                _ => continue,
            };

            let start = Instant::now();
            let result = match *provider {
                "anthropic" => {
                    self.providers
                        .call_anthropic_vision(model, user_text, image_b64, 4096, api_key)
                        .await
                }
                "openai" => {
                    self.providers
                        .call_openai_vision(model, user_text, image_b64, 4096, api_key)
                        .await
                }
                "google" => {
                    self.providers
                        .call_google_vision(model, user_text, image_b64, api_key)
                        .await
                }
                _ => continue,
            };

            match result {
                Ok((content, tokens_in, tokens_out)) => {
                    let duration = start.elapsed().as_millis() as u64;
                    let cost = (tokens_in as f64 * 0.003 / 1000.0)
                        + (tokens_out as f64 * 0.015 / 1000.0);

                    info!(
                        model = model_id,
                        tokens_in,
                        tokens_out,
                        duration_ms = duration,
                        "Vision LLM call succeeded"
                    );

                    return Ok(LLMResponse {
                        task_id: Uuid::new_v4().to_string(),
                        content,
                        model: model_id.to_string(),
                        provider: provider.to_string(),
                        tokens_in,
                        tokens_out,
                        cost,
                        duration_ms: duration,
                    });
                }
                Err(e) => {
                    warn!(model = model_id, error = %e, "Vision call failed, trying next");
                    continue;
                }
            }
        }

        Err("All vision LLM providers failed. Check your API keys.".to_string())
    }

    pub async fn health_check(&self, settings: &Settings) -> serde_json::Value {
        serde_json::json!({
            "anthropic": !settings.anthropic_api_key.is_empty(),
            "openai": !settings.openai_api_key.is_empty(),
            "google": !settings.google_api_key.is_empty(),
        })
    }
}
