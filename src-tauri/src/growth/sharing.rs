use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareableContent {
    pub content_type: String, // "playbook", "result", "persona"
    pub title: String,
    pub description: String,
    pub share_url: String,
    pub created_at: String,
}

pub struct ShareManager;

impl ShareManager {
    pub fn create_share_link(content_type: &str, id: &str, title: &str) -> ShareableContent {
        ShareableContent {
            content_type: content_type.to_string(),
            title: title.to_string(),
            description: format!("Shared {} from AgentOS", content_type),
            share_url: format!("https://agentos.app/share/{}/{}", content_type, id),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn create_referral_link(user_id: &str) -> String {
        format!("https://agentos.app/ref/{}", user_id)
    }
}
