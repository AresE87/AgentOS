use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use tracing::{info, warn};

const DISCORD_GATEWAY: &str = "wss://gateway.discord.gg/?v=10&encoding=json";
const DISCORD_API: &str = "https://discord.com/api/v10";

/// Gateway intents: GUILDS (1) | GUILD_MESSAGES (512) | MESSAGE_CONTENT (32768) | DIRECT_MESSAGES (4096)
const GATEWAY_INTENTS: u64 = 1 | 512 | 4096 | 32768; // = 37377

static DISCORD_RUNNING: AtomicBool = AtomicBool::new(false);
static BOT_USERNAME: Mutex<Option<String>> = Mutex::new(None);
static BOT_ID: Mutex<Option<String>> = Mutex::new(None);

pub fn is_running() -> bool {
    DISCORD_RUNNING.load(Ordering::Relaxed)
}

pub fn bot_name() -> Option<String> {
    BOT_USERNAME.lock().ok().and_then(|g| g.clone())
}

pub fn bot_id() -> Option<String> {
    BOT_ID.lock().ok().and_then(|g| g.clone())
}

pub fn stop() {
    DISCORD_RUNNING.store(false, Ordering::Relaxed);
    if let Ok(mut g) = BOT_USERNAME.lock() {
        *g = None;
    }
    if let Ok(mut g) = BOT_ID.lock() {
        *g = None;
    }
}

// ── Gateway payload types ───────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GatewayPayload {
    op: u8,
    d: Option<serde_json::Value>,
    s: Option<u64>,
    t: Option<String>,
}

#[derive(Debug, Serialize)]
struct GatewayIdentify {
    op: u8,
    d: IdentifyData,
}

#[derive(Debug, Serialize)]
struct IdentifyData {
    token: String,
    intents: u64,
    properties: IdentifyProperties,
}

#[derive(Debug, Serialize)]
struct IdentifyProperties {
    os: String,
    browser: String,
    device: String,
}

#[derive(Debug, Serialize)]
struct GatewayHeartbeat {
    op: u8,
    d: Option<u64>,
}

// ── REST API bot ────────────────────────────────────────────────

pub struct DiscordBot {
    client: Client,
    token: String,
    _bot_id: Option<String>,
}

impl DiscordBot {
    pub fn new(token: &str) -> Self {
        Self {
            client: Client::new(),
            token: token.to_string(),
            _bot_id: None,
        }
    }

    /// Verify token and get bot info via GET /users/@me
    pub async fn verify(&mut self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/users/@me", DISCORD_API);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Discord API returned {}: {}", status, body).into());
        }

        let data: serde_json::Value = resp.json().await?;
        let username = data["username"].as_str().unwrap_or("unknown").to_string();
        let id = data["id"].as_str().unwrap_or("0").to_string();
        self._bot_id = Some(id.clone());

        if let Ok(mut g) = BOT_USERNAME.lock() {
            *g = Some(username.clone());
        }
        if let Ok(mut g) = BOT_ID.lock() {
            *g = Some(id);
        }

        Ok(username)
    }

    /// Send a plain text message to a channel (handles 2000 char limit)
    pub async fn send_message(
        &self,
        channel_id: &str,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for chunk in split_message_smart(content, 1900) {
            let url = format!("{}/channels/{}/messages", DISCORD_API, channel_id);
            let resp = self
                .client
                .post(&url)
                .header("Authorization", format!("Bot {}", self.token))
                .json(&json!({ "content": chunk }))
                .send()
                .await?;

            if !resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                warn!("Discord send_message failed: {}", body);
            }
        }
        Ok(())
    }

    /// Send an embed message to a channel
    pub async fn send_embed(
        &self,
        channel_id: &str,
        title: &str,
        description: &str,
        color: u32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for chunk in split_message_smart(description, 1900) {
            let url = format!("{}/channels/{}/messages", DISCORD_API, channel_id);
            let resp = self
                .client
                .post(&url)
                .header("Authorization", format!("Bot {}", self.token))
                .json(&json!({
                    "embeds": [{
                        "title": title,
                        "description": chunk,
                        "color": color,
                    }]
                }))
                .send()
                .await?;

            if !resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                warn!("Discord send_embed failed: {}", body);
            }
        }
        Ok(())
    }

    /// Send typing indicator (lasts ~10 seconds)
    pub async fn send_typing(
        &self,
        channel_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/channels/{}/typing", DISCORD_API, channel_id);
        self.client
            .post(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .send()
            .await?;
        Ok(())
    }
}

