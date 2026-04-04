use std::collections::HashMap;

use super::traits::{EngagementMetrics, Mention, PostResult, SocialPlatform, SocialPost};

pub struct SocialManager {
    platforms: HashMap<String, Box<dyn SocialPlatform>>,
}

impl SocialManager {
    pub fn new() -> Self {
        Self {
            platforms: HashMap::new(),
        }
    }

    pub fn add_platform(&mut self, platform: Box<dyn SocialPlatform>) {
        let name = platform.name().to_string();
        self.platforms.insert(name, platform);
    }

    pub fn remove_platform(&mut self, name: &str) {
        self.platforms.remove(name);
    }

    pub fn get(&self, name: &str) -> Option<&dyn SocialPlatform> {
        self.platforms.get(name).map(|p| p.as_ref())
    }

    pub fn list_connected(&self) -> Vec<String> {
        self.platforms
            .iter()
            .filter(|(_, p)| p.is_connected())
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Post to multiple platforms at once.
    pub async fn post_to_all(
        &self,
        post: &SocialPost,
        platforms: &[String],
    ) -> Vec<(String, Result<PostResult, String>)> {
        let mut results = Vec::new();
        for name in platforms {
            let result = if let Some(platform) = self.platforms.get(name) {
                platform.post(post).await
            } else {
                Err(format!("Platform '{name}' not found"))
            };
            results.push((name.clone(), result));
        }
        results
    }

    /// Get mentions from all connected platforms.
    pub async fn get_all_mentions(&self, since_hours: u32) -> Vec<Mention> {
        let mut all_mentions = Vec::new();
        for (_, platform) in &self.platforms {
            if platform.is_connected() {
                if let Ok(mentions) = platform.get_mentions(since_hours).await {
                    all_mentions.extend(mentions);
                }
            }
        }
        all_mentions
    }

    /// Get aggregated engagement from all connected platforms.
    pub async fn get_total_engagement(&self, period_days: u32) -> Vec<EngagementMetrics> {
        let mut all_metrics = Vec::new();
        for (_, platform) in &self.platforms {
            if platform.is_connected() {
                if let Ok(metrics) = platform.get_engagement(period_days).await {
                    all_metrics.push(metrics);
                }
            }
        }
        all_metrics
    }
}
