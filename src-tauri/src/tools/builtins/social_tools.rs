use crate::tools::trait_def::*;

// ---------------------------------------------------------------------------
// social_post — publish content to one or more platforms
// ---------------------------------------------------------------------------
pub struct SocialPostTool;

#[async_trait::async_trait]
impl Tool for SocialPostTool {
    fn name(&self) -> &str {
        "social_post"
    }

    fn description(&self) -> &str {
        "Post content to social media platforms. Provide content text, target platforms, and optional tags. Posts are stored in the local database for tracking."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "content": { "type": "string", "description": "The post content text" },
                "platforms": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Target platforms (twitter, linkedin, reddit, hn)"
                },
                "tags": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional tags/hashtags for the post"
                }
            },
            "required": ["content", "platforms"]
        })
    }

    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::Dangerous
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<ToolOutput, ToolError> {
        let content = input
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError("Missing 'content' parameter".into()))?;

        let platforms: Vec<String> = input
            .get("platforms")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let tags: Vec<String> = input
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        if platforms.is_empty() {
            return Err(ToolError("At least one platform is required".into()));
        }

        // Store each post in the local database for tracking
        let conn = rusqlite::Connection::open(&ctx.db_path)
            .map_err(|e| ToolError(format!("DB connection failed: {}", e)))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS social_posts (
                id TEXT PRIMARY KEY,
                platform TEXT NOT NULL,
                content TEXT NOT NULL,
                tags TEXT,
                status TEXT DEFAULT 'queued',
                created_at TEXT DEFAULT (datetime('now'))
            );",
        )
        .map_err(|e| ToolError(format!("Table init failed: {}", e)))?;

        let mut posted = Vec::new();
        for platform in &platforms {
            let id = uuid::Uuid::new_v4().to_string();
            let tags_json = serde_json::to_string(&tags).unwrap_or_default();
            conn.execute(
                "INSERT INTO social_posts (id, platform, content, tags, status) VALUES (?1, ?2, ?3, ?4, 'queued')",
                rusqlite::params![id, platform, content, tags_json],
            )
            .map_err(|e| ToolError(format!("Insert failed: {}", e)))?;
            posted.push(format!("{} (id: {})", platform, id));
        }

        Ok(ToolOutput {
            content: format!(
                "Post queued for {} platform(s): {}",
                posted.len(),
                posted.join(", ")
            ),
            is_error: false,
        })
    }
}

// ---------------------------------------------------------------------------
// social_reply — reply to a specific social media post
// ---------------------------------------------------------------------------
pub struct SocialReplyTool;

#[async_trait::async_trait]
impl Tool for SocialReplyTool {
    fn name(&self) -> &str {
        "social_reply"
    }

    fn description(&self) -> &str {
        "Reply to a social media post or mention. Provide the platform, post ID, and reply content."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "platform": { "type": "string", "description": "The platform (twitter, linkedin, reddit)" },
                "post_id": { "type": "string", "description": "The ID of the post to reply to" },
                "content": { "type": "string", "description": "The reply content" }
            },
            "required": ["platform", "post_id", "content"]
        })
    }

    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::Dangerous
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<ToolOutput, ToolError> {
        let platform = input
            .get("platform")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError("Missing 'platform' parameter".into()))?;

        let post_id = input
            .get("post_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError("Missing 'post_id' parameter".into()))?;

        let content = input
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError("Missing 'content' parameter".into()))?;

        let conn = rusqlite::Connection::open(&ctx.db_path)
            .map_err(|e| ToolError(format!("DB connection failed: {}", e)))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS social_replies (
                id TEXT PRIMARY KEY,
                platform TEXT NOT NULL,
                parent_post_id TEXT NOT NULL,
                content TEXT NOT NULL,
                status TEXT DEFAULT 'queued',
                created_at TEXT DEFAULT (datetime('now'))
            );",
        )
        .map_err(|e| ToolError(format!("Table init failed: {}", e)))?;

        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO social_replies (id, platform, parent_post_id, content, status) VALUES (?1, ?2, ?3, ?4, 'queued')",
            rusqlite::params![id, platform, post_id, content],
        )
        .map_err(|e| ToolError(format!("Insert failed: {}", e)))?;

        Ok(ToolOutput {
            content: format!(
                "Reply queued on {} to post {} (reply id: {})",
                platform, post_id, id
            ),
            is_error: false,
        })
    }
}

// ---------------------------------------------------------------------------
// social_mentions — list recent mentions / interactions
// ---------------------------------------------------------------------------
pub struct SocialMentionsTool;

#[async_trait::async_trait]
impl Tool for SocialMentionsTool {
    fn name(&self) -> &str {
        "social_mentions"
    }

