use async_trait::async_trait;
use serde::Deserialize;

use super::traits::{
    EngagementMetrics, Mention, PostResult, SearchResult, SocialPlatform, SocialPost,
};

pub struct HackerNewsConnector {
    client: reqwest::Client,
    username: String,
    password: String, // HN uses cookie-based auth
    connected: bool,
}

// ── HN Firebase + Algolia response types ───────────────────────────────────

#[allow(dead_code)]
#[derive(Deserialize)]
struct HNItem {
    id: Option<u64>,
    title: Option<String>,
    #[serde(rename = "by")]
    author: Option<String>,
    text: Option<String>,
    url: Option<String>,
    score: Option<i64>,
    time: Option<u64>,
    kids: Option<Vec<u64>>,
}

#[derive(Deserialize)]
struct AlgoliaResponse {
    hits: Option<Vec<AlgoliaHit>>,
}

#[derive(Deserialize)]
struct AlgoliaHit {
    #[serde(rename = "objectID")]
    object_id: Option<String>,
    title: Option<String>,
    author: Option<String>,
    url: Option<String>,
    points: Option<i64>,
    created_at: Option<String>,
    story_text: Option<String>,
    comment_text: Option<String>,
}

impl HackerNewsConnector {
    pub fn new(username: &str, password: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            username: username.to_string(),
            password: password.to_string(),
            connected: !username.is_empty(),
        }
    }
}

#[async_trait]
impl SocialPlatform for HackerNewsConnector {
    fn name(&self) -> &str {
        "hackernews"
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    async fn post(&self, _post: &SocialPost) -> Result<PostResult, String> {
        // HN does not have a public write API.  Posting requires browser-based
        // cookie authentication (login → get cookie → POST to /submit form).
        // For M8-1 we document this limitation and return an actionable error.
        Err("HackerNews posting requires browser automation — not available in this version. \
             Use the HN web interface to post, then track engagement here."
            .into())
    }

    async fn reply(&self, _post_id: &str, _content: &str) -> Result<PostResult, String> {
        Err("HackerNews reply requires browser automation — not available in this version.".into())
    }

    async fn get_mentions(&self, since_hours: u32) -> Result<Vec<Mention>, String> {
        if !self.connected {
            return Err("HackerNews not connected".into());
        }

        // Use Algolia search API to find recent mentions of the username.
        let encoded = urlencoding::encode(&self.username);
        let url = format!(
            "https://hn.algolia.com/api/v1/search?query={encoded}&tags=comment&numericFilters=created_at_i>{}",
            (chrono::Utc::now() - chrono::Duration::hours(i64::from(since_hours))).timestamp()
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("HN mentions search failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("HN Algolia {status}: {text}"));
        }

        let data: AlgoliaResponse = resp
            .json()
            .await
            .map_err(|e| format!("HN mentions parse: {e}"))?;

        let mentions = data
            .hits
            .unwrap_or_default()
            .into_iter()
            .map(|h| {
                let oid = h.object_id.unwrap_or_default();
                Mention {
                    id: oid.clone(),
                    platform: "hackernews".into(),
                    author: h.author.unwrap_or_else(|| "unknown".into()),
                    content: h
                        .comment_text
                        .or(h.story_text)
                        .unwrap_or_default(),
                    url: format!("https://news.ycombinator.com/item?id={oid}"),
                    mention_type: "comment".into(),
                    sentiment: None,
                    created_at: h.created_at.unwrap_or_default(),
                    replied: false,
                }
            })
            .collect();

        Ok(mentions)
    }

    async fn get_engagement(&self, period_days: u32) -> Result<EngagementMetrics, String> {
        if !self.connected {
            return Err("HackerNews not connected".into());
        }

        // Fetch user profile via Firebase API
        let url = format!(
            "https://hacker-news.firebaseio.com/v0/user/{}.json",
            self.username
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("HN user fetch failed: {e}"))?;

        #[derive(Deserialize)]
        struct HNUser {
            karma: Option<u64>,
            submitted: Option<Vec<u64>>,
        }

        let user: HNUser = resp
            .json()
            .await
            .map_err(|e| format!("HN user parse: {e}"))?;

        let karma = user.karma.unwrap_or(0);
        let submissions = user.submitted.map(|s| s.len() as u64).unwrap_or(0);

        Ok(EngagementMetrics {
            platform: "hackernews".into(),
            followers: 0, // HN has no follower concept
            posts_count: submissions,
            likes_total: karma,
            replies_total: 0,
            impressions_total: 0,
            engagement_rate: 0.0,
            period: format!("{period_days}d"),
        })
    }

    async fn search(&self, query: &str, limit: u32) -> Result<Vec<SearchResult>, String> {
        // Algolia search — no auth needed
        let encoded = urlencoding::encode(query);
        let url = format!(
            "https://hn.algolia.com/api/v1/search?query={encoded}&hitsPerPage={limit}"
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("HN search failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("HN search {status}: {text}"));
        }

        let data: AlgoliaResponse = resp
            .json()
            .await
            .map_err(|e| format!("HN search parse: {e}"))?;

        let results = data
            .hits
            .unwrap_or_default()
            .into_iter()
            .map(|h| {
                let oid = h.object_id.unwrap_or_default();
                SearchResult {
                    id: oid.clone(),
                    platform: "hackernews".into(),
                    title: h.title.unwrap_or_default(),
                    url: h
                        .url
                        .unwrap_or_else(|| format!("https://news.ycombinator.com/item?id={oid}")),
                    author: h.author.unwrap_or_else(|| "unknown".into()),
                    score: h.points,
                    created_at: h.created_at.unwrap_or_default(),
                }
            })
            .collect();

        Ok(results)
    }

    async fn delete_post(&self, _post_id: &str) -> Result<(), String> {
        Err("HackerNews does not support post deletion via API.".into())
    }
}

// Suppress dead-code warnings for password — stored for future cookie-auth.
impl HackerNewsConnector {
    #[allow(dead_code)]
    fn _password(&self) -> &str {
        &self.password
    }
}
