use reqwest::Client;
use serde_json::json;

use super::types::Message;

pub struct Providers {
    client: Client,
}

impl Providers {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn call_anthropic(
        &self,
        model: &str,
        messages: &[Message],
        max_tokens: u32,
        api_key: &str,
    ) -> Result<(String, u32, u32), Box<dyn std::error::Error + Send + Sync>> {
        // Anthropic: system prompt goes in top-level "system" field, not in messages
        let system_prompt: Option<String> = messages
            .iter()
            .find(|m| m.role == "system")
            .map(|m| m.content.clone());
        let user_messages: Vec<_> = messages
            .iter()
            .filter(|m| m.role != "system")
            .map(|m| json!({ "role": m.role, "content": m.content }))
            .collect();

        let mut body = json!({
            "model": model,
            "max_tokens": max_tokens,
            "messages": user_messages,
        });
        // Use structured system prompt with cache_control for prompt caching
        if let Some(sp) = system_prompt {
            body["system"] = json!([{
                "type": "text",
                "text": sp,
                "cache_control": {"type": "ephemeral"}
            }]);
        }

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("anthropic-beta", "prompt-caching-2024-07-31")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let data: serde_json::Value = resp.json().await?;

        if !status.is_success() {
            let err_msg = data["error"]["message"]
                .as_str()
                .unwrap_or("Unknown Anthropic API error");
            return Err(err_msg.to_string().into());
        }

        let content = data["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let tokens_in = data["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32;
        let tokens_out = data["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32;

        // Log prompt cache performance
        let cache_creation = data["usage"]["cache_creation_input_tokens"].as_u64().unwrap_or(0);
        let cache_read = data["usage"]["cache_read_input_tokens"].as_u64().unwrap_or(0);
        if cache_read > 0 {
            info!("Prompt cache hit: {} tokens read from cache", cache_read);
        } else if cache_creation > 0 {
            info!("Prompt cache miss: {} tokens written to cache", cache_creation);
        }

        Ok((content, tokens_in, tokens_out))
    }

    pub async fn call_openai(
        &self,
        model: &str,
        messages: &[Message],
        max_tokens: u32,
        api_key: &str,
    ) -> Result<(String, u32, u32), Box<dyn std::error::Error + Send + Sync>> {
        let body = json!({
            "model": model,
            "max_tokens": max_tokens,
            "messages": messages.iter().map(|m| json!({
                "role": m.role,
                "content": m.content,
            })).collect::<Vec<_>>(),
        });

        let resp = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let data: serde_json::Value = resp.json().await?;

        if !status.is_success() {
            let err_msg = data["error"]["message"]
                .as_str()
                .unwrap_or("Unknown OpenAI API error");
            return Err(err_msg.to_string().into());
        }

        let content = data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let tokens_in = data["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32;
        let tokens_out = data["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32;

        Ok((content, tokens_in, tokens_out))
    }

    pub async fn call_google(
        &self,
        model: &str,
        messages: &[Message],
        api_key: &str,
    ) -> Result<(String, u32, u32), Box<dyn std::error::Error + Send + Sync>> {
        let contents: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| {
                json!({
                    "role": if m.role == "assistant" { "model" } else { "user" },
                    "parts": [{ "text": m.content }],
                })
            })
            .collect();

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, api_key
        );

        let resp = self
            .client
            .post(&url)
            .json(&json!({ "contents": contents }))
            .send()
            .await?;

        let status = resp.status();
        let data: serde_json::Value = resp.json().await?;

        if !status.is_success() {
            let err_msg = data["error"]["message"]
                .as_str()
                .unwrap_or("Unknown Google API error");
            return Err(err_msg.to_string().into());
        }

        let content = data["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let tokens_in = data["usageMetadata"]["promptTokenCount"]
            .as_u64()
            .unwrap_or(0) as u32;
        let tokens_out = data["usageMetadata"]["candidatesTokenCount"]
            .as_u64()
            .unwrap_or(0) as u32;

        Ok((content, tokens_in, tokens_out))
    }

    // ── Tool-use variants (for agentic loop) ──────────────────────

    pub async fn call_anthropic_with_tools(
        api_key: &str,
        model: &str,
        messages: &[serde_json::Value],
        tools: &[serde_json::Value],
        system_prompt: Option<&str>,
        max_tokens: u32,
    ) -> Result<serde_json::Value, String> {
        let client = reqwest::Client::new();

        let mut body = json!({
            "model": model,
            "max_tokens": max_tokens,
            "messages": messages,
        });

        if !tools.is_empty() {
            body["tools"] = serde_json::Value::Array(tools.to_vec());
        }

        // Use structured system prompt with cache_control for prompt caching
        if let Some(sys) = system_prompt {
            body["system"] = json!([{
                "type": "text",
                "text": sys,
                "cache_control": {"type": "ephemeral"}
            }]);
        }

        let max_retries = 3u32;
        let mut last_error = String::new();

        for attempt in 0..=max_retries {
            let response = client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01")
                .header("anthropic-beta", "prompt-caching-2024-07-31")
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    let response_json: serde_json::Value = resp
                        .json()
                        .await
                        .map_err(|e| format!("Failed to parse response: {}", e))?;

                    // Log prompt cache performance
                    if let Some(usage) = response_json.get("usage") {
                        let cache_creation = usage.get("cache_creation_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                        let cache_read = usage.get("cache_read_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                        if cache_read > 0 {
                            info!("Prompt cache hit: {} tokens read from cache", cache_read);
                        } else if cache_creation > 0 {
                            info!("Prompt cache miss: {} tokens written to cache", cache_creation);
                        }
                    }

                    return Ok(response_json);
                }
                Ok(resp) if is_retryable_status(resp.status()) && attempt < max_retries => {
                    last_error = format!("HTTP {}", resp.status());
                    let backoff = std::time::Duration::from_millis(200 * 2u64.pow(attempt));
                    tokio::time::sleep(backoff.min(std::time::Duration::from_secs(2))).await;
                    continue;
                }
                Ok(resp) => {
                    let status = resp.status();
                    let response_json: serde_json::Value = resp
                        .json()
                        .await
                        .map_err(|e| format!("Failed to parse response: {}", e))?;
                    return Err(format!(
                        "Anthropic API error {}: {}",
                        status, response_json
                    ));
                }
                Err(e) if attempt < max_retries && (e.is_connect() || e.is_timeout()) => {
                    last_error = e.to_string();
                    let backoff = std::time::Duration::from_millis(200 * 2u64.pow(attempt));
                    tokio::time::sleep(backoff.min(std::time::Duration::from_secs(2))).await;
                    continue;
                }
                Err(e) => return Err(format!("Anthropic API error: {}", e)),
            }
        }

        Err(format!("Anthropic API: max retries exhausted: {}", last_error))
    }

    pub async fn call_openai_with_tools(
        api_key: &str,
        model: &str,
        messages: &[serde_json::Value],
        tools: &[serde_json::Value],
        max_tokens: u32,
    ) -> Result<serde_json::Value, String> {
        let client = reqwest::Client::new();

        // Convert Anthropic-style tool defs to OpenAI function-calling format
        let openai_tools: Vec<serde_json::Value> = tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                        "description": t.get("description").and_then(|v| v.as_str()).unwrap_or(""),
                        "parameters": t.get("input_schema").cloned().unwrap_or(json!({})),
                    }
                })
            })
            .collect();

        let mut body = json!({
            "model": model,
            "max_tokens": max_tokens,
            "messages": messages,
        });

        if !openai_tools.is_empty() {
            body["tools"] = serde_json::Value::Array(openai_tools);
        }

        let max_retries = 3u32;
        let mut last_error = String::new();

        let data: serde_json::Value = 'retry: {
            for attempt in 0..=max_retries {
                let response = client
                    .post("https://api.openai.com/v1/chat/completions")
                    .header("Authorization", format!("Bearer {}", api_key))
                    .header("Content-Type", "application/json")
                    .json(&body)
                    .send()
                    .await;

                match response {
                    Ok(resp) if resp.status().is_success() => {
                        let parsed: serde_json::Value = resp
                            .json()
                            .await
                            .map_err(|e| format!("Failed to parse OpenAI response: {}", e))?;
                        break 'retry parsed;
                    }
                    Ok(resp) if is_retryable_status(resp.status()) && attempt < max_retries => {
                        last_error = format!("HTTP {}", resp.status());
                        let backoff = std::time::Duration::from_millis(200 * 2u64.pow(attempt));
                        tokio::time::sleep(backoff.min(std::time::Duration::from_secs(2))).await;
                        continue;
                    }
                    Ok(resp) => {
                        let status = resp.status();
                        let err_data: serde_json::Value = resp
                            .json()
                            .await
                            .map_err(|e| format!("Failed to parse OpenAI response: {}", e))?;
                        return Err(format!("OpenAI API error {}: {}", status, err_data));
                    }
                    Err(e) if attempt < max_retries && (e.is_connect() || e.is_timeout()) => {
                        last_error = e.to_string();
                        let backoff = std::time::Duration::from_millis(200 * 2u64.pow(attempt));
                        tokio::time::sleep(backoff.min(std::time::Duration::from_secs(2))).await;
                        continue;
                    }
                    Err(e) => return Err(format!("OpenAI API error: {}", e)),
                }
            }
            return Err(format!("OpenAI API: max retries exhausted: {}", last_error));
        };

        // Normalize OpenAI response to Anthropic-like format for the agent loop
        let choice = &data["choices"][0];
        let message = &choice["message"];
        let finish_reason = choice["finish_reason"].as_str().unwrap_or("stop");

        let mut content_blocks: Vec<serde_json::Value> = vec![];

        // Add text content if present
        if let Some(text) = message["content"].as_str() {
            if !text.is_empty() {
                content_blocks.push(json!({
                    "type": "text",
                    "text": text,
                }));
            }
        }

        // Add tool calls if present
        if let Some(tool_calls) = message["tool_calls"].as_array() {
            for tc in tool_calls {
                let func = &tc["function"];
                let input: serde_json::Value = func["arguments"]
                    .as_str()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or(json!({}));
                content_blocks.push(json!({
                    "type": "tool_use",
                    "id": tc["id"].as_str().unwrap_or(""),
                    "name": func["name"].as_str().unwrap_or(""),
                    "input": input,
                }));
            }
        }

        let stop_reason = match finish_reason {
            "tool_calls" => "tool_use",
            "stop" => "end_turn",
            other => other,
        };

        Ok(json!({
            "content": content_blocks,
            "stop_reason": stop_reason,
            "usage": {
                "input_tokens": data["usage"]["prompt_tokens"].as_u64().unwrap_or(0),
                "output_tokens": data["usage"]["completion_tokens"].as_u64().unwrap_or(0),
            }
        }))
    }

    // ── Vision (multimodal) variants ──────────────────────────────

    pub async fn call_anthropic_vision(
        &self,
        model: &str,
        text: &str,
        image_b64: &str,
        max_tokens: u32,
        api_key: &str,
    ) -> Result<(String, u32, u32), Box<dyn std::error::Error + Send + Sync>> {
        let body = json!({
            "model": model,
            "max_tokens": max_tokens,
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": "image/jpeg",
                            "data": image_b64,
                        }
                    },
                    {
                        "type": "text",
                        "text": text,
                    }
                ]
            }],
        });

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("anthropic-beta", "prompt-caching-2024-07-31")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let data: serde_json::Value = resp.json().await?;

        if !status.is_success() {
            let err_msg = data["error"]["message"]
                .as_str()
                .unwrap_or("Anthropic vision API error");
            return Err(err_msg.to_string().into());
        }

        let content = data["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let tokens_in = data["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32;
        let tokens_out = data["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32;

        Ok((content, tokens_in, tokens_out))
    }

    pub async fn call_openai_vision(
        &self,
        model: &str,
        text: &str,
        image_b64: &str,
        max_tokens: u32,
        api_key: &str,
    ) -> Result<(String, u32, u32), Box<dyn std::error::Error + Send + Sync>> {
        let body = json!({
            "model": model,
            "max_tokens": max_tokens,
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:image/jpeg;base64,{}", image_b64),
                        }
                    },
                    {
                        "type": "text",
                        "text": text,
                    }
                ]
            }],
        });

        let resp = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let data: serde_json::Value = resp.json().await?;

        if !status.is_success() {
            let err_msg = data["error"]["message"]
                .as_str()
                .unwrap_or("OpenAI vision API error");
            return Err(err_msg.to_string().into());
        }

        let content = data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let tokens_in = data["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32;
        let tokens_out = data["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32;

        Ok((content, tokens_in, tokens_out))
    }

    pub async fn call_google_vision(
        &self,
        model: &str,
        text: &str,
        image_b64: &str,
        api_key: &str,
    ) -> Result<(String, u32, u32), Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, api_key
        );

        let body = json!({
            "contents": [{
                "role": "user",
                "parts": [
                    {
                        "inline_data": {
                            "mime_type": "image/jpeg",
                            "data": image_b64,
                        }
                    },
                    { "text": text }
                ]
            }]
        });

        let resp = self.client.post(&url).json(&body).send().await?;
        let status = resp.status();
        let data: serde_json::Value = resp.json().await?;

        if !status.is_success() {
            let err_msg = data["error"]["message"]
                .as_str()
                .unwrap_or("Google vision API error");
            return Err(err_msg.to_string().into());
        }

        let content = data["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let tokens_in = data["usageMetadata"]["promptTokenCount"]
            .as_u64()
            .unwrap_or(0) as u32;
        let tokens_out = data["usageMetadata"]["candidatesTokenCount"]
            .as_u64()
            .unwrap_or(0) as u32;

        Ok((content, tokens_in, tokens_out))
    }
}

/// Returns true for HTTP status codes that are safe to retry.
fn is_retryable_status(status: reqwest::StatusCode) -> bool {
    matches!(status.as_u16(), 408 | 429 | 500 | 502 | 503 | 504)
}