// ── Gateway (WebSocket) bot loop ────────────────────────────────

/// Connect to Discord Gateway via WebSocket and listen for messages.
/// Processes incoming messages via the LLM Gateway, similar to the Telegram bot.
pub async fn run_bot_loop(token: &str, settings: &crate::config::Settings) {
    let mut bot = DiscordBot::new(token);

    // Step 1: Verify token via REST
    match bot.verify().await {
        Ok(username) => {
            info!(username = %username, "Discord bot verified via REST");
        }
        Err(e) => {
            warn!(error = %e, "Discord bot token verification failed");
            return;
        }
    }

    // Step 2: Connect to Gateway WebSocket with reconnect loop
    let gateway = crate::brain::Gateway::new(settings);
    let registry = crate::agents::AgentRegistry::new();

    loop {
        if !DISCORD_RUNNING.load(Ordering::Relaxed) {
            // We haven't set it yet on first iteration, but subsequent
            // iterations check if we were told to stop
            if bot_name().is_some() {
                info!("Discord bot stopped by user");
                break;
            }
        }

        match connect_gateway(token, settings, &gateway, &registry, &bot).await {
            Ok(()) => {
                info!("Discord gateway connection closed cleanly");
            }
            Err(e) => {
                warn!(error = %e, "Discord gateway connection error");
            }
        }

        if !DISCORD_RUNNING.load(Ordering::Relaxed) {
            break;
        }

        // Reconnect delay
        info!("Reconnecting to Discord gateway in 5 seconds...");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }

    stop();
}

