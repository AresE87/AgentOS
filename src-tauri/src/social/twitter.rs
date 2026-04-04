use async_trait::async_trait;
use serde::Deserialize;

use super::traits::{
    EngagementMetrics, Mention, PostResult, SearchResult, SocialPlatform, SocialPost,
};

pub struct TwitterConnector {
    client: reqwest::Client,
    bearer_token: String,
    api_key: String,
    api_secret: String,
    access_token: String,
    access_secret: String,
    connected: bool,
}

// ── Twitter API v2 response types ──────────────────────────────────────────

#[derive(Deserialize)]
struct TweetData {
    id: Option<String>,
}

#[derive(Deserialize)]
struct TweetResponse {
    data: Option<TweetData>,
}

#[derive(Deserialize)]
struct MentionItem {
    id: Option<String>,
    text: Option<String>,
    author_id: Option<String>,
    created_at: Option<String>,
}

#[derive(Deserialize)]
struct MentionsResponse {
    data: Option<Vec<MentionItem>>,
}

#[derive(Deserialize)]
struct UserPublicMetrics {
    followers_count: Option<u64>,
    tweet_count: Option<u64>,
}

#[derive(Deserialize)]
struct UserData {
    public_metrics: Option<UserPublicMetrics>,
}

#[derive(Deserialize)]
struct UserResponse {
    data: Option<UserData>,
}

#[derive(Deserialize)]
struct SearchTweetItem {
    id: Option<String>,
    text: Option<String>,
    author_id: Option<String>,
    created_at: Option<String>,
}

#[derive(Deserialize)]
struct SearchTweetsResponse {
    data: Option<Vec<SearchTweetItem>>,
}

impl TwitterConnector {
    pub fn new(
        bearer_token: &str,
        api_key: &str,
        api_secret: &str,
        access_token: &str,
        access_secret: &str,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            bearer_token: bearer_token.to_string(),
            api_key: api_key.to_string(),
            api_secret: api_secret.to_string(),
            access_token: access_token.to_string(),
            access_secret: access_secret.to_string(),
            connected: !bearer_token.is_empty(),
        }
    }

    /// Build the auth header. Twitter v2 supports Bearer token for app-only auth.
    fn auth_header(&self) -> String {
        format!("Bearer {}", self.bearer_token)
    }
}

#[async_trait]
impl SocialPlatform for TwitterConnector {
    fn name(&self) -> &str {
        "twitter"
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    async fn post(&self, post: &SocialPost) -> Result<PostResult, String> {
        if !self.connected {
            return Err("Twitter not connected".into());
        }

        // Twitter API v2 — POST https://api.twitter.com/2/tweets
        let mut body = serde_json::json!({ "text": post.content });
        if let Some(ref reply_to) = post.reply_to {
            body["reply"] = serde_json::json!({ "in_reply_to_tweet_id": reply_to });
        }

        let resp = self
            .client
            .post("https://api.twitter.com/2/tweets")
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Twitter post request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Twitter API {status}: {text}"));
        }

        let tweet: TweetResponse = resp
            .json()
            .await
            .map_err(|e| format!("Twitter response parse error: {e}"))?;

        let tweet_id = tweet
            .data
            .and_then(|d| d.id)
            .unwrap_or_else(|| "unknown".into());

        Ok(PostResult {
            id: tweet_id.clone(),
            url: format!("https://twitter.com/i/web/status/{tweet_id}"),
            platform: "twitter".into(),
            posted_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    async fn reply(&self, post_id: &str, content: &str) -> Result<PostResult, String> {
        let post = SocialPost {
            content: content.to_string(),
            media_url: None,
            reply_to: Some(post_id.to_string()),
            tags: vec![],
        };
        self.post(&post).await
    }

    async fn get_mentions(&self, since_hours: u32) -> Result<Vec<Mention>, String> {
        if !self.connected {
            return Err("Twitter not connected".into());
        }

        // We need the authenticated user's ID first.  For simplicity, use the
        // /2/users/me endpoint.
        let me_resp = self
            .client
            .get("https://api.twitter.com/2/users/me")
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| format!("Twitter users/me failed: {e}"))?;

        #[derive(Deserialize)]
        struct MeData {
            id: Option<String>,
        }
        #[derive(Deserialize)]
        struct MeResp {
            data: Option<MeData>,
        }

        let me: MeResp = me_resp
            .json()
            .await
            .map_err(|e| format!("Twitter users/me parse: {e}"))?;

        let user_id = me
            .data
            .and_then(|d| d.id)
            .ok_or("Could not determine Twitter user id")?;

        let start_time = (chrono::Utc::now()
            - chrono::Duration::hours(i64::from(since_hours)))
        .to_rfc3339();

        let url = format!(
            "https://api.twitter.com/2/users/{user_id}/mentions?start_time={start_time}&tweet.fields=author_id,created_at"
        );

        let resp = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| format!("Twitter mentions failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Twitter mentions {status}: {text}"));
        }

        let data: MentionsResponse = resp
            .json()
            .await
            .map_err(|e| format!("Twitter mentions parse: {e}"))?;

        let mentions = data
            .data
            .unwrap_or_default()
            .into_iter()
            .map(|m| Mention {
                id: m.id.clone().unwrap_or_default(),
                platform: "twitter".into(),
                author: m.author_id.unwrap_or_else(|| "unknown".into()),
                content: m.text.unwrap_or_default(),
                url: format!(
                    "https://twitter.com/i/web/status/{}",
                    m.id.unwrap_or_default()
                ),
                mention_type: "mention".into(),
                sentiment: None,
                created_at: m.created_at.unwrap_or_default(),
                replied: false,
            })
            .collect();

        Ok(mentions)
    }

