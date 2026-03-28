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

/// Run the Telegram bot polling loop — processes messages via LLM
pub async fn run_bot_loop(token: &str, settings: &crate::config::Settings) {
    let mut bot = TelegramBot::new(token);
    let gateway = crate::brain::Gateway::new(settings);
    let registry = crate::agents::AgentRegistry::new();

    loop {
        match bot.get_updates().await {
            Ok(messages) => {
                for msg in messages {
                    tracing::info!(
                        chat_id = msg.chat_id,
                        from = %msg.from,
                        text = %msg.text,
                        "Telegram message received"
                    );

                    // Handle commands
                    if msg.text.starts_with("/start") || msg.text.starts_with("/help") {
                        let help = "AgentOS - Tu equipo de IA\n\nEnvía cualquier mensaje y te respondo con el especialista adecuado.\n\nComandos:\n/status - Estado del agente\n/help - Esta ayuda";
                        let _ = bot.send_message(msg.chat_id, help).await;
                        continue;
                    }

                    if msg.text.starts_with("/status") {
                        let _ = bot.send_message(msg.chat_id, "AgentOS: Online").await;
                        continue;
                    }

                    // Find best agent and respond
                    let agent = registry.find_best(&msg.text);
                    let response = gateway
                        .complete_with_system(&msg.text, Some(&agent.system_prompt), settings)
                        .await;

                    match response {
                        Ok(resp) => {
                            let reply = format!("[{}] {}", agent.name, resp.content);
                            let _ = bot.send_message(msg.chat_id, &reply).await;
                        }
                        Err(e) => {
                            let _ = bot
                                .send_message(msg.chat_id, &format!("Error: {}", e))
                                .await;
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Telegram polling error: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }
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
