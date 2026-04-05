use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection};
use std::collections::HashMap;

// ── Per-platform rate limits ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PlatformLimit {
    pub max_per_hour: u32,
    pub max_per_day: u32,
    pub cooldown_secs: u64,
}

// ── Social Rate Limiter ─────────────────────────────────────────────────

pub struct SocialRateLimiter {
    limits: HashMap<String, PlatformLimit>,
    history: HashMap<String, Vec<DateTime<Utc>>>,
}

impl SocialRateLimiter {
    pub fn new() -> Self {
        let mut limits = HashMap::new();

        // Twitter: 5/hour, 15/day, 15 min cooldown
        limits.insert(
            "twitter".to_string(),
            PlatformLimit {
                max_per_hour: 5,
                max_per_day: 15,
                cooldown_secs: 900,
            },
        );

        // LinkedIn: 2/hour, 5/day, 1 hour cooldown
        limits.insert(
            "linkedin".to_string(),
            PlatformLimit {
                max_per_hour: 2,
                max_per_day: 5,
                cooldown_secs: 3600,
            },
        );

        // Reddit: 1/hour, 3/day, 30 min cooldown
        limits.insert(
            "reddit".to_string(),
            PlatformLimit {
                max_per_hour: 1,
                max_per_day: 3,
                cooldown_secs: 1800,
            },
        );

        Self {
            limits,
            history: HashMap::new(),
        }
    }

    /// Check if we can post to the given platform right now.
    pub fn can_post(&self, platform: &str) -> bool {
        let limit = match self.limits.get(platform) {
            Some(l) => l,
            None => return true, // No limit defined, allow
        };

        let history = match self.history.get(platform) {
            Some(h) => h,
            None => return true, // No history, allow
        };

        let now = Utc::now();

        // Check cooldown since last post
        if let Some(last) = history.last() {
            let elapsed = (now - *last).num_seconds();
            if elapsed < limit.cooldown_secs as i64 {
                return false;
            }
        }

        // Check hourly limit
        let one_hour_ago = now - Duration::hours(1);
        let posts_last_hour = history.iter().filter(|t| **t > one_hour_ago).count() as u32;
        if posts_last_hour >= limit.max_per_hour {
            return false;
        }

        // Check daily limit
        let one_day_ago = now - Duration::days(1);
        let posts_last_day = history.iter().filter(|t| **t > one_day_ago).count() as u32;
        if posts_last_day >= limit.max_per_day {
            return false;
        }

        true
    }

    /// Record that a post was made on the given platform.
    pub fn record_post(&mut self, platform: &str) {
        let entry = self.history.entry(platform.to_string()).or_default();
        entry.push(Utc::now());

        // Prune entries older than 24h to prevent unbounded growth
        let cutoff = Utc::now() - Duration::days(1);
        entry.retain(|t| *t > cutoff);
    }

    /// Return the next allowed posting time for the given platform,
    /// or None if posting is allowed right now.
    pub fn next_allowed_at(&self, platform: &str) -> Option<DateTime<Utc>> {
        if self.can_post(platform) {
            return None;
        }

        let limit = self.limits.get(platform)?;
        let history = self.history.get(platform)?;
        let now = Utc::now();

        let mut earliest = now + Duration::days(1); // Pessimistic default

        // Cooldown-based: next = last_post + cooldown
        if let Some(last) = history.last() {
            let cooldown_end = *last + Duration::seconds(limit.cooldown_secs as i64);
            if cooldown_end < earliest {
                earliest = cooldown_end;
            }
        }

        // Hourly limit: when the oldest post in the last hour expires
        let one_hour_ago = now - Duration::hours(1);
        let mut recent_hour: Vec<_> = history.iter().filter(|t| **t > one_hour_ago).collect();
        recent_hour.sort();
        if recent_hour.len() as u32 >= limit.max_per_hour {
            if let Some(oldest) = recent_hour.first() {
                let hourly_end = **oldest + Duration::hours(1);
                if hourly_end < earliest {
                    earliest = hourly_end;
                }
            }
        }

        // Daily limit: when the oldest post in the last day expires
        let one_day_ago = now - Duration::days(1);
        let mut recent_day: Vec<_> = history.iter().filter(|t| **t > one_day_ago).collect();
        recent_day.sort();
        if recent_day.len() as u32 >= limit.max_per_day {
            if let Some(oldest) = recent_day.first() {
                let daily_end = **oldest + Duration::days(1);
                if daily_end < earliest {
                    earliest = daily_end;
                }
            }
        }

        Some(earliest)
    }

