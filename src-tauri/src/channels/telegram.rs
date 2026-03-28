use reqwest::Client;
use serde_json::json;
use tracing::{info, warn};

const TELEGRAM_API: &str = "https://api.telegram.org";

pub fn is_configured() -> bool {
    std::env::var("TELEGRAM_BOT_TOKEN")
        .map(|t| !t.is_empty())
        .unwrap_or(false)
}

pub struct TelegramBot {
    client: Client,
    token: String,
    offset: i64,
}

impl TelegramBot {
    pub fn new(token: &str) -> Self {
        Self {
            client: Client::new(),
            token: token.to_string(),
            offset: 0,
        }
    }

    /// Send a message to a chat
    pub async fn send_message(
        &self,
        chat_id: i64,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Split long messages (Telegram limit: 4096 chars)
        for chunk in split_message(text, 4000) {
            let url = format!("{}/bot{}/sendMessage", TELEGRAM_API, self.token);
            self.client
                .post(&url)
                .json(&json!({
                    "chat_id": chat_id,
                    "text": chunk,
                    "parse_mode": "Markdown",
                }))
                .send()
                .await?;
        }
        Ok(())
    }

    /// Poll for new messages (long polling)
    pub async fn get_updates(
        &mut self,
    ) -> Result<Vec<TelegramMessage>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/bot{}/getUpdates", TELEGRAM_API, self.token);
        let resp = self
            .client
            .get(&url)
            .query(&[
                ("offset", self.offset.to_string()),
                ("timeout", "30".to_string()),
            ])
            .send()
            .await?;

        let data: serde_json::Value = resp.json().await?;
        let mut messages = Vec::new();

        if let Some(results) = data["result"].as_array() {
            for update in results {
                let update_id = update["update_id"].as_i64().unwrap_or(0);
                if update_id >= self.offset {
                    self.offset = update_id + 1;
                }

                if let Some(msg) = update.get("message") {
                    let chat_id = msg["chat"]["id"].as_i64().unwrap_or(0);
                    let text = msg["text"].as_str().unwrap_or("").to_string();
                    let from = msg["from"]["first_name"]
                        .as_str()
                        .unwrap_or("Unknown")
                        .to_string();

                    if !text.is_empty() {
                        messages.push(TelegramMessage {
                            chat_id,
                            text,
                            from,
                        });
                    }
                }
            }
        }

        Ok(messages)
    }
}

pub struct TelegramMessage {
    pub chat_id: i64,
    pub text: String,
    pub from: String,
}

fn split_message(text: &str, max_len: usize) -> Vec<&str> {
    if text.len() <= max_len {
        return vec![text];
    }
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let end = (start + max_len).min(text.len());
        chunks.push(&text[start..end]);
        start = end;
    }
    chunks
}
