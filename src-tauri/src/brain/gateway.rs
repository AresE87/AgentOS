use std::time::Instant;
use tracing::{info, warn};
use uuid::Uuid;

use super::local_llm::LocalLLMProvider;
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
        let classification = super::classify_smart(user_text, self, settings).await;
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
                    let cost =
                        (tokens_in as f64 * 0.003 / 1000.0) + (tokens_out as f64 * 0.015 / 1000.0);

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

    /// Complete with forced Standard tier — for PC control tasks that need
    /// a capable model regardless of input length/complexity.
    /// Cheap models (haiku, flash, gpt4o-mini) cannot follow complex system prompts.
    pub async fn complete_as_agent(
        &self,
        user_text: &str,
        system_prompt: &str,
        settings: &Settings,
    ) -> Result<LLMResponse, String> {
        use super::classifier::{TaskClassification, TaskTier, TaskType};
        let forced = TaskClassification {
            task_type: TaskType::Text,
            tier: TaskTier::Standard,
            complexity: 3,
            suggested_specialist: "General Assistant".to_string(),
            confidence: 1.0,
            inference_source: "cloud".to_string(),
            local_available: false,
            local_active: false,
            fallback_reason: None,
            latency_ms: 0,
        };
        let chain = self.router.get_fallback_chain(&forced);

        let mut messages = Vec::new();
        messages.push(Message {
            role: "system".to_string(),
            content: system_prompt.to_string(),
        });
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
                        "Agent LLM call succeeded"
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
                    warn!(model = %model_entry.id, error = %e, "Agent LLM call failed, trying next");
                    continue;
                }
            }
        }

        Err("All LLM providers failed for agent task. Check your API keys in Settings.".to_string())
    }

    /// Complete using forced Cheap tier — for internal classification calls.
    /// Uses the cheapest models (Haiku/Flash/GPT4o-mini) to minimize cost.
    pub async fn complete_cheap(
        &self,
        prompt: &str,
        settings: &Settings,
    ) -> Result<LLMResponse, String> {
        use super::classifier::{TaskClassification, TaskTier, TaskType};
        let forced = TaskClassification {
            task_type: TaskType::Text,
            tier: TaskTier::Cheap,
            complexity: 1,
            suggested_specialist: "General Assistant".to_string(),
            confidence: 1.0,
            inference_source: "cloud".to_string(),
            local_available: false,
            local_active: false,
            fallback_reason: None,
            latency_ms: 0,
        };
        let chain = self.router.get_fallback_chain(&forced);

        let messages = vec![Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        }];

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
                        .call_anthropic(&model_entry.model, &messages, 256, api_key)
                        .await
                }
                "openai" => {
                    self.providers
                        .call_openai(&model_entry.model, &messages, 256, api_key)
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
                        "Cheap LLM call succeeded (classifier)"
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
                    warn!(model = %model_entry.id, error = %e, "Cheap LLM call failed, trying next");
                    continue;
                }
            }
        }

        Err("All cheap LLM providers failed — no API keys configured.".to_string())
    }

    /// Complete an LLM call with tool definitions (for agentic loop).
    /// Returns raw JSON response in Anthropic-normalized format.
    pub async fn complete_with_tools(
        &self,
        messages: &[serde_json::Value],
        tools: &[serde_json::Value],
        system_prompt: &str,
        settings: &Settings,
    ) -> Result<serde_json::Value, String> {
        // Try Anthropic first, then OpenAI
        if !settings.anthropic_api_key.is_empty() {
            return Providers::call_anthropic_with_tools(
                &settings.anthropic_api_key,
                "claude-sonnet-4-20250514",
                messages,
                tools,
                Some(system_prompt),
                4096,
            )
            .await;
        }

        if !settings.openai_api_key.is_empty() {
            // Build OpenAI-style messages (inject system prompt)
            let mut oai_messages = vec![serde_json::json!({
                "role": "system",
                "content": system_prompt
            })];
            oai_messages.extend_from_slice(messages);

            return Providers::call_openai_with_tools(
                &settings.openai_api_key,
                "gpt-4o",
                &oai_messages,
                tools,
                4096,
            )
            .await;
        }

        Err("No LLM API key configured for tool-use calls".into())
    }

    pub async fn health_check(&self, settings: &Settings) -> serde_json::Value {
        serde_json::json!({
            "anthropic": !settings.anthropic_api_key.is_empty(),
            "openai": !settings.openai_api_key.is_empty(),
            "google": !settings.google_api_key.is_empty(),
        })
    }

    /// Try local Ollama first (when enabled), then fall back to cloud.
    /// Uses `complete_with_system` for the cloud path.
    pub async fn complete_with_local_fallback(
        &self,
        user_text: &str,
        system_prompt: Option<&str>,
        settings: &Settings,
    ) -> Result<LLMResponse, String> {
        if settings.use_local_llm {
            let provider = LocalLLMProvider::new(&settings.local_llm_url);
            let system = system_prompt.unwrap_or("");
            match provider
                .complete(&settings.local_model, user_text, system)
                .await
            {
                Ok(content) => {
                    info!(
                        model = %settings.local_model,
                        "Local LLM (Ollama) completion used"
                    );
                    return Ok(LLMResponse {
                        task_id: Uuid::new_v4().to_string(),
                        content,
                        model: format!("ollama/{}", settings.local_model),
                        provider: "ollama".to_string(),
                        tokens_in: 0,
                        tokens_out: 0,
                        cost: 0.0,
                        duration_ms: 0,
                    });
                }
                Err(e) => {
                    warn!(
                        model = %settings.local_model,
                        error = %e,
                        "Local LLM failed — falling back to cloud"
                    );
                }
            }
        }
        self.complete_with_system(user_text, system_prompt, settings)
            .await
    }
}
