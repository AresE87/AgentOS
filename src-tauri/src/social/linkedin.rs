use async_trait::async_trait;
use serde::Deserialize;

use super::traits::{
    EngagementMetrics, Mention, PostResult, SearchResult, SocialPlatform, SocialPost,
};

pub struct LinkedInConnector {
    client: reqwest::Client,
    access_token: String,
    person_urn: String, // "urn:li:person:XXXXX"
    connected: bool,
}

// ── LinkedIn API response types ────────────────────────────────────────────

#[derive(Deserialize)]
struct LinkedInPostResponse {
    id: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct CommentValue {
    #[serde(rename = "com.linkedin.ugc.ShareComment")]
    comment: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct CommentElement {
    #[serde(rename = "$URN")]
    urn: Option<String>,
    actor: Option<String>,
    message: Option<CommentValue>,
    created: Option<CommentCreated>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct CommentCreated {
    time: Option<u64>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct CommentsResponse {
    elements: Option<Vec<CommentElement>>,
}

impl LinkedInConnector {
    pub fn new(access_token: &str, person_urn: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            access_token: access_token.to_string(),
            person_urn: person_urn.to_string(),
            connected: !access_token.is_empty() && !person_urn.is_empty(),
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.access_token)
    }
}

#[async_trait]
impl SocialPlatform for LinkedInConnector {
    fn name(&self) -> &str {
        "linkedin"
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    async fn post(&self, post: &SocialPost) -> Result<PostResult, String> {
        if !self.connected {
            return Err("LinkedIn not connected".into());
        }

        // POST https://api.linkedin.com/v2/ugcPosts
        let body = serde_json::json!({
            "author": self.person_urn,
            "lifecycleState": "PUBLISHED",
            "specificContent": {
                "com.linkedin.ugc.ShareContent": {
                    "shareCommentary": {
                        "text": post.content
                    },
                    "shareMediaCategory": "NONE"
                }
            },
            "visibility": {
                "com.linkedin.ugc.MemberNetworkVisibility": "PUBLIC"
            }
        });

        let resp = self
            .client
            .post("https://api.linkedin.com/v2/ugcPosts")
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .header("X-Restli-Protocol-Version", "2.0.0")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("LinkedIn post failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("LinkedIn API {status}: {text}"));
        }

        let data: LinkedInPostResponse = resp
            .json()
            .await
            .map_err(|e| format!("LinkedIn parse: {e}"))?;

        let post_id = data.id.unwrap_or_else(|| "unknown".into());

        Ok(PostResult {
            id: post_id.clone(),
            url: format!("https://www.linkedin.com/feed/update/{post_id}"),
            platform: "linkedin".into(),
            posted_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    async fn reply(&self, post_id: &str, content: &str) -> Result<PostResult, String> {
        if !self.connected {
            return Err("LinkedIn not connected".into());
        }

        // POST https://api.linkedin.com/v2/socialActions/{urn}/comments
        let encoded_urn = urlencoding::encode(post_id);
        let url = format!(
            "https://api.linkedin.com/v2/socialActions/{encoded_urn}/comments"
        );

        let body = serde_json::json!({
            "actor": self.person_urn,
            "message": {
                "text": content
            }
        });

        let resp = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("LinkedIn reply failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("LinkedIn reply {status}: {text}"));
        }

        Ok(PostResult {
            id: format!("comment-on-{post_id}"),
            url: format!("https://www.linkedin.com/feed/update/{post_id}"),
            platform: "linkedin".into(),
            posted_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    async fn get_mentions(&self, _since_hours: u32) -> Result<Vec<Mention>, String> {
        if !self.connected {
            return Err("LinkedIn not connected".into());
        }

        // LinkedIn API v2 does not provide a straightforward "mentions" endpoint
        // for individual members.  Return empty for now — the real implementation
        // would poll the organization notifications API or use webhooks.
        Ok(vec![])
    }

    async fn get_engagement(&self, period_days: u32) -> Result<EngagementMetrics, String> {
        if !self.connected {
            return Err("LinkedIn not connected".into());
        }

        // GET https://api.linkedin.com/v2/me — basic profile
        let resp = self
            .client
            .get("https://api.linkedin.com/v2/me")
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| format!("LinkedIn profile failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("LinkedIn profile {status}: {text}"));
        }

        // LinkedIn profile does not expose follower count directly via /v2/me.
        // Org pages can use /v2/networkSizes but personal profiles need
        // /v2/connections?q=viewer with a scope.  Return basic metrics for now.
        Ok(EngagementMetrics {
            platform: "linkedin".into(),
            followers: 0,
            posts_count: 0,
            likes_total: 0,
            replies_total: 0,
            impressions_total: 0,
            engagement_rate: 0.0,
            period: format!("{period_days}d"),
        })
    }

    async fn search(&self, _query: &str, _limit: u32) -> Result<Vec<SearchResult>, String> {
        if !self.connected {
            return Err("LinkedIn not connected".into());
        }

        // LinkedIn does not expose a public search API for posts via v2.
        // Company pages can search their own content, but personal profiles cannot.
        Ok(vec![])
    }

    async fn delete_post(&self, post_id: &str) -> Result<(), String> {
        if !self.connected {
            return Err("LinkedIn not connected".into());
        }

        let encoded = urlencoding::encode(post_id);
        let url = format!("https://api.linkedin.com/v2/ugcPosts/{encoded}");

        let resp = self
            .client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| format!("LinkedIn delete failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("LinkedIn delete {status}: {text}"));
        }

        Ok(())
    }
}
