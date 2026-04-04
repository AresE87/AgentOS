use async_trait::async_trait;
use serde::Deserialize;

use super::traits::{
    EngagementMetrics, Mention, PostResult, SearchResult, SocialPlatform, SocialPost,
};

pub struct RedditConnector {
    client: reqwest::Client,
    access_token: String,
    username: String,
    connected: bool,
}

// ── Reddit API response types ──────────────────────────────────────────────

#[derive(Deserialize)]
struct RedditSubmitResponse {
    json: Option<RedditSubmitJson>,
}

#[derive(Deserialize)]
struct RedditSubmitJson {
    data: Option<RedditSubmitData>,
}

#[derive(Deserialize)]
struct RedditSubmitData {
    id: Option<String>,
    url: Option<String>,
    #[allow(dead_code)]
    name: Option<String>,
}

#[derive(Deserialize)]
struct RedditAbout {
    data: Option<RedditUserData>,
}

#[derive(Deserialize)]
struct RedditUserData {
    link_karma: Option<u64>,
    comment_karma: Option<u64>,
}

#[derive(Deserialize)]
struct RedditListing {
    data: Option<RedditListingData>,
}

#[derive(Deserialize)]
struct RedditListingData {
    children: Option<Vec<RedditChild>>,
}

#[derive(Deserialize)]
struct RedditChild {
    data: Option<RedditChildData>,
}

#[derive(Deserialize)]
struct RedditChildData {
    id: Option<String>,
    title: Option<String>,
    selftext: Option<String>,
    author: Option<String>,
    permalink: Option<String>,
    score: Option<i64>,
    created_utc: Option<f64>,
    body: Option<String>,
    #[allow(dead_code)]
    subreddit: Option<String>,
}

impl RedditConnector {
    pub fn new(access_token: &str, username: &str) -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("AgentOS/1.0 (by /u/agentos)")
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            access_token: access_token.to_string(),
            username: username.to_string(),
            connected: !access_token.is_empty() && !username.is_empty(),
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.access_token)
    }
}

