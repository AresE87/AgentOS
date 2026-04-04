use crate::brain::Gateway;
use crate::config::Settings;
use serde::{Deserialize, Serialize};

pub struct ContentGenerator;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedContent {
    pub platform: String,
    pub content: String,
    pub hashtags: Vec<String>,
    pub suggested_media: Option<String>,
    pub tone: String,
    pub estimated_engagement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledPost {
    pub id: String,
    pub platform: String,
    pub content: String,
    pub scheduled_for: String,
    pub status: String,
    pub tags: Vec<String>,
}

impl ContentGenerator {
    /// Generate content for a topic, adapted per platform.
    pub async fn generate(
        topic: &str,
        platforms: &[String],
        tone: &str,
        gateway: &Gateway,
        settings: &Settings,
    ) -> Result<Vec<GeneratedContent>, String> {
        let prompt = format!(
            "Generate social media posts about: {}\nTone: {}\nPlatforms: {}\n\n\
             For each platform, respond with JSON array:\n\
             [{{\"platform\": \"twitter\", \"content\": \"...\", \"hashtags\": [...], \"tone\": \"...\", \"estimated_engagement\": \"medium\"}}]\n\n\
             Rules:\n\
             - Twitter: max 270 chars, 2-3 hashtags, engaging hook\n\
             - LinkedIn: 150-300 words, professional, end with question or CTA\n\
             - Reddit: title (max 100 chars) + body (informative, not promotional)\n\
             - HN: concise technical title only",
            topic,
            tone,
            platforms.join(", ")
        );

        let response = gateway
            .complete_with_system(
                &prompt,
                Some("You are a social media expert. Always respond with valid JSON arrays."),
                settings,
            )
            .await?;

        // Attempt to parse JSON array from response text
        let text = response.content.trim();
        // Find the JSON array in the response
        let json_start = text.find('[').unwrap_or(0);
        let json_end = text.rfind(']').map(|i| i + 1).unwrap_or(text.len());
        let json_slice = &text[json_start..json_end];

        let items: Vec<serde_json::Value> =
            serde_json::from_str(json_slice).map_err(|e| format!("Failed to parse LLM response as JSON: {}", e))?;

        let mut results = Vec::new();
        for item in items {
            results.push(GeneratedContent {
                platform: item
                    .get("platform")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                content: item
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                hashtags: item
                    .get("hashtags")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default(),
                suggested_media: item
                    .get("suggested_media")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                tone: item
                    .get("tone")
                    .and_then(|v| v.as_str())
                    .unwrap_or(tone)
                    .to_string(),
                estimated_engagement: item
                    .get("estimated_engagement")
                    .and_then(|v| v.as_str())
                    .unwrap_or("medium")
                    .to_string(),
            });
        }

        Ok(results)
    }

    /// Generate a week of scheduled content across topics and platforms.
    pub async fn generate_weekly_plan(
        topics: &[String],
        platforms: &[String],
        posts_per_week: u32,
        gateway: &Gateway,
        settings: &Settings,
    ) -> Result<Vec<ScheduledPost>, String> {
        let prompt = format!(
            "Create a weekly content plan with {} posts.\n\
             Topics: {}\n\
             Platforms: {}\n\n\
             For each post, respond with JSON array:\n\
             [{{\"platform\": \"twitter\", \"content\": \"...\", \"scheduled_for\": \"Monday 09:00\", \"tags\": [...]}}]\n\n\
             Distribute posts evenly across the week. Vary topics and platforms.\n\
             Best posting times: Twitter 9-11am, LinkedIn 8-10am, Reddit 6-9am.",
            posts_per_week,
            topics.join(", "),
            platforms.join(", ")
        );

        let response = gateway
            .complete_with_system(
                &prompt,
                Some("You are a social media calendar planner. Always respond with valid JSON arrays."),
                settings,
            )
            .await?;

        let text = response.content.trim();
        let json_start = text.find('[').unwrap_or(0);
        let json_end = text.rfind(']').map(|i| i + 1).unwrap_or(text.len());
        let json_slice = &text[json_start..json_end];

        let items: Vec<serde_json::Value> =
            serde_json::from_str(json_slice).map_err(|e| format!("Failed to parse weekly plan JSON: {}", e))?;

        let mut posts = Vec::new();
        for item in &items {
            posts.push(ScheduledPost {
                id: uuid::Uuid::new_v4().to_string(),
                platform: item
                    .get("platform")
                    .and_then(|v| v.as_str())
                    .unwrap_or("twitter")
                    .to_string(),
                content: item
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                scheduled_for: item
                    .get("scheduled_for")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Monday 09:00")
                    .to_string(),
                status: "draft".to_string(),
                tags: item
                    .get("tags")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default(),
            });
        }

        Ok(posts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduled_post_serialization_roundtrip() {
        let post = ScheduledPost {
            id: "test-123".to_string(),
            platform: "twitter".to_string(),
            content: "Hello world".to_string(),
            scheduled_for: "2025-01-01T09:00:00Z".to_string(),
            status: "draft".to_string(),
            tags: vec!["launch".to_string()],
        };
        let json = serde_json::to_string(&post).unwrap();
        let back: ScheduledPost = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "test-123");
        assert_eq!(back.platform, "twitter");
        assert_eq!(back.status, "draft");
    }

    #[test]
    fn generated_content_serialization_roundtrip() {
        let content = GeneratedContent {
            platform: "linkedin".to_string(),
            content: "Professional post".to_string(),
            hashtags: vec!["#tech".to_string()],
            suggested_media: Some("photo".to_string()),
            tone: "professional".to_string(),
            estimated_engagement: "high".to_string(),
        };
        let json = serde_json::to_string(&content).unwrap();
        let back: GeneratedContent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.platform, "linkedin");
        assert_eq!(back.hashtags.len(), 1);
    }
}
