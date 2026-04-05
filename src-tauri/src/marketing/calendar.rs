use crate::brain::Gateway;
use crate::config::Settings;
use chrono::Datelike;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

// ── Post type classification ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PostType {
    Value,
    Demo,
    Engagement,
    Trending,
}

impl PostType {
    pub fn as_str(&self) -> &str {
        match self {
            PostType::Value => "value",
            PostType::Demo => "demo",
            PostType::Engagement => "engagement",
            PostType::Trending => "trending",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "demo" => PostType::Demo,
            "engagement" => PostType::Engagement,
            "trending" => PostType::Trending,
            _ => PostType::Value,
        }
    }
}

// ── Post status lifecycle ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PostStatus {
    Draft,
    Approved,
    Scheduled,
    Published,
    Failed,
}

impl PostStatus {
    pub fn as_str(&self) -> &str {
        match self {
            PostStatus::Draft => "draft",
            PostStatus::Approved => "approved",
            PostStatus::Scheduled => "scheduled",
            PostStatus::Published => "published",
            PostStatus::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "approved" => PostStatus::Approved,
            "scheduled" => PostStatus::Scheduled,
            "published" => PostStatus::Published,
            "failed" => PostStatus::Failed,
            _ => PostStatus::Draft,
        }
    }
}

// ── PlannedPost — SQLite-backed calendar entry ───────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedPost {
    pub id: String,
    pub platform: String,
    pub content: String,
    pub hashtags: Vec<String>,
    pub post_type: PostType,
    pub publish_at: String,
    pub status: PostStatus,
    pub platform_post_id: Option<String>,
    pub retry_count: u32,
    pub created_at: String,
}

// ── PostMetrics — engagement data for a published post ──────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostMetrics {
    pub post_id: String,
    pub likes: u64,
    pub replies: u64,
    pub reposts: u64,
    pub impressions: u64,
    pub fetched_at: String,
}

// ── ContentCalendar — SQLite-backed editorial calendar ──────────────────

pub struct ContentCalendar;

impl ContentCalendar {
    // ── Schema ───────────────────────────────────────────────────────────

