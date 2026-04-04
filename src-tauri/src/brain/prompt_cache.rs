//! Prompt-cache statistics tracker for Anthropic API responses.
//!
//! Anthropic's prompt caching allows the API to reuse previously-seen
//! system prompt and message prefix tokens, reducing latency and cost.
//! This module tracks cache creation vs. cache read tokens across
//! the lifetime of the application so callers can monitor efficiency.

use std::sync::atomic::{AtomicU64, Ordering};

/// Accumulates prompt-cache hit/miss statistics across API calls.
#[derive(Debug, Default)]
pub struct PromptCacheStats {
    pub cache_creation_tokens: AtomicU64,
    pub cache_read_tokens: AtomicU64,
    pub total_requests: AtomicU64,
}

impl PromptCacheStats {
    /// Record cache token counts from a single API response.
    pub fn record(&self, creation: u64, read: u64) {
        self.cache_creation_tokens
            .fetch_add(creation, Ordering::Relaxed);
        self.cache_read_tokens.fetch_add(read, Ordering::Relaxed);
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// Total tokens served from cache (i.e. tokens saved).
    pub fn tokens_saved(&self) -> u64 {
        self.cache_read_tokens.load(Ordering::Relaxed)
    }

    /// Return a JSON summary suitable for diagnostics / status endpoints.
    pub fn summary(&self) -> serde_json::Value {
        let creation = self.cache_creation_tokens.load(Ordering::Relaxed);
        let read = self.cache_read_tokens.load(Ordering::Relaxed);
        let requests = self.total_requests.load(Ordering::Relaxed);
        let hit_rate = if requests > 0 {
            (read as f64) / ((creation + read) as f64).max(1.0)
        } else {
            0.0
        };

        serde_json::json!({
            "cache_creation_tokens": creation,
            "cache_read_tokens": read,
            "total_requests": requests,
            "tokens_saved": self.tokens_saved(),
            "cache_hit_rate": format!("{:.1}%", hit_rate * 100.0),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_tokens() {
        let stats = PromptCacheStats::default();
        stats.record(1000, 0); // first call: cache creation
        stats.record(0, 1000); // second call: cache hit
        stats.record(0, 1000); // third call: cache hit

        assert_eq!(stats.cache_creation_tokens.load(Ordering::Relaxed), 1000);
        assert_eq!(stats.cache_read_tokens.load(Ordering::Relaxed), 2000);
        assert_eq!(stats.total_requests.load(Ordering::Relaxed), 3);
        assert_eq!(stats.tokens_saved(), 2000);
    }

    #[test]
    fn summary_is_valid_json() {
        let stats = PromptCacheStats::default();
        stats.record(500, 300);
        let summary = stats.summary();
        assert!(summary.get("cache_creation_tokens").is_some());
        assert!(summary.get("cache_hit_rate").is_some());
    }
}
