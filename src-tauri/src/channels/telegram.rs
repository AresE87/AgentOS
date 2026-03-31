use reqwest::Client;
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tracing::{info, warn};

const TELEGRAM_API: &str = "https://api.telegram.org";

/// Track whether the Telegram bot is connected and polling
static TELEGRAM_RUNNING: AtomicBool = AtomicBool::new(false);

/// Store the bot username once verified, so it's accessible from channel status
static BOT_USERNAME: Mutex<Option<String>> = Mutex::new(None);

pub fn is_running() -> bool {
    TELEGRAM_RUNNING.load(Ordering::Relaxed)
}

/// Returns the bot username if connected, or None
pub fn bot_name() -> Option<String> {
    BOT_USERNAME.lock().ok().and_then(|g| g.clone())
}

pub struct TelegramBot {
    client: Client,
    token: String,
    offset: i64,
    bot_username: Option<String>,
}

impl TelegramBot {
    pub fn new(token: &str) -> Self {
        Self {
            client: Client::new(),
            token: token.to_string(),
            offset: 0,
            bot_username: None,
        }
    }

    /// Verify token and get bot info
    pub async fn verify(&mut self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/bot{}/getMe", TELEGRAM_API, self.token);
        let resp = self.client.get(&url).send().await?;
        let data: serde_json::Value = resp.json().await?;
        if data["ok"].as_bool() != Some(true) {
            return Err("Invalid bot token".into());
        }
        let username = data["result"]["username"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        self.bot_username = Some(username.clone());
        // Store globally so channel status can report the bot name
        if let Ok(mut g) = BOT_USERNAME.lock() {
            *g = Some(username.clone());
        }
        Ok(username)
    }

    /// Send typing indicator
    pub async fn send_typing(
        &self,
        chat_id: i64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/bot{}/sendChatAction", TELEGRAM_API, self.token);
        self.client
            .post(&url)
            .json(&json!({ "chat_id": chat_id, "action": "typing" }))
            .send()
            .await?;
        Ok(())
    }

    /// Send a message to a chat with Markdown formatting
    pub async fn send_message(
        &self,
        chat_id: i64,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for chunk in split_message_smart(text, 4000) {
            let url = format!("{}/bot{}/sendMessage", TELEGRAM_API, self.token);
            let resp = self
                .client
                .post(&url)
                .json(&json!({
                    "chat_id": chat_id,
                    "text": chunk,
                    "parse_mode": "Markdown",
                }))
                .send()
                .await?;

            // If Markdown fails, retry without parse_mode
            if !resp.status().is_success() {
                let url = format!("{}/bot{}/sendMessage", TELEGRAM_API, self.token);
                self.client
                    .post(&url)
                    .json(&json!({
                        "chat_id": chat_id,
                        "text": chunk,
                    }))
                    .send()
                    .await?;
            }
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

    // Verify token first
    match bot.verify().await {
        Ok(username) => {
            info!(username = %username, "Telegram bot connected");
            TELEGRAM_RUNNING.store(true, Ordering::Relaxed);
        }
        Err(e) => {
            warn!(error = %e, "Telegram bot failed to connect");
            return;
        }
    }

    let gateway = crate::brain::Gateway::new(settings);
    let registry = crate::agents::AgentRegistry::new();

    loop {
        match bot.get_updates().await {
            Ok(messages) => {
                for msg in messages {
                    info!(
                        chat_id = msg.chat_id,
                        from = %msg.from,
                        text = %msg.text,
                        "Telegram message received"
                    );

                    // Handle commands
                    if msg.text.starts_with("/start") || msg.text.starts_with("/help") {
                        let help = concat!(
                            "*AgentOS* — Your AI desktop agent\n\n",
                            "Send any message and I'll respond with the right specialist.\n\n",
                            "*Commands:*\n",
                            "/status — Agent status\n",
                            "/help — This help message",
                        );
                        let _ = bot.send_message(msg.chat_id, help).await;
                        continue;
                    }

                    if msg.text.starts_with("/status") {
                        let providers = settings.configured_providers();
                        let status = format!(
                            "*AgentOS Status*\n\nProviders: {}\nStatus: Online",
                            if providers.is_empty() {
                                "None configured".to_string()
                            } else {
                                providers.join(", ")
                            }
                        );
                        let _ = bot.send_message(msg.chat_id, &status).await;
                        continue;
                    }

                    // Send typing indicator
                    let _ = bot.send_typing(msg.chat_id).await;

                    // Find best agent and respond
                    let agent = registry.find_best(&msg.text);

                    // Send typing again while waiting for LLM
                    let typing_chat_id = msg.chat_id;
                    let typing_token = token.to_string();
                    let typing_handle = tokio::spawn(async move {
                        let client = Client::new();
                        loop {
                            let url =
                                format!("{}/bot{}/sendChatAction", TELEGRAM_API, typing_token);
                            let _ = client
                                .post(&url)
                                .json(&json!({ "chat_id": typing_chat_id, "action": "typing" }))
                                .send()
                                .await;
                            tokio::time::sleep(std::time::Duration::from_secs(4)).await;
                        }
                    });

                    let response = gateway
                        .complete_with_system(&msg.text, Some(&agent.system_prompt), settings)
                        .await;

                    // Stop typing indicator
                    typing_handle.abort();

                    match response {
                        Ok(resp) => {
                            let reply = format!(
                                "*{}*\n\n{}\n\n_{} · ${:.4} · {:.1}s_",
                                agent.name,
                                resp.content,
                                resp.model,
                                resp.cost,
                                resp.duration_ms as f64 / 1000.0,
                            );
                            let _ = bot.send_message(msg.chat_id, &reply).await;
                        }
                        Err(e) => {
                            let reply = format!("*Error*\n\n{}", e);
                            let _ = bot.send_message(msg.chat_id, &reply).await;
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Telegram polling error: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }
}

/// Split message at word boundaries to respect Telegram's char limit
fn split_message_smart(text: &str, max_len: usize) -> Vec<String> {
    if text.len() <= max_len {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        if remaining.len() <= max_len {
            chunks.push(remaining.to_string());
            break;
        }

        // Find a good break point: newline, then space, then hard cut
        let search_range = &remaining[..max_len];
        let break_at = search_range
            .rfind('\n')
            .or_else(|| search_range.rfind(' '))
            .unwrap_or(max_len);

        let break_at = if break_at == 0 { max_len } else { break_at };

        chunks.push(remaining[..break_at].to_string());
        remaining = remaining[break_at..].trim_start();
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_short_message_unchanged() {
        let result = split_message_smart("hello world", 4000);
        assert_eq!(result, vec!["hello world"]);
    }

    #[test]
    fn split_long_message_at_word_boundary() {
        let msg = "word ".repeat(100); // 500 chars
        let result = split_message_smart(&msg, 50);
        assert!(result.len() > 1);
        for chunk in &result {
            assert!(chunk.len() <= 50);
        }
    }

    #[test]
    fn split_preserves_all_content() {
        let msg = "The quick brown fox jumps over the lazy dog. ".repeat(20);
        let result = split_message_smart(&msg, 100);
        let rejoined: String = result.join(" ");
        // All words should be present
        assert!(rejoined.contains("quick"));
        assert!(rejoined.contains("lazy"));
    }

    #[test]
    fn split_at_newlines_preferred() {
        let msg = "line one\nline two\nline three\nline four\nline five";
        let result = split_message_smart(msg, 20);
        // Should break at newlines, not mid-word
        assert!(result[0].ends_with("one") || result[0].ends_with("two"));
    }

    #[test]
    fn split_empty_message() {
        let result = split_message_smart("", 4000);
        assert_eq!(result, vec![""]);
    }
}