    fn description(&self) -> &str {
        "Retrieve recent social media mentions, comments, and interactions. Returns items from the last N hours."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "since_hours": {
                    "type": "integer",
                    "description": "Retrieve mentions from the last N hours (default: 24)",
                    "default": 24
                }
            }
        })
    }

    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::ReadOnly
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<ToolOutput, ToolError> {
        let since_hours = input
            .get("since_hours")
            .and_then(|v| v.as_i64())
            .unwrap_or(24);

        let conn = rusqlite::Connection::open(&ctx.db_path)
            .map_err(|e| ToolError(format!("DB connection failed: {}", e)))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS social_mentions (
                id TEXT PRIMARY KEY,
                platform TEXT NOT NULL,
                author TEXT NOT NULL,
                content TEXT NOT NULL,
                url TEXT,
                classification TEXT,
                created_at TEXT DEFAULT (datetime('now'))
            );",
        )
        .map_err(|e| ToolError(format!("Table init failed: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, platform, author, content, url, classification, created_at \
                 FROM social_mentions \
                 WHERE created_at >= datetime('now', ?1) \
                 ORDER BY created_at DESC LIMIT 50",
            )
            .map_err(|e| ToolError(format!("Query failed: {}", e)))?;

        let since_param = format!("-{} hours", since_hours);
        let rows: Vec<String> = stmt
            .query_map(rusqlite::params![since_param], |row| {
                let id: String = row.get(0)?;
                let platform: String = row.get(1)?;
                let author: String = row.get(2)?;
                let content: String = row.get(3)?;
                let url: Option<String> = row.get(4)?;
                let classification: Option<String> = row.get(5)?;
                let date: String = row.get(6)?;
                Ok(format!(
                    "[{}] {} @{}: {} {} [{}] ({})",
                    id,
                    platform,
                    author,
                    content,
                    url.unwrap_or_default(),
                    classification.unwrap_or_else(|| "unclassified".to_string()),
                    date,
                ))
            })
            .map_err(|e| ToolError(format!("Query failed: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();

        let output = if rows.is_empty() {
            format!(
                "No mentions found in the last {} hours.",
                since_hours
            )
        } else {
            format!(
                "Found {} mentions in the last {} hours:\n{}",
                rows.len(),
                since_hours,
                rows.join("\n")
            )
        };

        Ok(ToolOutput {
            content: output,
            is_error: false,
        })
    }
}

// ---------------------------------------------------------------------------
// social_engagement — retrieve engagement metrics
// ---------------------------------------------------------------------------
pub struct SocialEngagementTool;

#[async_trait::async_trait]
impl Tool for SocialEngagementTool {
    fn name(&self) -> &str {
        "social_engagement"
    }

    fn description(&self) -> &str {
        "Get social media engagement metrics (impressions, likes, shares, replies) for a given time period."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "period_days": {
                    "type": "integer",
                    "description": "Number of days to look back (default: 7)",
                    "default": 7
                }
            }
        })
    }

    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::ReadOnly
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<ToolOutput, ToolError> {
        let period_days = input
            .get("period_days")
            .and_then(|v| v.as_i64())
            .unwrap_or(7);

        let conn = rusqlite::Connection::open(&ctx.db_path)
            .map_err(|e| ToolError(format!("DB connection failed: {}", e)))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS social_engagement (
                id TEXT PRIMARY KEY,
                platform TEXT NOT NULL,
                post_id TEXT,
                impressions INTEGER DEFAULT 0,
                likes INTEGER DEFAULT 0,
                shares INTEGER DEFAULT 0,
                replies INTEGER DEFAULT 0,
                recorded_at TEXT DEFAULT (datetime('now'))
            );",
        )
        .map_err(|e| ToolError(format!("Table init failed: {}", e)))?;

        let since_param = format!("-{} days", period_days);
        let mut stmt = conn
            .prepare(
                "SELECT platform, \
                 COALESCE(SUM(impressions), 0) as total_impressions, \
                 COALESCE(SUM(likes), 0) as total_likes, \
                 COALESCE(SUM(shares), 0) as total_shares, \
                 COALESCE(SUM(replies), 0) as total_replies, \
                 COUNT(*) as post_count \
                 FROM social_engagement \
                 WHERE recorded_at >= datetime('now', ?1) \
                 GROUP BY platform",
            )
            .map_err(|e| ToolError(format!("Query failed: {}", e)))?;

        let rows: Vec<String> = stmt
            .query_map(rusqlite::params![since_param], |row| {
                let platform: String = row.get(0)?;
                let impressions: i64 = row.get(1)?;
                let likes: i64 = row.get(2)?;
                let shares: i64 = row.get(3)?;
                let replies: i64 = row.get(4)?;
                let count: i64 = row.get(5)?;
                Ok(format!(
                    "{}: {} posts, {} impressions, {} likes, {} shares, {} replies",
                    platform, count, impressions, likes, shares, replies
                ))
            })
            .map_err(|e| ToolError(format!("Query failed: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();

        let output = if rows.is_empty() {
            format!(
                "No engagement data found for the last {} days. Post some content first!",
                period_days
            )
        } else {
            format!(
                "Engagement metrics (last {} days):\n{}",
                period_days,
                rows.join("\n")
            )
        };

        Ok(ToolOutput {
            content: output,
            is_error: false,
        })
    }
}
