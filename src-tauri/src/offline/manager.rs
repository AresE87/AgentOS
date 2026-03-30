use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineStatus {
    pub is_online: bool,
    pub cached_responses: u32,
    pub pending_sync: u32,
    pub last_online: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse {
    pub id: String,
    pub task: String,
    pub response: String,
    pub cached_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingSyncItem {
    pub id: String,
    pub action: String,
    pub payload: String,
    pub queued_at: String,
}

/// Manages offline-first functionality: connectivity detection, response caching,
/// and sync queue for pending operations.
pub struct OfflineManager {
    is_online: bool,
    cache: Vec<CachedResponse>,
    pending: Vec<PendingSyncItem>,
    last_online: Option<String>,
}

impl OfflineManager {
    pub fn new() -> Self {
        Self {
            is_online: true,
            cache: Vec::new(),
            pending: Vec::new(),
            last_online: Some(chrono::Utc::now().to_rfc3339()),
        }
    }

    /// Initialize SQLite table for offline cache persistence.
    pub fn init_db(db: &rusqlite::Connection) -> Result<(), String> {
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS offline_cache (
                id TEXT PRIMARY KEY,
                task TEXT NOT NULL,
                response TEXT NOT NULL,
                cached_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS offline_pending (
                id TEXT PRIMARY KEY,
                action TEXT NOT NULL,
                payload TEXT NOT NULL,
                queued_at TEXT NOT NULL
            );",
        )
        .map_err(|e| format!("Failed to init offline tables: {}", e))
    }

    /// Check network connectivity by attempting to reach a well-known endpoint.
    pub async fn check_connectivity(&mut self) -> bool {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build();

        let online = match client {
            Ok(c) => c
                .get("https://www.google.com/generate_204")
                .send()
                .await
                .is_ok(),
            Err(_) => false,
        };

        if online && !self.is_online {
            // Transitioning from offline to online
            self.last_online = Some(chrono::Utc::now().to_rfc3339());
        }
        self.is_online = online;
        if online {
            self.last_online = Some(chrono::Utc::now().to_rfc3339());
        }
        online
    }

    /// Cache a response for a given task.
    pub fn cache_response(&mut self, task: String, response: String) {
        let id = uuid::Uuid::new_v4().to_string();
        let cached_at = chrono::Utc::now().to_rfc3339();
        self.cache.push(CachedResponse {
            id,
            task,
            response,
            cached_at,
        });
        // Keep cache bounded
        if self.cache.len() > 500 {
            self.cache.remove(0);
        }
    }

    /// Get a cached response for a task query (simple substring match).
    pub fn get_cached(&self, task: &str) -> Option<CachedResponse> {
        let query = task.to_lowercase();
        self.cache
            .iter()
            .rev()
            .find(|c| c.task.to_lowercase().contains(&query) || query.contains(&c.task.to_lowercase()))
            .cloned()
    }

    /// Get all items pending sync.
    pub fn get_pending_sync(&self) -> Vec<PendingSyncItem> {
        self.pending.clone()
    }

    /// Queue an action for sync when connectivity is restored.
    pub fn queue_for_sync(&mut self, action: String, payload: String) {
        let id = uuid::Uuid::new_v4().to_string();
        let queued_at = chrono::Utc::now().to_rfc3339();
        self.pending.push(PendingSyncItem {
            id,
            action,
            payload,
            queued_at,
        });
    }

    /// Attempt to sync all pending items (clears the queue).
    pub fn sync_when_online(&mut self) -> Result<u32, String> {
        if !self.is_online {
            return Err("Cannot sync: device is offline".to_string());
        }
        let count = self.pending.len() as u32;
        self.pending.clear();
        self.last_online = Some(chrono::Utc::now().to_rfc3339());
        Ok(count)
    }

    /// Get current offline status summary.
    pub fn get_status(&self) -> OfflineStatus {
        OfflineStatus {
            is_online: self.is_online,
            cached_responses: self.cache.len() as u32,
            pending_sync: self.pending.len() as u32,
            last_online: self.last_online.clone(),
        }
    }
}
