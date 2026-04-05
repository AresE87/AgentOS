//! Bloque 4: Telegram notifications for marketing events.
//!
//! Fire-and-forget Telegram messages when marketing events occur:
//! 1. Weekly plan generated
//! 2. Mentions need review
//! 3. Daily summary

use crate::config::Settings;
use tracing::{info, warn};

const TELEGRAM_API: &str = "https://api.telegram.org";

/// Send a Telegram message if bot token and chat_id are configured.
/// Fire-and-forget: errors are logged, never propagated.
pub async fn notify_telegram(settings: &Settings, message: &str) {
    if settings.telegram_bot_token.is_empty() {
        return;
    }
    if settings.telegram_chat_id == 0 {
        // No chat_id stored yet — skip silently.
        // The chat_id gets set when the user first messages the bot.
        return;
    }

    let client = reqwest::Client::new();
    let url = format!(
        "{}/bot{}/sendMessage",
        TELEGRAM_API, settings.telegram_bot_token
    );

    let body = serde_json::json!({
        "chat_id": settings.telegram_chat_id,
        "text": message,
        "parse_mode": "Markdown",
    });

    match client.post(&url).json(&body).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                info!("Telegram marketing notification sent");
            } else {
                // Retry without Markdown if it fails
                let plain_body = serde_json::json!({
                    "chat_id": settings.telegram_chat_id,
                    "text": message,
                });
                let _ = client.post(&url).json(&plain_body).send().await;
                warn!("Telegram notification: Markdown failed, retried as plain text");
            }
        }
        Err(e) => {
            warn!("Telegram notification failed: {}", e);
        }
    }
}

/// Notify that a weekly content plan was generated.
pub async fn notify_plan_generated(settings: &Settings, post_count: usize) {
    let msg = format!(
        "📅 Tu calendario esta listo: {} posts programados. Responde 'ok' para aprobar.",
        post_count
    );
    notify_telegram(settings, &msg).await;
}

/// Notify that mentions need review.
pub async fn notify_mentions_pending(settings: &Settings, mention_count: usize) {
    if mention_count == 0 {
        return;
    }
    let msg = format!(
        "📬 {} menciones nuevas esperando review.",
        mention_count
    );
    notify_telegram(settings, &msg).await;
}

/// Send a daily summary of marketing activity.
pub async fn notify_daily_summary(
    settings: &Settings,
    published: usize,
    impressions: u64,
    replies: usize,
) {
    let msg = format!(
        "📊 Resumen: {} publicados, {} impresiones, {} respuestas.",
        published, impressions, replies
    );
    notify_telegram(settings, &msg).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notify_skips_when_no_token() {
        let settings = Settings::default();
        // This should not panic or attempt any network call
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            notify_telegram(&settings, "test").await;
        });
    }
}
