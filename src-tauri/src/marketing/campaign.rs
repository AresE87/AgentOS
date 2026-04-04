use super::content::ScheduledPost;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Campaign {
    pub id: String,
    pub name: String,
    pub description: String,
    pub platforms: Vec<String>,
    pub posts: Vec<ScheduledPost>,
    pub status: String,
    pub created_at: String,
    pub metrics: Option<CampaignMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignMetrics {
    pub total_posts: u32,
    pub published: u32,
    pub total_impressions: u64,
    pub total_engagements: u64,
    pub best_performing: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignManager {
    campaigns: Vec<Campaign>,
}

impl CampaignManager {
    pub fn new() -> Self {
        Self {
            campaigns: Vec::new(),
        }
    }

    pub fn create(
        &mut self,
        name: &str,
        description: &str,
        platforms: Vec<String>,
    ) -> Campaign {
        let campaign = Campaign {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: description.to_string(),
            platforms,
            posts: Vec::new(),
            status: "draft".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            metrics: None,
        };
        self.campaigns.push(campaign.clone());
        campaign
    }

    pub fn add_posts(&mut self, campaign_id: &str, posts: Vec<ScheduledPost>) {
        if let Some(campaign) = self.campaigns.iter_mut().find(|c| c.id == campaign_id) {
            campaign.posts.extend(posts);
            // Update metrics
            let total = campaign.posts.len() as u32;
            let published = campaign
                .posts
                .iter()
                .filter(|p| p.status == "published")
                .count() as u32;
            campaign.metrics = Some(CampaignMetrics {
                total_posts: total,
                published,
                total_impressions: 0,
                total_engagements: 0,
                best_performing: None,
            });
        }
    }

    pub fn start(&mut self, campaign_id: &str) {
        if let Some(campaign) = self.campaigns.iter_mut().find(|c| c.id == campaign_id) {
            campaign.status = "active".to_string();
            // Mark all draft posts as scheduled
            for post in &mut campaign.posts {
                if post.status == "draft" {
                    post.status = "scheduled".to_string();
                }
            }
        }
    }

    pub fn pause(&mut self, campaign_id: &str) {
        if let Some(campaign) = self.campaigns.iter_mut().find(|c| c.id == campaign_id) {
            campaign.status = "paused".to_string();
        }
    }

    pub fn complete(&mut self, campaign_id: &str) {
        if let Some(campaign) = self.campaigns.iter_mut().find(|c| c.id == campaign_id) {
            campaign.status = "completed".to_string();
        }
    }

    pub fn get(&self, id: &str) -> Option<&Campaign> {
        self.campaigns.iter().find(|c| c.id == id)
    }

    pub fn list(&self) -> &[Campaign] {
        &self.campaigns
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "total_campaigns": self.campaigns.len(),
            "campaigns": serde_json::to_value(&self.campaigns).unwrap_or(serde_json::Value::Array(vec![])),
        })
    }
}

impl Default for CampaignManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_campaign() {
        let mut mgr = CampaignManager::new();
        let campaign = mgr.create("Launch Q1", "Q1 product launch", vec!["twitter".into(), "linkedin".into()]);
        assert_eq!(campaign.name, "Launch Q1");
        assert_eq!(campaign.status, "draft");
        assert_eq!(campaign.platforms.len(), 2);
        assert_eq!(mgr.list().len(), 1);
    }

    #[test]
    fn start_and_pause_campaign() {
        let mut mgr = CampaignManager::new();
        let campaign = mgr.create("Test", "test campaign", vec!["twitter".into()]);
        let id = campaign.id.clone();

        mgr.add_posts(
            &id,
            vec![ScheduledPost {
                id: "p1".to_string(),
                platform: "twitter".to_string(),
                content: "Hello".to_string(),
                scheduled_for: "2025-01-06T09:00".to_string(),
                status: "draft".to_string(),
                tags: vec![],
            }],
        );

        mgr.start(&id);
        let c = mgr.get(&id).unwrap();
        assert_eq!(c.status, "active");
        assert_eq!(c.posts[0].status, "scheduled");

        mgr.pause(&id);
        let c = mgr.get(&id).unwrap();
        assert_eq!(c.status, "paused");
    }

    #[test]
    fn campaign_metrics_update_on_add_posts() {
        let mut mgr = CampaignManager::new();
        let campaign = mgr.create("Metrics Test", "test", vec!["twitter".into()]);
        let id = campaign.id.clone();

        mgr.add_posts(
            &id,
            vec![
                ScheduledPost {
                    id: "p1".to_string(),
                    platform: "twitter".to_string(),
                    content: "Post 1".to_string(),
                    scheduled_for: "2025-01-06T09:00".to_string(),
                    status: "published".to_string(),
                    tags: vec![],
                },
                ScheduledPost {
                    id: "p2".to_string(),
                    platform: "twitter".to_string(),
                    content: "Post 2".to_string(),
                    scheduled_for: "2025-01-07T09:00".to_string(),
                    status: "draft".to_string(),
                    tags: vec![],
                },
            ],
        );

        let c = mgr.get(&id).unwrap();
        let metrics = c.metrics.as_ref().unwrap();
        assert_eq!(metrics.total_posts, 2);
        assert_eq!(metrics.published, 1);
    }

    #[test]
    fn get_nonexistent_campaign_returns_none() {
        let mgr = CampaignManager::new();
        assert!(mgr.get("nonexistent").is_none());
    }
}
