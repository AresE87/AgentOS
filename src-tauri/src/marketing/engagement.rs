use crate::brain::Gateway;
use crate::config::Settings;
use crate::social::manager::SocialManager;
use serde::{Deserialize, Serialize};

pub struct EngagementManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mention {
    pub id: String,
    pub platform: String,
    pub author: String,
    pub content: String,
    pub timestamp: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MentionResponse {
    pub mention_id: String,
    pub platform: String,
    pub original_content: String,
    pub classification: String,
    pub suggested_reply: String,
    pub confidence: f64,
    pub auto_reply: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngagementMetrics {
    pub platform: String,
    pub impressions: u64,
    pub engagements: u64,
    pub replies: u64,
    pub shares: u64,
    pub likes: u64,
    pub period: String,
}

impl EngagementManager {
    /// Classify mentions and generate responses using the LLM.
    pub async fn process_mentions(
        mentions: &[Mention],
        brand_voice: &str,
        gateway: &Gateway,
        settings: &Settings,
    ) -> Result<Vec<MentionResponse>, String> {
        if mentions.is_empty() {
            return Ok(Vec::new());
        }

        let mentions_text: Vec<String> = mentions
            .iter()
            .enumerate()
            .map(|(i, m)| {
                format!(
                    "{}. [{}] @{}: \"{}\"",
                    i + 1,
                    m.platform,
                    m.author,
                    m.content
                )
            })
            .collect();

        let prompt = format!(
            "Classify these social media mentions and suggest replies.\n\
             Brand voice: {}\n\n\
             Mentions:\n{}\n\n\
             For each mention, respond with JSON array:\n\
             [{{\"index\": 0, \"classification\": \"question|complaint|praise|spam|feedback\", \
             \"suggested_reply\": \"...\", \"confidence\": 0.95, \"auto_reply\": true}}]\n\n\
             Rules:\n\
             - auto_reply=true only if confidence>=0.85 and classification is \"praise\" or simple \"question\"\n\
             - For complaints, always set auto_reply=false\n\
             - Match the brand voice in all replies\n\
             - Keep replies concise (under 280 chars for Twitter)",
            brand_voice,
            mentions_text.join("\n")
        );

        let response = gateway
            .complete_with_system(
                &prompt,
                Some("You are a community management expert. Always respond with valid JSON arrays."),
                settings,
            )
            .await?;

        let text = response.content.trim();
        let json_start = text.find('[').unwrap_or(0);
        let json_end = text.rfind(']').map(|i| i + 1).unwrap_or(text.len());
        let json_slice = &text[json_start..json_end];

        let items: Vec<serde_json::Value> = serde_json::from_str(json_slice)
            .map_err(|e| format!("Failed to parse mention responses: {}", e))?;

        let mut results = Vec::new();
        for item in &items {
            let index = item
                .get("index")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;

            let mention = mentions.get(index).cloned().unwrap_or_else(|| Mention {
                id: String::new(),
                platform: String::new(),
                author: String::new(),
                content: String::new(),
                timestamp: String::new(),
                url: None,
            });

            results.push(MentionResponse {
                mention_id: mention.id.clone(),
                platform: mention.platform.clone(),
                original_content: mention.content.clone(),
                classification: item
                    .get("classification")
                    .and_then(|v| v.as_str())
                    .unwrap_or("feedback")
                    .to_string(),
                suggested_reply: item
                    .get("suggested_reply")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                confidence: item
                    .get("confidence")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.5),
                auto_reply: item
                    .get("auto_reply")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
            });
        }

        Ok(results)
    }

    /// Summarize engagement across platforms into a JSON value.
    pub fn summarize_engagement(metrics: &[EngagementMetrics]) -> serde_json::Value {
        let total_impressions: u64 = metrics.iter().map(|m| m.impressions).sum();
        let total_engagements: u64 = metrics.iter().map(|m| m.engagements).sum();
        let total_replies: u64 = metrics.iter().map(|m| m.replies).sum();
        let total_shares: u64 = metrics.iter().map(|m| m.shares).sum();
        let total_likes: u64 = metrics.iter().map(|m| m.likes).sum();

        let engagement_rate = if total_impressions > 0 {
            (total_engagements as f64 / total_impressions as f64) * 100.0
        } else {
            0.0
        };

        let by_platform: Vec<serde_json::Value> = metrics
            .iter()
            .map(|m| {
                let rate = if m.impressions > 0 {
                    (m.engagements as f64 / m.impressions as f64) * 100.0
                } else {
                    0.0
                };
                serde_json::json!({
                    "platform": m.platform,
                    "impressions": m.impressions,
                    "engagements": m.engagements,
                    "replies": m.replies,
                    "shares": m.shares,
                    "likes": m.likes,
                    "engagement_rate": format!("{:.2}%", rate),
                    "period": m.period,
                })
            })
            .collect();

        serde_json::json!({
            "total_impressions": total_impressions,
            "total_engagements": total_engagements,
            "total_replies": total_replies,
            "total_shares": total_shares,
            "total_likes": total_likes,
            "engagement_rate": format!("{:.2}%", engagement_rate),
            "by_platform": by_platform,
        })
    }

    /// Process mentions AND auto-reply to high-confidence positive ones.
    ///
    /// 1. Fetches mentions from all connected platforms (last 24h).
    /// 2. Classifies each via the LLM.
    /// 3. Auto-replies to those with `auto_reply == true` and `confidence > 0.8`.
    ///
    /// Returns a list of (mention_id, reply_text) pairs that were actually sent.
    pub async fn auto_respond(
        social_manager: &SocialManager,
        gateway: &Gateway,
        settings: &Settings,
        brand_voice: &str,
    ) -> Result<Vec<(String, String)>, String> {
        // 1. Get mentions from all connected platforms
        let social_mentions = social_manager.get_all_mentions(24).await;

        // Convert social::traits::Mention -> marketing::engagement::Mention
        let mentions: Vec<Mention> = social_mentions
            .iter()
            .map(|m| Mention {
                id: m.id.clone(),
                platform: m.platform.clone(),
                author: m.author.clone(),
                content: m.content.clone(),
                timestamp: m.created_at.clone(),
                url: Some(m.url.clone()),
            })
            .collect();

        if mentions.is_empty() {
            return Ok(Vec::new());
        }

        // 2. Process each with LLM
        let responses = Self::process_mentions(&mentions, brand_voice, gateway, settings).await?;

        // 3. Auto-reply to high-confidence positive ones
        let mut replied = Vec::new();
        for resp in &responses {
            if resp.auto_reply && resp.confidence > 0.8 {
                if let Some(platform) = social_manager.get(&resp.platform) {
                    match platform.reply(&resp.mention_id, &resp.suggested_reply).await {
                        Ok(_pr) => {
                            replied.push((
                                resp.mention_id.clone(),
                                resp.suggested_reply.clone(),
                            ));
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Auto-reply failed for mention {} on {}: {}",
                                resp.mention_id,
                                resp.platform,
                                e
                            );
                        }
                    }
                }
            }
        }

        Ok(replied)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarize_empty_metrics() {
        let summary = EngagementManager::summarize_engagement(&[]);
        assert_eq!(summary["total_impressions"], 0);
        assert_eq!(summary["engagement_rate"], "0.00%");
    }

    #[test]
    fn summarize_multiple_platforms() {
        let metrics = vec![
            EngagementMetrics {
                platform: "twitter".to_string(),
                impressions: 1000,
                engagements: 50,
                replies: 10,
                shares: 20,
                likes: 20,
                period: "7d".to_string(),
            },
            EngagementMetrics {
                platform: "linkedin".to_string(),
                impressions: 500,
                engagements: 100,
                replies: 30,
                shares: 40,
                likes: 30,
                period: "7d".to_string(),
            },
        ];
        let summary = EngagementManager::summarize_engagement(&metrics);
        assert_eq!(summary["total_impressions"], 1500);
        assert_eq!(summary["total_engagements"], 150);
        assert_eq!(summary["engagement_rate"], "10.00%");
        assert_eq!(summary["by_platform"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn mention_serialization_roundtrip() {
        let mention = Mention {
            id: "m1".to_string(),
            platform: "twitter".to_string(),
            author: "user123".to_string(),
            content: "Great product!".to_string(),
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            url: Some("https://twitter.com/status/123".to_string()),
        };
        let json = serde_json::to_string(&mention).unwrap();
        let back: Mention = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "m1");
        assert_eq!(back.author, "user123");
    }
}
