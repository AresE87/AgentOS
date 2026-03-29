use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct RateLimiter {
    windows: Arc<RwLock<HashMap<String, Vec<Instant>>>>,
    limits: RateLimits,
}

#[derive(Clone)]
pub struct RateLimits {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
}

impl RateLimits {
    pub fn free() -> Self {
        Self {
            requests_per_minute: 100,
            requests_per_hour: 1000,
        }
    }

    pub fn pro() -> Self {
        Self {
            requests_per_minute: 1000,
            requests_per_hour: 10000,
        }
    }

    pub fn team() -> Self {
        Self {
            requests_per_minute: 5000,
            requests_per_hour: 50000,
        }
    }
}

impl RateLimiter {
    pub fn new(limits: RateLimits) -> Self {
        Self {
            windows: Arc::new(RwLock::new(HashMap::new())),
            limits,
        }
    }

    /// Check if request is allowed for a given key (e.g., API key or IP)
    pub async fn check(&self, key: &str) -> Result<(), String> {
        let mut windows = self.windows.write().await;
        let timestamps = windows.entry(key.to_string()).or_default();

        let now = Instant::now();

        // Clean old entries
        timestamps.retain(|t| now.duration_since(*t) < Duration::from_secs(3600));

        // Check per-minute
        let last_minute = timestamps
            .iter()
            .filter(|t| now.duration_since(**t) < Duration::from_secs(60))
            .count() as u32;

        if last_minute >= self.limits.requests_per_minute {
            return Err(format!(
                "Rate limit exceeded: {} requests/minute (limit: {})",
                last_minute, self.limits.requests_per_minute
            ));
        }

        // Check per-hour
        let last_hour = timestamps.len() as u32;
        if last_hour >= self.limits.requests_per_hour {
            return Err(format!(
                "Rate limit exceeded: {} requests/hour (limit: {})",
                last_hour, self.limits.requests_per_hour
            ));
        }

        timestamps.push(now);
        Ok(())
    }

    /// Update limits (e.g., when plan changes)
    pub fn update_limits(&mut self, limits: RateLimits) {
        self.limits = limits;
    }

    /// Get current stats for a key
    pub async fn get_stats(&self, key: &str) -> (u32, u32) {
        let windows = self.windows.read().await;
        if let Some(timestamps) = windows.get(key) {
            let now = Instant::now();
            let per_min = timestamps
                .iter()
                .filter(|t| now.duration_since(**t) < Duration::from_secs(60))
                .count() as u32;
            let per_hour = timestamps
                .iter()
                .filter(|t| now.duration_since(**t) < Duration::from_secs(3600))
                .count() as u32;
            (per_min, per_hour)
        } else {
            (0, 0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn allows_requests_under_limit() {
        let limiter = RateLimiter::new(RateLimits::free());
        assert!(limiter.check("test_key").await.is_ok());
    }

    #[tokio::test]
    async fn tracks_stats() {
        let limiter = RateLimiter::new(RateLimits::free());
        limiter.check("test_key").await.unwrap();
        limiter.check("test_key").await.unwrap();
        let (per_min, per_hour) = limiter.get_stats("test_key").await;
        assert_eq!(per_min, 2);
        assert_eq!(per_hour, 2);
    }

    #[tokio::test]
    async fn enforces_per_minute_limit() {
        let limiter = RateLimiter::new(RateLimits {
            requests_per_minute: 2,
            requests_per_hour: 100,
        });
        assert!(limiter.check("k").await.is_ok());
        assert!(limiter.check("k").await.is_ok());
        assert!(limiter.check("k").await.is_err());
    }

    #[tokio::test]
    async fn separate_keys_independent() {
        let limiter = RateLimiter::new(RateLimits {
            requests_per_minute: 1,
            requests_per_hour: 100,
        });
        assert!(limiter.check("a").await.is_ok());
        assert!(limiter.check("b").await.is_ok());
        assert!(limiter.check("a").await.is_err());
    }
}