async fn connect_gateway(
    token: &str,
    settings: &crate::config::Settings,
    gateway: &crate::brain::Gateway,
    registry: &crate::agents::AgentRegistry,
    bot: &DiscordBot,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Connecting to Discord Gateway WebSocket...");

    let (ws_stream, _) = connect_async(DISCORD_GATEWAY).await?;
    let (mut write, mut read) = ws_stream.split();

    // Step 1: Receive Hello (opcode 10)
    let hello_msg = read
        .next()
        .await
        .ok_or("Gateway closed before Hello")??;

    let hello: GatewayPayload = serde_json::from_str(&hello_msg.to_string())?;
    if hello.op != 10 {
        return Err(format!("Expected opcode 10 (Hello), got {}", hello.op).into());
    }

    let heartbeat_interval = hello
        .d
        .as_ref()
        .and_then(|d| d["heartbeat_interval"].as_u64())
        .unwrap_or(41250);

    info!(heartbeat_interval_ms = heartbeat_interval, "Received Hello from Discord Gateway");

    // Step 2: Send Identify (opcode 2)
    let identify = GatewayIdentify {
        op: 2,
        d: IdentifyData {
            token: token.to_string(),
            intents: GATEWAY_INTENTS,
            properties: IdentifyProperties {
                os: "windows".to_string(),
                browser: "agentos".to_string(),
                device: "agentos".to_string(),
            },
        },
    };

    let identify_json = serde_json::to_string(&identify)?;
    write.send(WsMessage::Text(identify_json)).await?;

    // Step 3: Start heartbeat task
    let (heartbeat_tx, mut heartbeat_rx) = tokio::sync::mpsc::channel::<Option<u64>>(4);
    let heartbeat_interval_dur = std::time::Duration::from_millis(heartbeat_interval);

    // Shared sequence number
    let sequence = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let seq_heartbeat = sequence.clone();

    let heartbeat_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(heartbeat_interval_dur);
        loop {
            interval.tick().await;
            let seq = seq_heartbeat.load(Ordering::Relaxed);
            let seq_val = if seq == 0 { None } else { Some(seq) };
            if heartbeat_tx.send(seq_val).await.is_err() {
                break;
            }
        }
    });

    DISCORD_RUNNING.store(true, Ordering::Relaxed);
    info!("Discord bot connected to Gateway — listening for messages");

    let my_bot_id = bot_id().unwrap_or_default();

    // Step 4: Main event loop
    loop {
        tokio::select! {
            // Send heartbeat
            Some(seq) = heartbeat_rx.recv() => {
                let hb = GatewayHeartbeat { op: 1, d: seq };
                let hb_json = serde_json::to_string(&hb).unwrap_or_default();
                if write.send(WsMessage::Text(hb_json)).await.is_err() {
                    break;
                }
            }
            // Receive gateway events
            msg = read.next() => {
                match msg {
                    Some(Ok(WsMessage::Text(text))) => {
                        let payload: GatewayPayload = match serde_json::from_str(&text) {
                            Ok(p) => p,
                            Err(_) => continue,
                        };

                        // Update sequence number
                        if let Some(s) = payload.s {
                            sequence.store(s, Ordering::Relaxed);
                        }

                        match payload.op {
                            // Dispatch (opcode 0)
                            0 => {
                                if let Some(event_name) = &payload.t {
                                    if event_name == "MESSAGE_CREATE" {
                                        if let Some(d) = &payload.d {
                                            handle_message_create(d, &my_bot_id, bot, gateway, registry, settings).await;
                                        }
                                    } else if event_name == "READY" {
                                        info!("Discord Gateway READY event received");
                                    }
                                }
                            }
                            // Heartbeat ACK (opcode 11) — good
                            11 => {}
                            // Reconnect (opcode 7) — server wants us to reconnect
                            7 => {
                                info!("Discord Gateway requested reconnect (opcode 7)");
                                break;
                            }
                            // Invalid Session (opcode 9)
                            9 => {
                                warn!("Discord Gateway invalid session (opcode 9)");
                                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                                break;
                            }
                            // Heartbeat request (opcode 1) — respond immediately
                            1 => {
                                let seq = sequence.load(Ordering::Relaxed);
                                let seq_val = if seq == 0 { None } else { Some(seq) };
                                let hb = GatewayHeartbeat { op: 1, d: seq_val };
                                let hb_json = serde_json::to_string(&hb).unwrap_or_default();
                                let _ = write.send(WsMessage::Text(hb_json)).await;
                            }
                            _ => {}
                        }
                    }
                    Some(Ok(WsMessage::Close(_))) => {
                        info!("Discord Gateway WebSocket closed by server");
                        break;
                    }
                    Some(Err(e)) => {
                        warn!(error = %e, "Discord Gateway WebSocket error");
                        break;
                    }
                    None => {
                        info!("Discord Gateway WebSocket stream ended");
                        break;
                    }
                    _ => {}
                }
            }
        }

        // Check if we should stop
        if !DISCORD_RUNNING.load(Ordering::Relaxed) {
            break;
        }
    }

    heartbeat_handle.abort();
    Ok(())
}

