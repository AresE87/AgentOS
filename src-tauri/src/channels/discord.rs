use reqwest::Client;
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{info, warn};

const DISCORD_API: &str = "https://discord.com/api/v10";

static DISCORD_RUNNING: AtomicBool = AtomicBool::new(false);

pub fn is_running() -> bool {
    DISCORD_RUNNING.load(Ordering::Relaxed)
}

pub fn is_configured() -> bool {
    DISCORD_RUNNING.load(Ordering::Relaxed)
}

pub struct DiscordBot {
    client: Client,
    token: String,
    bot_id: Option<String>,
}

impl DiscordBot {
    pub fn new(token: &str) -> Self {
        Self {
            client: Client::new(),
            token: token.to_string(),
            bot_id: None,
        }
    }

    /// Verify token and get bot info
    pub async fn verify(&mut self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/users/@me", DISCORD_API);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err("Invalid Discord bot token".into());
        }

        let data: serde_json::Value = resp.json().await?;
        let username = data["username"].as_str().unwrap_or("unknown").to_string();
        let id = data["id"].as_str().unwrap_or("0").to_string();
        self.bot_id = Some(id);
        Ok(username)
    }

    /// Send a message to a Discord channel with embed
    pub async fn send_embed(
        &self,
        channel_id: &str,
        title: &str,
        description: &str,
        footer: &str,
        color: u32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for chunk in split_message_smart(description, 1900) {
            let url = format!("{}/channels/{}/messages", DISCORD_API, channel_id);
            self.client
                .post(&url)
                .header("Authorization", format!("Bot {}", self.token))
                .json(&json!({
                    "embeds": [{
                        "title": title,
                        "description": chunk,
                        "color": color,
                        "footer": { "text": footer },
                    }]
                }))
                .send()
                .await?;
        }
        Ok(())
    }

    /// Send a plain message
    pub async fn send_message(
        &self,
        channel_id: &str,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for chunk in split_message_smart(content, 1900) {
            let url = format!("{}/channels/{}/messages", DISCORD_API, channel_id);
            self.client
                .post(&url)
                .header("Authorization", format!("Bot {}", self.token))
                .json(&json!({ "content": chunk }))
                .send()
                .await?;
        }
        Ok(())
    }

    /// Send typing indicator
    pub async fn send_typing(&self, channel_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/channels/{}/typing", DISCORD_API, channel_id);
        self.client
            .post(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .send()
            .await?;
        Ok(())
    }

    /// Get messages from a channel (for polling — not ideal but works without gateway)
    pub async fn get_recent_messages(
        &self,
        channel_id: &str,
        after: Option<&str>,
    ) -> Result<Vec<DiscordMessage>, Box<dyn std::error::Error + Send + Sync>> {
        let mut url = format!("{}/channels/{}/messages?limit=10", DISCORD_API, channel_id);
        if let Some(after_id) = after {
            url.push_str(&format!("&after={}", after_id));
        }

        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let data: Vec<serde_json::Value> = resp.json().await?;
        let bot_id = self.bot_id.as_deref().unwrap_or("");

        let messages: Vec<DiscordMessage> = data
            .iter()
            .filter(|msg| {
                // Skip messages from the bot itself
                let author_id = msg["author"]["id"].as_str().unwrap_or("");
                let is_bot = msg["author"]["bot"].as_bool().unwrap_or(false);
                author_id != bot_id && !is_bot
            })
            .filter_map(|msg| {
                let content = msg["content"].as_str()?.to_string();
                if content.is_empty() { return None; }
                Some(DiscordMessage {
                    id: msg["id"].as_str().unwrap_or("0").to_string(),
                    channel_id: msg["channel_id"].as_str().unwrap_or("").to_string(),
                    content,
                    author: msg["author"]["username"].as_str().unwrap_or("unknown").to_string(),
                })
            })
            .collect();

        Ok(messages)
    }
}

pub struct DiscordMessage {
    pub id: String,
    pub channel_id: String,
    pub content: String,
    pub author: String,
}

/// Run Discord bot polling loop
/// Note: This uses HTTP polling, not WebSocket gateway. Simpler but less efficient.
/// For production, should use Discord Gateway (WebSocket) instead.
pub async fn run_bot_loop(token: &str, settings: &crate::config::Settings) {
    let mut bot = DiscordBot::new(token);

    match bot.verify().await {
        Ok(username) => {
            info!(username = %username, "Discord bot connected");
            DISCORD_RUNNING.store(true, Ordering::Relaxed);
        }
        Err(e) => {
            warn!(error = %e, "Discord bot failed to connect");
            return;
        }
    }

    // Note: Without WebSocket Gateway, we can't receive messages via polling easily
    // in Discord. The HTTP API doesn't support long-polling like Telegram.
    // For now, the bot can SEND messages but relies on being mentioned or
    // used via slash commands (future R5 enhancement).
    // Mark as running so the UI shows "Connected".
    info!("Discord bot is running (send-only mode — WebSocket gateway not implemented)");

    // Keep the task alive
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}

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
