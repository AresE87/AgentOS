use crate::brain::Gateway;
use crate::config::Settings;
use crate::social::manager::SocialManager;
use crate::social::traits::SocialPost;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::calendar::ContentCalendar;

// ── Mention classification ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MentionClassificationType {
    Positive,
    Question,
    Negative,
    FeatureReq,
    Spam,
}

impl MentionClassificationType {
    pub fn as_str(&self) -> &str {
        match self {
            MentionClassificationType::Positive => "positive",
            MentionClassificationType::Question => "question",
            MentionClassificationType::Negative => "negative",
            MentionClassificationType::FeatureReq => "feature_req",
            MentionClassificationType::Spam => "spam",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "positive" | "praise" => MentionClassificationType::Positive,
            "question" => MentionClassificationType::Question,
            "negative" | "complaint" => MentionClassificationType::Negative,
            "feature_req" | "feature_request" | "feedback" => MentionClassificationType::FeatureReq,
            "spam" => MentionClassificationType::Spam,
            _ => MentionClassificationType::Question,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MentionClassification {
    pub mention_id: String,
    pub platform: String,
    pub author: String,
    pub text: String,
    pub classification: MentionClassificationType,
    pub suggested_response: String,
    pub confidence: f64,
    pub status: String, // "pending", "approved", "sent", "rejected"
}

// ── ResponseEngine ──────────────────────────────────────────────────────

pub struct ResponseEngine;

impl ResponseEngine {
    /// Process new mentions from all platforms: fetch, classify via LLM,
    /// generate suggested responses, and store in response_log.
    ///
    /// Uses `db_path` to open fresh connections that do not cross await points.
    pub async fn process_new_mentions(
        db_path: &std::path::Path,
        social_manager: &SocialManager,
        gateway: &Gateway,
        settings: &Settings,
    ) -> Result<Vec<MentionClassification>, String> {
        // Phase 1: ensure tables + fetch known mention IDs (sync)
        let known_ids: std::collections::HashSet<String> = {
            let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
            ContentCalendar::ensure_tables(&conn)?;
            let mut stmt = conn
                .prepare("SELECT mention_id FROM response_log")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?;
            rows.filter_map(|r| r.ok()).collect()
        };

        // Phase 2: fetch mentions from all platforms (async)
        let all_mentions = social_manager.get_all_mentions(24).await;

        // Filter out already-processed
        let new_mentions: Vec<_> = all_mentions
            .into_iter()
            .filter(|m| !known_ids.contains(&m.id))
            .collect();

        if new_mentions.is_empty() {
            return Ok(Vec::new());
        }

        // Phase 3: classify via LLM (async)
        let mentions_text: Vec<String> = new_mentions
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
            "Classify these social media mentions and suggest responses.\n\n\
             Mentions:\n{}\n\n\
             For each mention, respond with a JSON array:\n\
             [{{\"index\": 0, \"classification\": \"positive|question|negative|feature_req|spam\", \
             \"suggested_response\": \"...\", \"confidence\": 0.95}}]\n\n\
             Rules:\n\
             - Be human, warm, not corporate. No excessive emojis.\n\
             - Max 280 chars for Twitter replies.\n\
             - For spam, set confidence high and response to empty string.\n\
             - For negative, be empathetic and offer help.\n\
             - For questions, answer directly and helpfully.\n\
             - For feature requests, acknowledge and thank.\n\
             Respond ONLY with valid JSON array.",
            mentions_text.join("\n")
        );

        let response = gateway
            .complete_with_system(
                &prompt,
                Some("You are a community manager. Always respond with valid JSON arrays only."),
                settings,
            )
            .await?;

        let text = response.content.trim();
        let json_start = text.find('[').unwrap_or(0);
        let json_end = text.rfind(']').map(|i| i + 1).unwrap_or(text.len());
        let json_slice = &text[json_start..json_end];

        let items: Vec<serde_json::Value> = serde_json::from_str(json_slice)
            .map_err(|e| format!("Failed to parse LLM classification response: {}", e))?;

        // Phase 4: build classifications and persist (sync)
        let mut results = Vec::new();
        let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

        for item in &items {
            let index = item
                .get("index")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;

            if let Some(mention) = new_mentions.get(index) {
                let classification = MentionClassificationType::from_str(
                    item.get("classification")
                        .and_then(|v| v.as_str())
                        .unwrap_or("question"),
                );
                let suggested = item
                    .get("suggested_response")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let confidence = item
                    .get("confidence")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.5);

                let mc = MentionClassification {
                    mention_id: mention.id.clone(),
                    platform: mention.platform.clone(),
                    author: mention.author.clone(),
                    text: mention.content.clone(),
                    classification: classification.clone(),
                    suggested_response: suggested.clone(),
                    confidence,
                    status: "pending".to_string(),
                };

                // Persist to response_log
                let id = uuid::Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO response_log \
                     (id, mention_id, platform, author, original_text, classification, \
                      response_text, confidence, status) \
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                    params![
                        id,
                        mc.mention_id,
                        mc.platform,
                        mc.author,
                        mc.text,
                        mc.classification.as_str(),
                        mc.suggested_response,
                        mc.confidence,
                        mc.status,
                    ],
                )
                .map_err(|e| e.to_string())?;

                results.push(mc);
            }
        }

        Ok(results)
    }

