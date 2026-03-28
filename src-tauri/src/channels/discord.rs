use reqwest::Client;
use serde_json::json;

const DISCORD_API: &str = "https://discord.com/api/v10";

pub fn is_configured() -> bool {
    std::env::var("DISCORD_BOT_TOKEN")
        .map(|t| !t.is_empty())
        .unwrap_or(false)
}

pub struct DiscordBot {
    client: Client,
    token: String,
}

impl DiscordBot {
    pub fn new(token: &str) -> Self {
        Self {
            client: Client::new(),
            token: token.to_string(),
        }
    }

    /// Send a message to a Discord channel
    pub async fn send_message(
        &self,
        channel_id: &str,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Discord limit: 2000 chars
        for chunk in split_message(content, 1900) {
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