    /// Create the content_calendar, post_metrics, and response_log tables
    /// if they do not already exist.
    pub fn ensure_tables(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS content_calendar (
                id TEXT PRIMARY KEY,
                platform TEXT NOT NULL,
                content TEXT NOT NULL,
                hashtags TEXT NOT NULL DEFAULT '[]',
                post_type TEXT NOT NULL DEFAULT 'value',
                publish_at TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'draft',
                platform_post_id TEXT,
                retry_count INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_cc_status ON content_calendar(status);
            CREATE INDEX IF NOT EXISTS idx_cc_publish_at ON content_calendar(publish_at);

            CREATE TABLE IF NOT EXISTS post_metrics (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                post_id TEXT NOT NULL,
                likes INTEGER NOT NULL DEFAULT 0,
                replies INTEGER NOT NULL DEFAULT 0,
                reposts INTEGER NOT NULL DEFAULT 0,
                impressions INTEGER NOT NULL DEFAULT 0,
                fetched_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (post_id) REFERENCES content_calendar(id)
            );

            CREATE INDEX IF NOT EXISTS idx_pm_post ON post_metrics(post_id);

            CREATE TABLE IF NOT EXISTS response_log (
                id TEXT PRIMARY KEY,
                mention_id TEXT NOT NULL,
                platform TEXT NOT NULL,
                author TEXT NOT NULL,
                original_text TEXT NOT NULL,
                classification TEXT NOT NULL,
                response_text TEXT NOT NULL,
                confidence REAL NOT NULL DEFAULT 0.0,
                status TEXT NOT NULL DEFAULT 'pending',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_rl_status ON response_log(status);
            CREATE INDEX IF NOT EXISTS idx_rl_mention ON response_log(mention_id);",
        )
        .map_err(|e| e.to_string())
    }

    // ── Weekly plan generation via LLM ──────────────────────────────────

    /// Use the LLM Gateway to generate 15 posts distributed across
    /// Twitter (8), LinkedIn (4), Reddit (3) over Mon-Fri at optimal hours,
    /// then insert them as Draft in content_calendar.
    ///
    /// Uses `db_path` instead of a `Connection` reference to avoid holding
    /// a non-Send rusqlite handle across `.await` points.
    pub async fn generate_weekly_plan(
        db_path: &std::path::Path,
        gateway: &Gateway,
        settings: &Settings,
    ) -> Result<Vec<PlannedPost>, String> {
        // Phase 0: ensure tables (sync — conn dropped before await)
        {
            let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
            Self::ensure_tables(&conn)?;
        }

        // Phase 1: call LLM (async — no Connection held)
        let prompt = r#"Generate a social media content plan for the upcoming week (Monday-Friday) for an AI productivity tool called AgentOS.

Create exactly 15 posts as a JSON array with these fields:
- "platform": one of "twitter", "linkedin", "reddit"
- "content": the post text (Twitter max 270 chars, LinkedIn 150-300 words professional, Reddit technical informative)
- "hashtags": array of relevant hashtags (2-3 for Twitter, 3-5 for LinkedIn, 1-2 for Reddit)
- "post_type": one of "value" (tips/insights), "demo" (feature showcase), "engagement" (questions/polls), "trending" (timely topic)
- "day": one of "Monday","Tuesday","Wednesday","Thursday","Friday"
- "time": optimal post time in HH:MM (Twitter 09:00-11:00, LinkedIn 08:00-10:00, Reddit 06:00-09:00)

Distribution:
- Twitter: 8 posts (short, punchy, with hooks)
- LinkedIn: 4 posts (professional, end with CTA or question)
- Reddit: 3 posts (technical, informative, not promotional)

Mix: 6 value, 4 demo, 3 engagement, 2 trending.

Respond ONLY with a valid JSON array, no markdown fences."#;

        let response = gateway
            .complete_with_system(
                prompt,
                Some("You are a social media strategist. Always respond with valid JSON arrays only, no extra text."),
                settings,
            )
            .await?;

        let text = response.content.trim();
        let json_start = text.find('[').unwrap_or(0);
        let json_end = text.rfind(']').map(|i| i + 1).unwrap_or(text.len());
        let json_slice = &text[json_start..json_end];

        let items: Vec<serde_json::Value> = serde_json::from_str(json_slice)
            .map_err(|e| format!("Failed to parse weekly plan JSON: {}", e))?;

        // Phase 2: build posts and persist (sync — fresh conn)
        let now = chrono::Utc::now();
        let days_since_monday = now.weekday().num_days_from_monday();
        let next_monday = now + chrono::Duration::days((7 - days_since_monday as i64) % 7);

        let day_offset = |d: &str| -> i64 {
            match d.to_lowercase().as_str() {
                "monday" => 0,
                "tuesday" => 1,
                "wednesday" => 2,
                "thursday" => 3,
                "friday" => 4,
                _ => 0,
            }
        };

        let mut posts = Vec::new();
        for item in &items {
            let day_str = item.get("day").and_then(|v| v.as_str()).unwrap_or("Monday");
            let time_str = item.get("time").and_then(|v| v.as_str()).unwrap_or("09:00");
            let offset = day_offset(day_str);

            let publish_date = next_monday + chrono::Duration::days(offset);
            let publish_at = format!("{}T{}:00Z", publish_date.format("%Y-%m-%d"), time_str);

            let hashtags: Vec<String> = item
                .get("hashtags")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            posts.push(PlannedPost {
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
                hashtags,
                post_type: PostType::from_str(
                    item.get("post_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("value"),
                ),
                publish_at,
                status: PostStatus::Draft,
                platform_post_id: None,
                retry_count: 0,
                created_at: chrono::Utc::now().to_rfc3339(),
            });
        }

        // Persist all posts to SQLite (fresh connection, no await)
        let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
        for post in &posts {
            let hashtags_json =
                serde_json::to_string(&post.hashtags).unwrap_or_else(|_| "[]".into());
            conn.execute(
                "INSERT INTO content_calendar \
                 (id, platform, content, hashtags, post_type, publish_at, status, retry_count, created_at) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![
                    post.id,
                    post.platform,
                    post.content,
                    hashtags_json,
                    post.post_type.as_str(),
                    post.publish_at,
                    post.status.as_str(),
                    post.retry_count,
                    post.created_at,
                ],
            )
            .map_err(|e| e.to_string())?;
        }

        Ok(posts)
    }

    // ── Approval helpers ─────────────────────────────────────────────────

    /// Move all Draft posts to Scheduled.
    pub fn approve_all(conn: &Connection) -> Result<u64, String> {
        Self::ensure_tables(conn)?;
        let changed = conn
            .execute(
                "UPDATE content_calendar SET status = 'scheduled' WHERE status = 'draft'",
                [],
            )
            .map_err(|e| e.to_string())?;
        Ok(changed as u64)
    }

    /// Approve a single post (Draft -> Scheduled).
    pub fn approve_post(conn: &Connection, id: &str) -> Result<(), String> {
        Self::ensure_tables(conn)?;
        conn.execute(
            "UPDATE content_calendar SET status = 'scheduled' WHERE id = ?1 AND status = 'draft'",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Edit a post's content before publishing.
    pub fn edit_post(conn: &Connection, id: &str, content: &str) -> Result<(), String> {
        Self::ensure_tables(conn)?;
        conn.execute(
            "UPDATE content_calendar SET content = ?1 WHERE id = ?2 AND status IN ('draft','scheduled')",
            params![content, id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Publishing helpers ──────────────────────────────────────────────

    /// Fetch all posts that are scheduled and whose publish_at <= now.
    pub fn fetch_due(conn: &Connection) -> Result<Vec<PlannedPost>, String> {
        Self::ensure_tables(conn)?;
        let now = chrono::Utc::now().to_rfc3339();
        let mut stmt = conn
            .prepare(
                "SELECT id, platform, content, hashtags, post_type, publish_at, status, \
                 platform_post_id, retry_count, created_at \
                 FROM content_calendar \
                 WHERE status = 'scheduled' AND publish_at <= ?1 \
                 ORDER BY publish_at ASC",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![now], |row| {
                let hashtags_str: String = row.get(3)?;
                let post_type_str: String = row.get(4)?;
                let status_str: String = row.get(6)?;
                Ok(PlannedPost {
                    id: row.get(0)?,
                    platform: row.get(1)?,
                    content: row.get(2)?,
                    hashtags: serde_json::from_str(&hashtags_str).unwrap_or_default(),
                    post_type: PostType::from_str(&post_type_str),
                    publish_at: row.get(5)?,
                    status: PostStatus::from_str(&status_str),
                    platform_post_id: row.get(7)?,
                    retry_count: row.get(8)?,
                    created_at: row.get(9)?,
                })
            })
            .map_err(|e| e.to_string())?;

        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Mark a post as successfully published.
    pub fn mark_published(
        conn: &Connection,
        id: &str,
        platform_post_id: &str,
    ) -> Result<(), String> {
        conn.execute(
            "UPDATE content_calendar SET status = 'published', platform_post_id = ?1 WHERE id = ?2",
            params![platform_post_id, id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Mark a post as failed and increment its retry counter.
    pub fn mark_failed(conn: &Connection, id: &str, _error: &str) -> Result<(), String> {
        conn.execute(
            "UPDATE content_calendar SET status = 'failed', retry_count = retry_count + 1 WHERE id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Query helpers ───────────────────────────────────────────────────

    /// Return all posts for a given week (start_date is ISO date like "2025-06-02").
    pub fn get_week(conn: &Connection, start_date: &str) -> Result<Vec<PlannedPost>, String> {
        Self::ensure_tables(conn)?;
        // Calculate 7 days from start
        let end_date = if let Ok(dt) =
            chrono::NaiveDate::parse_from_str(start_date, "%Y-%m-%d")
        {
            (dt + chrono::Duration::days(7)).format("%Y-%m-%d").to_string()
        } else {
            // Fallback: prefix match
            format!("{}z", start_date)
        };

        let mut stmt = conn
            .prepare(
                "SELECT id, platform, content, hashtags, post_type, publish_at, status, \
                 platform_post_id, retry_count, created_at \
                 FROM content_calendar \
                 WHERE publish_at >= ?1 AND publish_at < ?2 \
                 ORDER BY publish_at ASC",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![start_date, end_date], |row| {
                let hashtags_str: String = row.get(3)?;
                let post_type_str: String = row.get(4)?;
                let status_str: String = row.get(6)?;
                Ok(PlannedPost {
                    id: row.get(0)?,
                    platform: row.get(1)?,
                    content: row.get(2)?,
                    hashtags: serde_json::from_str(&hashtags_str).unwrap_or_default(),
                    post_type: PostType::from_str(&post_type_str),
                    publish_at: row.get(5)?,
                    status: PostStatus::from_str(&status_str),
                    platform_post_id: row.get(7)?,
                    retry_count: row.get(8)?,
                    created_at: row.get(9)?,
                })
            })
            .map_err(|e| e.to_string())?;

        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Get a single post by ID.
    pub fn get_post(conn: &Connection, id: &str) -> Result<Option<PlannedPost>, String> {
        Self::ensure_tables(conn)?;
        let mut stmt = conn
            .prepare(
                "SELECT id, platform, content, hashtags, post_type, publish_at, status, \
                 platform_post_id, retry_count, created_at \
                 FROM content_calendar WHERE id = ?1",
            )
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query_map(params![id], |row| {
                let hashtags_str: String = row.get(3)?;
                let post_type_str: String = row.get(4)?;
                let status_str: String = row.get(6)?;
                Ok(PlannedPost {
                    id: row.get(0)?,
                    platform: row.get(1)?,
                    content: row.get(2)?,
                    hashtags: serde_json::from_str(&hashtags_str).unwrap_or_default(),
                    post_type: PostType::from_str(&post_type_str),
                    publish_at: row.get(5)?,
                    status: PostStatus::from_str(&status_str),
                    platform_post_id: row.get(7)?,
                    retry_count: row.get(8)?,
                    created_at: row.get(9)?,
                })
            })
            .map_err(|e| e.to_string())?;

        Ok(rows.next().and_then(|r| r.ok()))
    }

    // ── Metrics helpers ─────────────────────────────────────────────────

    /// Record engagement metrics for a published post.
    pub fn record_metrics(conn: &Connection, metrics: &PostMetrics) -> Result<(), String> {
        Self::ensure_tables(conn)?;
        conn.execute(
            "INSERT INTO post_metrics (post_id, likes, replies, reposts, impressions, fetched_at) \
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                metrics.post_id,
                metrics.likes,
                metrics.replies,
                metrics.reposts,
                metrics.impressions,
                metrics.fetched_at,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Analyze engagement for the last 7 days of published posts.
    pub fn analyze_last_week(conn: &Connection) -> Result<serde_json::Value, String> {
        Self::ensure_tables(conn)?;
        let week_ago = (chrono::Utc::now() - chrono::Duration::days(7)).to_rfc3339();

        // Total posts published in the last 7 days
        let total_published: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM content_calendar WHERE status = 'published' AND created_at >= ?1",
                params![week_ago],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // Aggregate metrics for those posts
        let (total_likes, total_replies, total_reposts, total_impressions): (u64, u64, u64, u64) = conn
            .query_row(
                "SELECT COALESCE(SUM(pm.likes),0), COALESCE(SUM(pm.replies),0), \
                 COALESCE(SUM(pm.reposts),0), COALESCE(SUM(pm.impressions),0) \
                 FROM post_metrics pm \
                 JOIN content_calendar cc ON pm.post_id = cc.id \
                 WHERE cc.created_at >= ?1",
                params![week_ago],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap_or((0, 0, 0, 0));

        // Posts by platform
        let mut stmt = conn
            .prepare(
                "SELECT platform, COUNT(*) FROM content_calendar \
                 WHERE status = 'published' AND created_at >= ?1 GROUP BY platform",
            )
            .map_err(|e| e.to_string())?;
        let by_platform: Vec<serde_json::Value> = stmt
            .query_map(params![week_ago], |row| {
                let platform: String = row.get(0)?;
                let count: u64 = row.get(1)?;
                Ok(serde_json::json!({ "platform": platform, "count": count }))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        // Posts by type
        let mut stmt2 = conn
            .prepare(
                "SELECT post_type, COUNT(*) FROM content_calendar \
                 WHERE status = 'published' AND created_at >= ?1 GROUP BY post_type",
            )
            .map_err(|e| e.to_string())?;
        let by_type: Vec<serde_json::Value> = stmt2
            .query_map(params![week_ago], |row| {
                let ptype: String = row.get(0)?;
                let count: u64 = row.get(1)?;
                Ok(serde_json::json!({ "type": ptype, "count": count }))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        let engagement_rate = if total_impressions > 0 {
            ((total_likes + total_replies + total_reposts) as f64 / total_impressions as f64)
                * 100.0
        } else {
            0.0
        };

        Ok(serde_json::json!({
            "period": "last_7_days",
            "total_published": total_published,
            "total_likes": total_likes,
            "total_replies": total_replies,
            "total_reposts": total_reposts,
            "total_impressions": total_impressions,
            "engagement_rate": format!("{:.2}%", engagement_rate),
            "by_platform": by_platform,
            "by_type": by_type,
        }))
    }

    /// Summary JSON for the calendar state (used by frontend).
    pub fn to_json(conn: &Connection) -> Result<serde_json::Value, String> {
        Self::ensure_tables(conn)?;

        let total: u64 = conn
            .query_row("SELECT COUNT(*) FROM content_calendar", [], |row| row.get(0))
            .unwrap_or(0);
        let draft: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM content_calendar WHERE status = 'draft'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        let scheduled: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM content_calendar WHERE status = 'scheduled'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        let published: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM content_calendar WHERE status = 'published'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        let failed: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM content_calendar WHERE status = 'failed'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        Ok(serde_json::json!({
            "total_posts": total,
            "by_status": {
                "draft": draft,
                "scheduled": scheduled,
                "published": published,
                "failed": failed,
            },
        }))
    }
}

// ── Legacy shim: EditorialCalendar ──────────────────────────────────────
// Kept so that existing code referencing EditorialCalendar still compiles.
// Delegates to ContentCalendar under the hood.

use super::content::ScheduledPost;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorialCalendar {
    posts: Vec<ScheduledPost>,
}

impl EditorialCalendar {
    pub fn new() -> Self {
        Self { posts: Vec::new() }
    }

    pub fn add_post(&mut self, post: ScheduledPost) {
        self.posts.push(post);
    }

    pub fn get_week(&self, start: &str) -> Vec<&ScheduledPost> {
        self.posts
            .iter()
            .filter(|p| p.scheduled_for.starts_with(start))
            .collect()
    }

    pub fn get_due_posts(&self) -> Vec<&ScheduledPost> {
        self.posts
            .iter()
            .filter(|p| p.status == "scheduled")
            .collect()
    }

    pub fn mark_published(&mut self, id: &str) {
        if let Some(post) = self.posts.iter_mut().find(|p| p.id == id) {
            post.status = "published".to_string();
        }
    }

    pub fn mark_failed(&mut self, id: &str, error: &str) {
        if let Some(post) = self.posts.iter_mut().find(|p| p.id == id) {
            post.status = format!("failed: {}", error);
        }
    }

    pub fn get_post(&self, id: &str) -> Option<&ScheduledPost> {
        self.posts.iter().find(|p| p.id == id)
    }

    pub fn all_posts(&self) -> &[ScheduledPost] {
        &self.posts
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "total_posts": self.posts.len(),
            "by_status": {
                "draft": self.posts.iter().filter(|p| p.status == "draft").count(),
                "scheduled": self.posts.iter().filter(|p| p.status == "scheduled").count(),
                "published": self.posts.iter().filter(|p| p.status == "published").count(),
                "failed": self.posts.iter().filter(|p| p.status.starts_with("failed")).count(),
            },
            "posts": serde_json::to_value(&self.posts).unwrap_or(serde_json::Value::Array(vec![])),
        })
    }
}

impl Default for EditorialCalendar {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_post(id: &str, status: &str, scheduled: &str) -> ScheduledPost {
        ScheduledPost {
            id: id.to_string(),
            platform: "twitter".to_string(),
            content: format!("Post {}", id),
            scheduled_for: scheduled.to_string(),
            status: status.to_string(),
            tags: vec![],
        }
    }

    #[test]
    fn editorial_calendar_add_and_retrieve() {
        let mut cal = EditorialCalendar::new();
        cal.add_post(make_post("p1", "draft", "2025-01-06T09:00"));
        cal.add_post(make_post("p2", "scheduled", "2025-01-07T10:00"));
        assert_eq!(cal.all_posts().len(), 2);
    }

    #[test]
    fn editorial_calendar_get_week_filters() {
        let mut cal = EditorialCalendar::new();
        cal.add_post(make_post("p1", "draft", "2025-01-06T09:00"));
        cal.add_post(make_post("p2", "draft", "2025-01-07T10:00"));
        cal.add_post(make_post("p3", "draft", "2025-02-01T10:00"));
        let week = cal.get_week("2025-01-0");
        assert_eq!(week.len(), 2);
    }

    #[test]
    fn editorial_calendar_due_posts() {
        let mut cal = EditorialCalendar::new();
        cal.add_post(make_post("p1", "draft", "2025-01-06T09:00"));
        cal.add_post(make_post("p2", "scheduled", "2025-01-07T10:00"));
        let due = cal.get_due_posts();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].id, "p2");
    }

    #[test]
    fn content_calendar_ensure_tables() {
        let conn = Connection::open_in_memory().unwrap();
        ContentCalendar::ensure_tables(&conn).unwrap();
        // Verify we can insert
        conn.execute(
            "INSERT INTO content_calendar (id, platform, content, publish_at) VALUES ('t1','twitter','hello','2025-01-06T09:00:00Z')",
            [],
        )
        .unwrap();
        let count: u64 = conn
            .query_row("SELECT COUNT(*) FROM content_calendar", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn content_calendar_approve_and_fetch() {
        let conn = Connection::open_in_memory().unwrap();
        ContentCalendar::ensure_tables(&conn).unwrap();

        // Insert a draft post with a past publish_at
        conn.execute(
            "INSERT INTO content_calendar (id, platform, content, publish_at, status) \
             VALUES ('p1','twitter','Test post','2020-01-01T00:00:00Z','draft')",
            [],
        )
        .unwrap();

        // Approve all
        let changed = ContentCalendar::approve_all(&conn).unwrap();
        assert_eq!(changed, 1);

        // Fetch due (publish_at in the past)
        let due = ContentCalendar::fetch_due(&conn).unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].id, "p1");
        assert_eq!(due[0].status, PostStatus::Scheduled);
    }

    #[test]
    fn content_calendar_mark_published_and_failed() {
        let conn = Connection::open_in_memory().unwrap();
        ContentCalendar::ensure_tables(&conn).unwrap();

        conn.execute(
            "INSERT INTO content_calendar (id, platform, content, publish_at, status) \
             VALUES ('p1','twitter','Post 1','2020-01-01T00:00:00Z','scheduled')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO content_calendar (id, platform, content, publish_at, status) \
             VALUES ('p2','linkedin','Post 2','2020-01-01T00:00:00Z','scheduled')",
            [],
        )
        .unwrap();

        ContentCalendar::mark_published(&conn, "p1", "tweet_123").unwrap();
        let post = ContentCalendar::get_post(&conn, "p1").unwrap().unwrap();
        assert_eq!(post.status, PostStatus::Published);
        assert_eq!(post.platform_post_id, Some("tweet_123".to_string()));

        ContentCalendar::mark_failed(&conn, "p2", "rate limited").unwrap();
        let post2 = ContentCalendar::get_post(&conn, "p2").unwrap().unwrap();
        assert_eq!(post2.status, PostStatus::Failed);
        assert_eq!(post2.retry_count, 1);
    }

    #[test]
    fn content_calendar_to_json() {
        let conn = Connection::open_in_memory().unwrap();
        ContentCalendar::ensure_tables(&conn).unwrap();
        conn.execute(
            "INSERT INTO content_calendar (id, platform, content, publish_at, status) \
             VALUES ('p1','twitter','Post 1','2025-01-06T09:00:00Z','draft')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO content_calendar (id, platform, content, publish_at, status) \
             VALUES ('p2','twitter','Post 2','2025-01-07T09:00:00Z','scheduled')",
            [],
        )
        .unwrap();
        let json = ContentCalendar::to_json(&conn).unwrap();
        assert_eq!(json["total_posts"], 2);
        assert_eq!(json["by_status"]["draft"], 1);
        assert_eq!(json["by_status"]["scheduled"], 1);
    }
}