    async fn get_engagement(&self, period_days: u32) -> Result<EngagementMetrics, String> {
        if !self.connected {
            return Err("Twitter not connected".into());
        }

        let resp = self
            .client
            .get("https://api.twitter.com/2/users/me?user.fields=public_metrics")
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| format!("Twitter engagement failed: {e}"))?;

        let user: UserResponse = resp
            .json()
            .await
            .map_err(|e| format!("Twitter engagement parse: {e}"))?;

        let metrics = user.data.and_then(|d| d.public_metrics);
        let followers = metrics.as_ref().and_then(|m| m.followers_count).unwrap_or(0);
        let posts_count = metrics.as_ref().and_then(|m| m.tweet_count).unwrap_or(0);

        Ok(EngagementMetrics {
            platform: "twitter".into(),
            followers,
            posts_count,
            likes_total: 0,     // requires per-tweet aggregation
            replies_total: 0,   // requires per-tweet aggregation
            impressions_total: 0,
            engagement_rate: 0.0,
            period: format!("{period_days}d"),
        })
    }

    async fn search(&self, query: &str, limit: u32) -> Result<Vec<SearchResult>, String> {
        if !self.connected {
            return Err("Twitter not connected".into());
        }

        let encoded = urlencoding::encode(query);
        let url = format!(
            "https://api.twitter.com/2/tweets/search/recent?query={encoded}&max_results={limit}&tweet.fields=author_id,created_at"
        );

        let resp = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| format!("Twitter search failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Twitter search {status}: {text}"));
        }

        let data: SearchTweetsResponse = resp
            .json()
            .await
            .map_err(|e| format!("Twitter search parse: {e}"))?;

        let results = data
            .data
            .unwrap_or_default()
            .into_iter()
            .map(|t| {
                let tid = t.id.unwrap_or_default();
                SearchResult {
                    id: tid.clone(),
                    platform: "twitter".into(),
                    title: t.text.unwrap_or_default(),
                    url: format!("https://twitter.com/i/web/status/{tid}"),
                    author: t.author_id.unwrap_or_else(|| "unknown".into()),
                    score: None,
                    created_at: t.created_at.unwrap_or_default(),
                }
            })
            .collect();

        Ok(results)
    }

    async fn delete_post(&self, post_id: &str) -> Result<(), String> {
        if !self.connected {
            return Err("Twitter not connected".into());
        }

        // DELETE https://api.twitter.com/2/tweets/:id
        let url = format!("https://api.twitter.com/2/tweets/{post_id}");
        let resp = self
            .client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| format!("Twitter delete failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Twitter delete {status}: {text}"));
        }

        Ok(())
    }
}

// Suppress unused-field warnings — these are stored for future OAuth 1.0a signing.
impl TwitterConnector {
    #[allow(dead_code)]
    fn _api_key(&self) -> &str {
        &self.api_key
    }
    #[allow(dead_code)]
    fn _api_secret(&self) -> &str {
        &self.api_secret
    }
    #[allow(dead_code)]
    fn _access_token(&self) -> &str {
        &self.access_token
    }
    #[allow(dead_code)]
    fn _access_secret(&self) -> &str {
        &self.access_secret
    }
}
