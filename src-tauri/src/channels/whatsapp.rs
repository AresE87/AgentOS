use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tracing::{info, warn};

static WHATSAPP_RUNNING: AtomicBool = AtomicBool::new(false);
static WHATSAPP_PHONE_ID: Mutex<Option<String>> = Mutex::new(None);

pub fn is_running() -> bool {
    WHATSAPP_RUNNING.load(Ordering::Relaxed)
}

pub fn phone_number_id() -> Option<String> {
    WHATSAPP_PHONE_ID.lock().ok().and_then(|g| g.clone())
}

pub fn set_running(running: bool) {
    WHATSAPP_RUNNING.store(running, Ordering::Relaxed);
}

pub fn set_phone_id(id: &str) {
    if let Ok(mut guard) = WHATSAPP_PHONE_ID.lock() {
        *guard = Some(id.to_string());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppConfig {
    pub phone_number_id: String,
    pub access_token: String,
    pub verify_token: String,
    pub webhook_port: u16,
}

pub struct WhatsAppChannel {
    config: WhatsAppConfig,
    client: Client,
}

// ── Webhook payload types ────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct WebhookPayload {
    pub object: String,
    pub entry: Vec<WebhookEntry>,
}

#[derive(Debug, Deserialize)]
pub struct WebhookEntry {
    pub id: String,
    pub changes: Vec<WebhookChange>,
}

#[derive(Debug, Deserialize)]
pub struct WebhookChange {
    pub value: WebhookValue,
    pub field: String,
}

#[derive(Debug, Deserialize)]
pub struct WebhookValue {
    pub messaging_product: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub messages: Option<Vec<IncomingMessage>>,
}

#[derive(Debug, Deserialize)]
pub struct IncomingMessage {
    pub from: String,
    pub id: String,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub text: Option<TextBody>,
    pub image: Option<MediaBody>,
    pub timestamp: String,
}

#[derive(Debug, Deserialize)]
pub struct TextBody {
    pub body: String,
}

#[derive(Debug, Deserialize)]
pub struct MediaBody {
    pub id: String,
    pub mime_type: Option<String>,
    pub caption: Option<String>,
}

// ── Webhook verification query (Meta sends hub.* params) ────

#[derive(Debug, Deserialize)]
pub struct VerifyQuery {
    #[serde(rename = "hub.mode")]
    pub mode: Option<String>,
    #[serde(rename = "hub.verify_token")]
    pub verify_token: Option<String>,
    #[serde(rename = "hub.challenge")]
    pub challenge: Option<String>,
}

impl WhatsAppChannel {
    pub fn new(config: WhatsAppConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    pub fn config(&self) -> &WhatsAppConfig {
        &self.config
    }

    /// Send a text message via WhatsApp Business Cloud API
    pub async fn send_message(&self, to: &str, text: &str) -> Result<(), String> {
        let url = format!(
            "https://graph.facebook.com/v19.0/{}/messages",
            self.config.phone_number_id
        );

        // WhatsApp limit: 4096 chars per message
        let chunks = split_message(text, 4096);
        for chunk in chunks {
            let body = serde_json::json!({
                "messaging_product": "whatsapp",
                "to": to,
                "type": "text",
                "text": { "body": chunk }
            });

            let resp = self
                .client
                .post(&url)
                .bearer_auth(&self.config.access_token)
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("WhatsApp send failed: {}", e))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body_text = resp.text().await.unwrap_or_default();
                return Err(format!("WhatsApp API error {}: {}", status, body_text));
            }
        }
        Ok(())
    }

    /// Send a formatted message with AgentOS branding
    pub async fn send_formatted(
        &self,
        to: &str,
        text: &str,
        agent_name: &str,
        model: &str,
        cost: f64,
        duration_secs: f64,
    ) -> Result<(), String> {
        let formatted = format!(
            "*AgentOS* \u{2014} {}\n\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\n{}\n\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\n_{} \u{00B7} ${:.4} \u{00B7} {:.1}s_",
            agent_name, text, model, cost, duration_secs
        );
        self.send_message(to, &formatted).await
    }

    /// Send an image message
    pub async fn send_image(&self, to: &str, image_url: &str, caption: &str) -> Result<(), String> {
        let url = format!(
            "https://graph.facebook.com/v19.0/{}/messages",
            self.config.phone_number_id
        );
        let body = serde_json::json!({
            "messaging_product": "whatsapp",
            "to": to,
            "type": "image",
            "image": { "link": image_url, "caption": caption }
        });
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.config.access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("WhatsApp send_image failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            return Err(format!("WhatsApp API error {}: {}", status, body_text));
        }
        Ok(())
    }

    /// Verify webhook (Meta sends GET with hub.* params)
    pub fn verify_webhook(query: &VerifyQuery, expected_token: &str) -> Result<String, String> {
        let mode = query.mode.as_deref().unwrap_or("");
        let token = query.verify_token.as_deref().unwrap_or("");
        let challenge = query.challenge.as_deref().unwrap_or("");

        if mode == "subscribe" && token == expected_token {
            Ok(challenge.to_string())
        } else {
            Err("Verification failed".to_string())
        }
    }

    /// Extract (from_number, message_text) pairs from a webhook payload
    pub fn extract_messages(payload: &WebhookPayload) -> Vec<(String, String)> {
        let mut messages = vec![];
        for entry in &payload.entry {
            for change in &entry.changes {
                if let Some(ref msgs) = change.value.messages {
                    for msg in msgs {
                        if let Some(ref text) = msg.text {
                            messages.push((msg.from.clone(), text.body.clone()));
                        }
                    }
                }
            }
        }
        messages
    }

    /// Test connection by checking if the API responds for this phone number.
    /// J2: Returns detailed error for invalid credentials instead of just false.
    pub async fn test_connection(&self) -> Result<bool, String> {
        let url = format!(
            "https://graph.facebook.com/v19.0/{}",
            self.config.phone_number_id
        );
        let res = self
            .client
            .get(&url)
            .bearer_auth(&self.config.access_token)
            .send()
            .await
            .map_err(|e| format!("WhatsApp connection test failed: {}", e))?;

        let status = res.status();
        if status.is_success() {
            return Ok(true);
        }

        // J2: Parse error response for clear messaging
        let body = res.text().await.unwrap_or_default();
        if status.as_u16() == 401 || status.as_u16() == 403 {
            return Err(format!(
                "WhatsApp credentials invalid (HTTP {}): access token is expired or incorrect. {}",
                status.as_u16(),
                body
            ));
        }
        if status.as_u16() == 400 {
            return Err(format!(
                "WhatsApp phone_number_id '{}' is invalid or not found: {}",
                self.config.phone_number_id, body
            ));
        }

        Err(format!(
            "WhatsApp API returned HTTP {}: {}",
            status.as_u16(),
            body
        ))
    }
}

