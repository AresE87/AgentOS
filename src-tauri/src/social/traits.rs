use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostResult {
    pub id: String,
    pub url: String,
    pub platform: String,
    pub posted_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mention {
    pub id: String,
    pub platform: String,
    pub author: String,
    pub content: String,
    pub url: String,
    pub mention_type: String, // "reply", "mention", "quote", "comment"
    pub sentiment: Option<String>, // "positive", "negative", "neutral"
    pub created_at: String,
    pub replied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngagementMetrics {
    pub platform: String,
    pub followers: u64,
    pub posts_count: u64,
    pub likes_total: u64,
    pub replies_total: u64,
    pub impressions_total: u64,
    pub engagement_rate: f64,
    pub period: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub platform: String,
    pub title: String,
    pub url: String,
    pub author: String,
    pub score: Option<i64>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialPost {
    pub content: String,
    pub media_url: Option<String>,
    pub reply_to: Option<String>,
    pub tags: Vec<String>,
}

#[async_trait]
pub trait SocialPlatform: Send + Sync {
    fn name(&self) -> &str;
    fn is_connected(&self) -> bool;
    async fn post(&self, post: &SocialPost) -> Result<PostResult, String>;
    async fn reply(&self, post_id: &str, content: &str) -> Result<PostResult, String>;
    async fn get_mentions(&self, since_hours: u32) -> Result<Vec<Mention>, String>;
    async fn get_engagement(&self, period_days: u32) -> Result<EngagementMetrics, String>;
    async fn search(&self, query: &str, limit: u32) -> Result<Vec<SearchResult>, String>;
    async fn delete_post(&self, post_id: &str) -> Result<(), String>;
}
