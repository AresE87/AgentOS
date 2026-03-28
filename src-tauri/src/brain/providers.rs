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
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
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
}
