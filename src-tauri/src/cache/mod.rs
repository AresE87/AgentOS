pub mod benchmarks;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppCache {
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
}

struct CacheEntry {
    value: serde_json::Value,
    expires_at: Instant,
}

impl AppCache {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get a cached value if still valid
    pub async fn get(&self, key: &str) -> Option<serde_json::Value> {
        let entries = self.entries.read().await;
        if let Some(entry) = entries.get(key) {
            if Instant::now() < entry.expires_at {
                return Some(entry.value.clone());
            }
        }
        None
    }

    /// Set a value with TTL
    pub async fn set(&self, key: &str, value: serde_json::Value, ttl: Duration) {
        let mut entries = self.entries.write().await;
        entries.insert(
            key.to_string(),
            CacheEntry {
                value,
                expires_at: Instant::now() + ttl,
            },
        );
    }

    /// Invalidate a specific key
    pub async fn invalidate(&self, key: &str) {
        let mut entries = self.entries.write().await;
        entries.remove(key);
    }

    /// Clear all expired entries
    pub async fn cleanup(&self) {
        let mut entries = self.entries.write().await;
        let now = Instant::now();
        entries.retain(|_, v| v.expires_at > now);
    }

    /// Clear everything
    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
    }

    /// Get cache stats: (total_entries, valid_entries)
    pub async fn stats(&self) -> (usize, usize) {
        let entries = self.entries.read().await;
        let now = Instant::now();
        let total = entries.len();
        let valid = entries.values().filter(|e| e.expires_at > now).count();
        (total, valid)
    }
}