#[async_trait]
impl SocialPlatform for RedditConnector {
    fn name(&self) -> &str {
        "reddit"
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    async fn post(&self, post: &SocialPost) -> Result<PostResult, String> {
        if !self.connected {
            return Err("Reddit not connected".into());
        }

        // POST https://oauth.reddit.com/api/submit
        // Reddit requires a subreddit; extract from tags or use first tag.
        let subreddit = post
            .tags
            .first()
            .cloned()
            .unwrap_or_else(|| "test".into());

        let params = [
            ("kind", "self"),
            ("sr", &subreddit),
            ("title", &post.content),
            ("text", &post.content),
            ("api_type", "json"),
        ];

        let resp = self
            .client
            .post("https://oauth.reddit.com/api/submit")
            .header("Authorization", self.auth_header())
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Reddit submit failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Reddit API {status}: {text}"));
        }

        let data: RedditSubmitResponse = resp
            .json()
            .await
            .map_err(|e| format!("Reddit parse: {e}"))?;

        let inner = data.json.and_then(|j| j.data);
        let post_id = inner
            .as_ref()
            .and_then(|d| d.id.clone())
            .unwrap_or_else(|| "unknown".into());
        let url = inner
            .as_ref()
            .and_then(|d| d.url.clone())
            .unwrap_or_else(|| format!("https://reddit.com/r/{subreddit}"));

        Ok(PostResult {
            id: post_id,
            url,
            platform: "reddit".into(),
            posted_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    async fn reply(&self, post_id: &str, content: &str) -> Result<PostResult, String> {
        if !self.connected {
            return Err("Reddit not connected".into());
        }

        // POST https://oauth.reddit.com/api/comment
        // thing_id must be a fullname like t3_xxxxx (link) or t1_xxxxx (comment).
        let thing_id = if post_id.starts_with("t1_") || post_id.starts_with("t3_") {
            post_id.to_string()
        } else {
            format!("t3_{post_id}")
        };

        let params = [
            ("thing_id", thing_id.as_str()),
            ("text", content),
            ("api_type", "json"),
        ];

        let resp = self
            .client
            .post("https://oauth.reddit.com/api/comment")
            .header("Authorization", self.auth_header())
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Reddit comment failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Reddit comment {status}: {text}"));
        }

        Ok(PostResult {
            id: format!("comment-on-{post_id}"),
            url: format!("https://reddit.com/comments/{post_id}"),
            platform: "reddit".into(),
            posted_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    async fn get_mentions(&self, since_hours: u32) -> Result<Vec<Mention>, String> {
        if !self.connected {
            return Err("Reddit not connected".into());
        }

        // GET https://oauth.reddit.com/message/inbox — includes replies/mentions
        let resp = self
            .client
            .get("https://oauth.reddit.com/message/inbox?limit=50")
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| format!("Reddit inbox failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Reddit inbox {status}: {text}"));
        }

        let listing: RedditListing = resp
            .json()
            .await
            .map_err(|e| format!("Reddit inbox parse: {e}"))?;

        let cutoff =
            (chrono::Utc::now() - chrono::Duration::hours(i64::from(since_hours))).timestamp()
                as f64;

        let mentions = listing
            .data
            .and_then(|d| d.children)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|c| {
                let d = c.data?;
                let created = d.created_utc.unwrap_or(0.0);
                if created < cutoff {
                    return None;
                }
                let permalink = d.permalink.unwrap_or_default();
                Some(Mention {
                    id: d.id.unwrap_or_default(),
                    platform: "reddit".into(),
                    author: d.author.unwrap_or_else(|| "unknown".into()),
                    content: d.body.or(d.selftext).unwrap_or_default(),
                    url: format!("https://reddit.com{permalink}"),
                    mention_type: "reply".into(),
                    sentiment: None,
                    created_at: chrono::DateTime::from_timestamp(created as i64, 0)
                        .map(|dt| dt.to_rfc3339())
                        .unwrap_or_default(),
                    replied: false,
                })
            })
            .collect();

        Ok(mentions)
    }

    async fn get_engagement(&self, period_days: u32) -> Result<EngagementMetrics, String> {
        if !self.connected {
            return Err("Reddit not connected".into());
        }

        // GET https://oauth.reddit.com/user/{username}/about
        let url = format!(
            "https://oauth.reddit.com/user/{}/about",
            self.username
        );

        let resp = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| format!("Reddit about failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Reddit about {status}: {text}"));
        }

        let about: RedditAbout = resp
            .json()
            .await
            .map_err(|e| format!("Reddit about parse: {e}"))?;

        let link_karma = about
            .data
            .as_ref()
            .and_then(|d| d.link_karma)
            .unwrap_or(0);
        let comment_karma = about
            .data
            .as_ref()
            .and_then(|d| d.comment_karma)
            .unwrap_or(0);

        Ok(EngagementMetrics {
            platform: "reddit".into(),
            followers: 0, // Reddit does not expose follower count via API
            posts_count: 0,
            likes_total: link_karma + comment_karma,
            replies_total: 0,
            impressions_total: 0,
            engagement_rate: 0.0,
            period: format!("{period_days}d"),
        })
    }

    async fn search(&self, query: &str, limit: u32) -> Result<Vec<SearchResult>, String> {
        if !self.connected {
            return Err("Reddit not connected".into());
        }

        // GET https://oauth.reddit.com/search?q=...&limit=...&type=link
        let encoded = urlencoding::encode(query);
        let url = format!(
            "https://oauth.reddit.com/search?q={encoded}&limit={limit}&type=link&sort=relevance"
        );

        let resp = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| format!("Reddit search failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Reddit search {status}: {text}"));
        }

        let listing: RedditListing = resp
            .json()
            .await
            .map_err(|e| format!("Reddit search parse: {e}"))?;

        let results = listing
            .data
            .and_then(|d| d.children)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|c| {
                let d = c.data?;
                let permalink = d.permalink.unwrap_or_default();
                Some(SearchResult {
                    id: d.id.unwrap_or_default(),
                    platform: "reddit".into(),
                    title: d.title.unwrap_or_default(),
                    url: format!("https://reddit.com{permalink}"),
                    author: d.author.unwrap_or_else(|| "unknown".into()),
                    score: d.score,
                    created_at: d
                        .created_utc
                        .and_then(|ts| chrono::DateTime::from_timestamp(ts as i64, 0))
                        .map(|dt| dt.to_rfc3339())
                        .unwrap_or_default(),
                })
            })
            .collect();

        Ok(results)
    }

    async fn delete_post(&self, post_id: &str) -> Result<(), String> {
        if !self.connected {
            return Err("Reddit not connected".into());
        }

        let thing_id = if post_id.starts_with("t3_") || post_id.starts_with("t1_") {
            post_id.to_string()
        } else {
            format!("t3_{post_id}")
        };

        let params = [("id", thing_id.as_str())];

        let resp = self
            .client
            .post("https://oauth.reddit.com/api/del")
            .header("Authorization", self.auth_header())
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Reddit delete failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Reddit delete {status}: {text}"));
        }

        Ok(())
    }
}