    /// Process new mentions and auto-reply to those with confidence > 0.8.
    /// Limits to max 20 auto-replies per invocation (hourly budget).
    pub async fn auto_respond(
        db_path: &std::path::Path,
        social_manager: &SocialManager,
        gateway: &Gateway,
        settings: &Settings,
    ) -> Result<Vec<(String, String)>, String> {
        let classifications =
            Self::process_new_mentions(db_path, social_manager, gateway, settings).await?;

        let mut replied = Vec::new();
        let mut count = 0u32;
        let max_auto = 20u32;

        for mc in &classifications {
            if count >= max_auto {
                break;
            }
            // Only auto-respond if confidence > 0.8, not spam, and has a response
            if mc.confidence > 0.8
                && mc.classification != MentionClassificationType::Spam
                && !mc.suggested_response.is_empty()
            {
                if let Some(platform) = social_manager.get(&mc.platform) {
                    match platform.reply(&mc.mention_id, &mc.suggested_response).await {
                        Ok(_pr) => {
                            // Mark as sent in DB
                            if let Ok(conn) = Connection::open(db_path) {
                                let _ = conn.execute(
                                    "UPDATE response_log SET status = 'sent' WHERE mention_id = ?1",
                                    params![mc.mention_id],
                                );
                            }
                            replied.push((mc.mention_id.clone(), mc.suggested_response.clone()));
                            count += 1;
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Auto-reply failed for mention {} on {}: {}",
                                mc.mention_id,
                                mc.platform,
                                e
                            );
                        }
                    }
                }
            }
        }

        Ok(replied)
    }

    /// Get pending review items (confidence < 0.8 or awaiting manual approval).
    pub fn get_pending_review(conn: &Connection) -> Result<Vec<MentionClassification>, String> {
        ContentCalendar::ensure_tables(conn)?;
        let mut stmt = conn
            .prepare(
                "SELECT mention_id, platform, author, original_text, classification, \
                 response_text, confidence, status \
                 FROM response_log \
                 WHERE status = 'pending' \
                 ORDER BY created_at DESC",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                let classification_str: String = row.get(4)?;
                Ok(MentionClassification {
                    mention_id: row.get(0)?,
                    platform: row.get(1)?,
                    author: row.get(2)?,
                    text: row.get(3)?,
                    classification: MentionClassificationType::from_str(&classification_str),
                    suggested_response: row.get(5)?,
                    confidence: row.get(6)?,
                    status: row.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?;

        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Approve and publish a pending response.
    pub async fn approve_response(
        db_path: &std::path::Path,
        social_manager: &SocialManager,
        mention_id: &str,
    ) -> Result<(String, String), String> {
        // Phase 1: load the pending response (sync)
        let (platform, response_text) = {
            let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
            let mut stmt = conn
                .prepare(
                    "SELECT platform, response_text FROM response_log WHERE mention_id = ?1 AND status = 'pending'",
                )
                .map_err(|e| e.to_string())?;
            let mut rows = stmt
                .query_map(params![mention_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|e| e.to_string())?;
            rows.next()
                .ok_or_else(|| format!("No pending response for mention {}", mention_id))?
                .map_err(|e| e.to_string())?
        };

        // Phase 2: send the reply (async)
        let platform_handle = social_manager
            .get(&platform)
            .ok_or_else(|| format!("Platform {} not connected", platform))?;
        platform_handle
            .reply(mention_id, &response_text)
            .await?;

        // Phase 3: update status (sync)
        {
            let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
            conn.execute(
                "UPDATE response_log SET status = 'sent' WHERE mention_id = ?1",
                params![mention_id],
            )
            .map_err(|e| e.to_string())?;
        }

        Ok((mention_id.to_string(), response_text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_roundtrip() {
        assert_eq!(
            MentionClassificationType::from_str("positive"),
            MentionClassificationType::Positive
        );
        assert_eq!(
            MentionClassificationType::from_str("question"),
            MentionClassificationType::Question
        );
        assert_eq!(
            MentionClassificationType::from_str("negative"),
            MentionClassificationType::Negative
        );
        assert_eq!(
            MentionClassificationType::from_str("feature_req"),
            MentionClassificationType::FeatureReq
        );
        assert_eq!(
            MentionClassificationType::from_str("spam"),
            MentionClassificationType::Spam
        );
    }

    #[test]
    fn mention_classification_serializes() {
        let mc = MentionClassification {
            mention_id: "m1".to_string(),
            platform: "twitter".to_string(),
            author: "user123".to_string(),
            text: "Great tool!".to_string(),
            classification: MentionClassificationType::Positive,
            suggested_response: "Thanks!".to_string(),
            confidence: 0.95,
            status: "pending".to_string(),
        };
        let json = serde_json::to_string(&mc).unwrap();
        let back: MentionClassification = serde_json::from_str(&json).unwrap();
        assert_eq!(back.mention_id, "m1");
        assert_eq!(back.confidence, 0.95);
    }

    #[test]
    fn get_pending_review_empty() {
        let conn = Connection::open_in_memory().unwrap();
        ContentCalendar::ensure_tables(&conn).unwrap();
        let pending = ResponseEngine::get_pending_review(&conn).unwrap();
        assert!(pending.is_empty());
    }

    #[test]
    fn get_pending_review_with_data() {
        let conn = Connection::open_in_memory().unwrap();
        ContentCalendar::ensure_tables(&conn).unwrap();
        conn.execute(
            "INSERT INTO response_log \
             (id, mention_id, platform, author, original_text, classification, response_text, confidence, status) \
             VALUES ('r1','m1','twitter','user1','hello','positive','thanks!',0.9,'pending')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO response_log \
             (id, mention_id, platform, author, original_text, classification, response_text, confidence, status) \
             VALUES ('r2','m2','twitter','user2','question','question','answer',0.6,'pending')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO response_log \
             (id, mention_id, platform, author, original_text, classification, response_text, confidence, status) \
             VALUES ('r3','m3','twitter','user3','sent','positive','ty',0.95,'sent')",
            [],
        )
        .unwrap();

        let pending = ResponseEngine::get_pending_review(&conn).unwrap();
        assert_eq!(pending.len(), 2);
    }
}