/// J2: Smart truncation for messages that exceed the limit.
/// Adds a "..." indicator and preserves word boundaries.
pub fn truncate_message(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        return text.to_string();
    }
    let suffix = "...";
    let target = max_len.saturating_sub(suffix.len());
    if target == 0 {
        return suffix.to_string();
    }
    // Find a word boundary before target
    let truncated = &text[..target];
    let break_at = truncated
        .rfind(|c: char| c.is_whitespace())
        .unwrap_or(target);
    format!("{}{}", &text[..break_at], suffix)
}

/// Split a long message at word boundaries
fn split_message(text: &str, max_len: usize) -> Vec<String> {
    if text.len() <= max_len {
        return vec![text.to_string()];
    }

    let mut chunks = vec![];
    let mut remaining = text;

    while !remaining.is_empty() {
        if remaining.len() <= max_len {
            chunks.push(remaining.to_string());
            break;
        }

        // Find a whitespace boundary before max_len
        let split_at = remaining[..max_len]
            .rfind(|c: char| c.is_whitespace())
            .unwrap_or(max_len);

        chunks.push(remaining[..split_at].to_string());
        remaining = remaining[split_at..].trim_start();
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_short_message() {
        let chunks = split_message("hello world", 4096);
        assert_eq!(chunks, vec!["hello world"]);
    }

    #[test]
    fn split_long_message() {
        let text = "a ".repeat(3000); // 6000 chars
        let chunks = split_message(&text, 4096);
        assert!(chunks.len() >= 2);
        for chunk in &chunks {
            assert!(chunk.len() <= 4096);
        }
    }

    #[test]
    fn verify_webhook_success() {
        let query = VerifyQuery {
            mode: Some("subscribe".to_string()),
            verify_token: Some("my_token".to_string()),
            challenge: Some("challenge_123".to_string()),
        };
        let result = WhatsAppChannel::verify_webhook(&query, "my_token");
        assert_eq!(result.unwrap(), "challenge_123");
    }

    #[test]
    fn verify_webhook_failure() {
        let query = VerifyQuery {
            mode: Some("subscribe".to_string()),
            verify_token: Some("wrong_token".to_string()),
            challenge: Some("challenge_123".to_string()),
        };
        let result = WhatsAppChannel::verify_webhook(&query, "my_token");
        assert!(result.is_err());
    }

    #[test]
    fn truncate_short_message_unchanged() {
        assert_eq!(truncate_message("hello", 100), "hello");
    }

    #[test]
    fn truncate_long_message_with_ellipsis() {
        let result = truncate_message("The quick brown fox jumps over the lazy dog", 20);
        assert!(result.len() <= 20);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn truncate_preserves_word_boundary() {
        let result = truncate_message("hello world goodbye", 15);
        assert!(result.ends_with("..."));
        assert!(!result.contains("goodby")); // Should not cut mid-word
    }

    #[test]
    fn extract_messages_from_payload() {
        let payload: WebhookPayload = serde_json::from_str(
            r#"{
            "object": "whatsapp_business_account",
            "entry": [{
                "id": "123",
                "changes": [{
                    "value": {
                        "messaging_product": "whatsapp",
                        "messages": [{
                            "from": "15551234567",
                            "id": "msg1",
                            "type": "text",
                            "text": { "body": "Hello agent!" },
                            "timestamp": "1234567890"
                        }]
                    },
                    "field": "messages"
                }]
            }]
        }"#,
        )
        .unwrap();

        let msgs = WhatsAppChannel::extract_messages(&payload);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].0, "15551234567");
        assert_eq!(msgs[0].1, "Hello agent!");
    }
}