    // ── Content safety checks (static, no &self) ────────────────────────

    /// Check if similar content was already posted (simple substring match
    /// against recent content_calendar entries).
    pub fn is_duplicate(conn: &Connection, content: &str) -> bool {
        // Normalize: lowercase, trim whitespace
        let normalized = content.trim().to_lowercase();
        if normalized.is_empty() {
            return false;
        }

        // Check for exact or near-exact match in last 7 days
        let week_ago = (Utc::now() - Duration::days(7)).to_rfc3339();
        let result: Result<u64, _> = conn.query_row(
            "SELECT COUNT(*) FROM content_calendar \
             WHERE LOWER(content) = ?1 AND created_at >= ?2",
            params![normalized, week_ago],
            |row| row.get(0),
        );
        matches!(result, Ok(count) if count > 0)
    }

    /// Count how many responses we have sent to a specific user on a platform
    /// within the given number of hours.
    pub fn response_count_to_user(
        conn: &Connection,
        platform: &str,
        author: &str,
        hours: u32,
    ) -> u32 {
        let cutoff = (Utc::now() - Duration::hours(hours as i64)).to_rfc3339();
        let result: Result<u32, _> = conn.query_row(
            "SELECT COUNT(*) FROM response_log \
             WHERE platform = ?1 AND author = ?2 AND status = 'sent' AND created_at >= ?3",
            params![platform, author, cutoff],
            |row| row.get(0),
        );
        result.unwrap_or(0)
    }
}

impl Default for SocialRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_limiter_allows_all() {
        let limiter = SocialRateLimiter::new();
        assert!(limiter.can_post("twitter"));
        assert!(limiter.can_post("linkedin"));
        assert!(limiter.can_post("reddit"));
        assert!(limiter.can_post("unknown_platform"));
    }

    #[test]
    fn next_allowed_returns_none_when_allowed() {
        let limiter = SocialRateLimiter::new();
        assert!(limiter.next_allowed_at("twitter").is_none());
    }

    #[test]
    fn record_and_cooldown() {
        let mut limiter = SocialRateLimiter::new();

        // Record a post
        limiter.record_post("twitter");

        // Immediately after, should be blocked by cooldown
        assert!(!limiter.can_post("twitter"));

        // Next allowed should be in the future
        let next = limiter.next_allowed_at("twitter");
        assert!(next.is_some());
        assert!(next.unwrap() > Utc::now());
    }

    #[test]
    fn hourly_limit_enforcement() {
        let mut limiter = SocialRateLimiter::new();

        // Reddit allows 1/hour — post once, then blocked
        limiter.record_post("reddit");
        // Manually push the timestamp back so cooldown passes but hourly doesn't
        if let Some(history) = limiter.history.get_mut("reddit") {
            // Set last post to 31 minutes ago (past 30min cooldown)
            history[0] = Utc::now() - Duration::minutes(31);
        }
        // Still blocked by hourly limit (1 post in last hour)
        assert!(!limiter.can_post("reddit"));
    }

    #[test]
    fn is_duplicate_checks_db() {
        let conn = Connection::open_in_memory().unwrap();
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
            )",
        )
        .unwrap();

        // No duplicates initially
        assert!(!SocialRateLimiter::is_duplicate(&conn, "Hello world"));

        // Insert a post
        conn.execute(
            "INSERT INTO content_calendar (id, platform, content, publish_at, created_at) \
             VALUES ('t1','twitter','Hello world','2025-01-06T09:00:00Z', datetime('now'))",
            [],
        )
        .unwrap();

        // Now it IS a duplicate (case-insensitive)
        assert!(SocialRateLimiter::is_duplicate(&conn, "hello world"));
        assert!(!SocialRateLimiter::is_duplicate(&conn, "goodbye world"));
    }

    #[test]
    fn response_count_to_user_checks_db() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS response_log (
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
            )",
        )
        .unwrap();

        assert_eq!(
            SocialRateLimiter::response_count_to_user(&conn, "twitter", "user1", 24),
            0
        );

        conn.execute(
            "INSERT INTO response_log (id, mention_id, platform, author, original_text, classification, response_text, confidence, status) \
             VALUES ('r1','m1','twitter','user1','hello','positive','thanks!',0.9,'sent')",
            [],
        )
        .unwrap();

        assert_eq!(
            SocialRateLimiter::response_count_to_user(&conn, "twitter", "user1", 24),
            1
        );
        assert_eq!(
            SocialRateLimiter::response_count_to_user(&conn, "twitter", "user2", 24),
            0
        );
    }
}