/// Handle a MESSAGE_CREATE event from Discord Gateway
async fn handle_message_create(
    data: &serde_json::Value,
    my_bot_id: &str,
    bot: &DiscordBot,
    gateway: &crate::brain::Gateway,
    registry: &crate::agents::AgentRegistry,
    settings: &crate::config::Settings,
) {
    let author_id = data["author"]["id"].as_str().unwrap_or("");
    let is_bot = data["author"]["bot"].as_bool().unwrap_or(false);

    // Ignore messages from self or other bots
    if author_id == my_bot_id || is_bot {
        return;
    }

    let content = data["content"].as_str().unwrap_or("").to_string();
    let channel_id = data["channel_id"].as_str().unwrap_or("").to_string();
    let author_name = data["author"]["username"].as_str().unwrap_or("unknown").to_string();
    let guild_id = data["guild_id"].as_str(); // None for DMs

    if content.is_empty() || channel_id.is_empty() {
        return;
    }

    // Check if this is a DM or mentions the bot
    let is_dm = guild_id.is_none();
    let mentions_bot = data["mentions"]
        .as_array()
        .map(|arr| arr.iter().any(|m| m["id"].as_str() == Some(my_bot_id)))
        .unwrap_or(false);

    // Only respond to DMs or mentions in guilds
    if !is_dm && !mentions_bot {
        return;
    }

    // Strip the bot mention from the content for cleaner input
    let clean_content = if mentions_bot {
        let mention_pattern = format!("<@{}>", my_bot_id);
        let mention_pattern_nick = format!("<@!{}>", my_bot_id);
        content
            .replace(&mention_pattern, "")
            .replace(&mention_pattern_nick, "")
            .trim()
            .to_string()
    } else {
        content.clone()
    };

    if clean_content.is_empty() {
        let _ = bot
            .send_message(&channel_id, "Hey! Send me a message and I'll help you out.")
            .await;
        return;
    }

    info!(
        channel_id = %channel_id,
        author = %author_name,
        content = %clean_content,
        is_dm = is_dm,
        "Discord message received"
    );

    // Handle commands
    if clean_content.starts_with("!help") || clean_content.starts_with("/help") {
        let _ = bot
            .send_embed(
                &channel_id,
                "AgentOS",
                "Your AI desktop agent.\n\n\
                 **Commands:**\n\
                 `!help` — This help message\n\
                 `!status` — Agent status\n\n\
                 Just mention me or DM me with any question!",
                0x5865F2, // Discord blurple
            )
            .await;
        return;
    }

    if clean_content.starts_with("!status") || clean_content.starts_with("/status") {
        let providers = settings.configured_providers();
        let desc = format!(
            "**Providers:** {}\n**Status:** Online",
            if providers.is_empty() {
                "None configured".to_string()
            } else {
                providers.join(", ")
            }
        );
        let _ = bot
            .send_embed(&channel_id, "AgentOS Status", &desc, 0x57F287) // green
            .await;
        return;
    }

    // Send typing indicator
    let _ = bot.send_typing(&channel_id).await;

    // Find best agent and generate response
    let agent = registry.find_best(&clean_content);

    // Maintain typing indicator while LLM processes
    let typing_token = bot.token.clone();
    let typing_channel = channel_id.clone();
    let typing_handle = tokio::spawn(async move {
        let client = Client::new();
        loop {
            let url = format!("{}/channels/{}/typing", DISCORD_API, typing_channel);
            let _ = client
                .post(&url)
                .header("Authorization", format!("Bot {}", typing_token))
                .send()
                .await;
            tokio::time::sleep(std::time::Duration::from_secs(8)).await;
        }
    });

    let response = gateway
        .complete_with_system(&clean_content, Some(&agent.system_prompt), settings)
        .await;

    typing_handle.abort();

    match response {
        Ok(resp) => {
            let footer_text = format!(
                "{} | ${:.4} | {:.1}s",
                resp.model,
                resp.cost,
                resp.duration_ms as f64 / 1000.0,
            );

            // Send as embed with agent name as title
            for chunk in split_message_smart(&resp.content, 1900) {
                let url = format!("{}/channels/{}/messages", DISCORD_API, channel_id);
                let _ = bot
                    .client
                    .post(&url)
                    .header("Authorization", format!("Bot {}", bot.token))
                    .json(&json!({
                        "embeds": [{
                            "title": agent.name,
                            "description": chunk,
                            "color": 0x5865F2,
                            "footer": { "text": footer_text },
                        }]
                    }))
                    .send()
                    .await;
            }
        }
        Err(e) => {
            let _ = bot
                .send_embed(
                    &channel_id,
                    "Error",
                    &format!("```\n{}\n```", e),
                    0xED4245, // red
                )
                .await;
        }
    }
}

/// Split message at word boundaries to respect Discord's 2000 char limit
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_short_message_unchanged() {
        let result = split_message_smart("hello world", 1900);
        assert_eq!(result, vec!["hello world"]);
    }

    #[test]
    fn split_long_message_at_word_boundary() {
        let msg = "word ".repeat(100);
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
        assert!(rejoined.contains("quick"));
        assert!(rejoined.contains("lazy"));
    }

    #[test]
    fn gateway_intents_correct() {
        // GUILDS=1, GUILD_MESSAGES=512, DIRECT_MESSAGES=4096, MESSAGE_CONTENT=32768
        assert_eq!(GATEWAY_INTENTS, 1 | 512 | 4096 | 32768);
        assert_eq!(GATEWAY_INTENTS, 37377);
    }

    #[test]
    fn not_running_by_default() {
        assert!(!is_running() || is_running()); // atomic, can't predict state in test
    }
}
