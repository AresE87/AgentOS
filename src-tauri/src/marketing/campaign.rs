use super::content::ScheduledPost;
use crate::social::manager::SocialManager;
use crate::social::traits::{PostResult, SocialPost};
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

    // ── SQLite persistence ────────────────────────────────────────────

    /// Ensure the marketing_campaigns table exists.
    pub fn ensure_table(conn: &rusqlite::Connection) -> Result<(), String> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS marketing_campaigns (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                platforms TEXT NOT NULL DEFAULT '[]',
                posts_json TEXT NOT NULL DEFAULT '[]',
                status TEXT NOT NULL DEFAULT 'draft',
                created_at TEXT NOT NULL,
                metrics_json TEXT
            )",
        )
        .map_err(|e| e.to_string())
    }

    /// Persist a single campaign to SQLite.
    pub fn save(conn: &rusqlite::Connection, campaign: &Campaign) -> Result<(), String> {
        Self::ensure_table(conn)?;
        let platforms =
            serde_json::to_string(&campaign.platforms).unwrap_or_else(|_| "[]".into());
        let posts = serde_json::to_string(&campaign.posts).unwrap_or_else(|_| "[]".into());
        let metrics = campaign
            .metrics
            .as_ref()
            .map(|m| serde_json::to_string(m).unwrap_or_else(|_| "{}".into()));

        conn.execute(
            "INSERT OR REPLACE INTO marketing_campaigns \
             (id, name, description, platforms, posts_json, status, created_at, metrics_json) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            rusqlite::params![
                campaign.id,
                campaign.name,
                campaign.description,
                platforms,
                posts,
                campaign.status,
                campaign.created_at,
                metrics,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Load all campaigns from SQLite.
    pub fn load_all(conn: &rusqlite::Connection) -> Result<Vec<Campaign>, String> {
        Self::ensure_table(conn)?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, description, platforms, posts_json, \
                 status, created_at, metrics_json \
                 FROM marketing_campaigns ORDER BY created_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                let platforms_str: String = row.get(3)?;
                let posts_str: String = row.get(4)?;
                let metrics_str: Option<String> = row.get(7)?;
                Ok(Campaign {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    platforms: serde_json::from_str(&platforms_str).unwrap_or_default(),
                    posts: serde_json::from_str(&posts_str).unwrap_or_default(),
                    status: row.get(5)?,
                    created_at: row.get(6)?,
                    metrics: metrics_str
                        .and_then(|s| serde_json::from_str(&s).ok()),
                })
            })
            .map_err(|e| e.to_string())?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    // ── In-memory operations (now also persist) ───────────────────────

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

    /// Create a campaign and persist it to SQLite.
    pub fn create_and_save(
        &mut self,
        name: &str,
        description: &str,
        platforms: Vec<String>,
        conn: &rusqlite::Connection,
    ) -> Result<Campaign, String> {
        let campaign = self.create(name, description, platforms);
        Self::save(conn, &campaign)?;
        Ok(campaign)
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

    /// Add posts and persist to SQLite.
    pub fn add_posts_and_save(
        &mut self,
        campaign_id: &str,
        posts: Vec<ScheduledPost>,
        conn: &rusqlite::Connection,
    ) -> Result<(), String> {
        self.add_posts(campaign_id, posts);
        if let Some(campaign) = self.campaigns.iter().find(|c| c.id == campaign_id) {
            Self::save(conn, campaign)?;
        }
        Ok(())
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

    /// Start a campaign and persist to SQLite.
    pub fn start_and_save(
        &mut self,
        campaign_id: &str,
        conn: &rusqlite::Connection,
    ) -> Result<(), String> {
        self.start(campaign_id);
        if let Some(campaign) = self.campaigns.iter().find(|c| c.id == campaign_id) {
            Self::save(conn, campaign)?;
        }
        Ok(())
    }

    pub fn pause(&mut self, campaign_id: &str) {
        if let Some(campaign) = self.campaigns.iter_mut().find(|c| c.id == campaign_id) {
            campaign.status = "paused".to_string();
        }
    }

    /// Pause a campaign and persist to SQLite.
    pub fn pause_and_save(
        &mut self,
        campaign_id: &str,
        conn: &rusqlite::Connection,
    ) -> Result<(), String> {
        self.pause(campaign_id);
        if let Some(campaign) = self.campaigns.iter().find(|c| c.id == campaign_id) {
            Self::save(conn, campaign)?;
        }
        Ok(())
    }

    pub fn complete(&mut self, campaign_id: &str) {
        if let Some(campaign) = self.campaigns.iter_mut().find(|c| c.id == campaign_id) {
            campaign.status = "completed".to_string();
        }
    }

    /// Complete a campaign and persist to SQLite.
    pub fn complete_and_save(
        &mut self,
        campaign_id: &str,
        conn: &rusqlite::Connection,
    ) -> Result<(), String> {
        self.complete(campaign_id);
        if let Some(campaign) = self.campaigns.iter().find(|c| c.id == campaign_id) {
            Self::save(conn, campaign)?;
        }
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&Campaign> {
        self.campaigns.iter().find(|c| c.id == id)
    }

    pub fn list(&self) -> &[Campaign] {
        &self.campaigns
    }

    /// Hydrate in-memory cache from SQLite on startup.
    pub fn load_from_db(&mut self, conn: &rusqlite::Connection) -> Result<(), String> {
        let loaded = Self::load_all(conn)?;
        self.campaigns = loaded;
        Ok(())
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "total_campaigns": self.campaigns.len(),
            "campaigns": serde_json::to_value(&self.campaigns).unwrap_or(serde_json::Value::Array(vec![])),
        })
    }

    // ── Scheduled Posts — auto-publish infrastructure ────────────────────

    /// Ensure the scheduled_posts table exists for auto-publishing.
    pub fn ensure_scheduled_posts_table(conn: &rusqlite::Connection) -> Result<(), String> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS scheduled_posts (
                id TEXT PRIMARY KEY,
                campaign_id TEXT,
                platform TEXT NOT NULL,
                content TEXT NOT NULL,
                tags TEXT NOT NULL DEFAULT '[]',
                scheduled_for TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'draft',
                post_url TEXT,
                error TEXT,
                created_at TEXT NOT NULL
            )",
        )
        .map_err(|e| e.to_string())
    }

    /// Insert a scheduled post into SQLite for later auto-publish.
    pub fn schedule_post(
        conn: &rusqlite::Connection,
        campaign_id: Option<&str>,
        post: &ScheduledPost,
    ) -> Result<(), String> {
        Self::ensure_scheduled_posts_table(conn)?;
        let tags_json =
            serde_json::to_string(&post.tags).unwrap_or_else(|_| "[]".into());
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR REPLACE INTO scheduled_posts \
             (id, campaign_id, platform, content, tags, scheduled_for, status, created_at) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            rusqlite::params![
                post.id,
                campaign_id,
                post.platform,
                post.content,
                tags_json,
                post.scheduled_for,
                post.status,
                now,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Fetch due posts from SQLite (synchronous — no .await).
    pub fn fetch_due_posts(
        conn: &rusqlite::Connection,
    ) -> Result<Vec<(String, String, String, String)>, String> {
        Self::ensure_scheduled_posts_table(conn)?;
        let now = chrono::Utc::now().to_rfc3339();
        let mut stmt = conn
            .prepare(
                "SELECT id, platform, content, tags FROM scheduled_posts \
                 WHERE status = 'scheduled' AND scheduled_for <= ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params![now], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let collected: Vec<_> = rows.filter_map(|r| r.ok()).collect();
        Ok(collected)
    }

    /// Mark a scheduled post as published or failed (synchronous — no .await).
    pub fn mark_post_published(
        conn: &rusqlite::Connection,
        post_id: &str,
        post_url: &str,
    ) -> Result<(), String> {
        conn.execute(
            "UPDATE scheduled_posts SET status = 'published', post_url = ?1 WHERE id = ?2",
            rusqlite::params![post_url, post_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn mark_post_failed(
        conn: &rusqlite::Connection,
        post_id: &str,
        error: &str,
    ) -> Result<(), String> {
        conn.execute(
            "UPDATE scheduled_posts SET status = 'failed', error = ?1 WHERE id = ?2",
            rusqlite::params![error, post_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Check for scheduled posts that are due and publish them via SocialManager.
    ///
    /// Uses `db_path` instead of a `Connection` reference to avoid holding a
    /// non-Send rusqlite handle across `.await` points.
    pub async fn publish_due_posts(
        db_path: &std::path::Path,
        social_manager: &SocialManager,
    ) -> Result<Vec<PostResult>, String> {
        // Phase 1: collect due posts (sync — conn dropped before await)
        let due_posts = {
            let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
            Self::fetch_due_posts(&conn)?
        };

        // Phase 2: post to social platforms (async)
        let mut outcomes: Vec<(String, Result<PostResult, String>)> = Vec::new();
        for (id, platform, content, tags_json) in &due_posts {
            let tags: Vec<String> =
                serde_json::from_str(tags_json).unwrap_or_default();
            let social_post = SocialPost {
                content: content.clone(),
                media_url: None,
                reply_to: None,
                tags,
            };
            let platform_results =
                social_manager.post_to_all(&social_post, &[platform.clone()]).await;
            for (_plat, result) in platform_results {
                outcomes.push((id.clone(), result));
            }
        }

        // Phase 3: update DB with results (sync — fresh conn)
        let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
        let mut results = Vec::new();
        for (id, outcome) in outcomes {
            match outcome {
                Ok(pr) => {
                    Self::mark_post_published(&conn, &id, &pr.url).ok();
                    results.push(pr);
                }
                Err(e) => {
                    Self::mark_post_failed(&conn, &id, &e).ok();
                    tracing::warn!("Failed to publish scheduled post {}: {}", id, e);
                }
            }
        }

        Ok(results)
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

    #[test]
    fn sqlite_round_trip() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        CampaignManager::ensure_table(&conn).unwrap();

        let mut mgr = CampaignManager::new();
        let campaign = mgr.create_and_save("DB Test", "testing persistence", vec!["twitter".into()], &conn).unwrap();
        assert_eq!(campaign.name, "DB Test");

        let loaded = CampaignManager::load_all(&conn).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, campaign.id);
        assert_eq!(loaded[0].name, "DB Test");
    }
}
