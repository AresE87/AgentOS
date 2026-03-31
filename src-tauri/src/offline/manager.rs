use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineStatus {
    pub is_online: bool,
    pub cached_responses: u32,
    pub pending_sync: u32,
    pub last_online: Option<String>,
    pub sync_state: String,
    pub last_sync_at: Option<String>,
    pub last_sync_error: Option<String>,
    pub connectivity_source: String,
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

/// Manages offline-first functionality: connectivity detection, cached task
/// responses, and a persistent local queue for actions that must be replayed.
pub struct OfflineManager {
    is_online: bool,
    cache: Vec<CachedResponse>,
    pending: Vec<PendingSyncItem>,
    last_online: Option<String>,
    connectivity_override: Option<bool>,
    last_sync_at: Option<String>,
    last_sync_error: Option<String>,
}

impl OfflineManager {
    pub fn new() -> Self {
        Self {
            is_online: true,
            cache: Vec::new(),
            pending: Vec::new(),
            last_online: Some(chrono::Utc::now().to_rfc3339()),
            connectivity_override: None,
            last_sync_at: None,
            last_sync_error: None,
        }
    }

    /// Initialize SQLite tables for offline persistence.
    pub fn init_db(db: &Connection) -> Result<(), String> {
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

    pub fn load_from_db(&mut self, db: &Connection) -> Result<(), String> {
        Self::init_db(db)?;

        let mut cache_stmt = db
            .prepare(
                "SELECT id, task, response, cached_at
                 FROM offline_cache
                 ORDER BY cached_at ASC",
            )
            .map_err(|e| e.to_string())?;
        self.cache = cache_stmt
            .query_map([], |row| {
                Ok(CachedResponse {
                    id: row.get(0)?,
                    task: row.get(1)?,
                    response: row.get(2)?,
                    cached_at: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        let mut pending_stmt = db
            .prepare(
                "SELECT id, action, payload, queued_at
                 FROM offline_pending
                 ORDER BY queued_at ASC",
            )
            .map_err(|e| e.to_string())?;
        self.pending = pending_stmt
            .query_map([], |row| {
                Ok(PendingSyncItem {
                    id: row.get(0)?,
                    action: row.get(1)?,
                    payload: row.get(2)?,
                    queued_at: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Check network connectivity by attempting to reach a well-known endpoint.
    pub async fn check_connectivity(&mut self) -> bool {
        if let Some(forced) = self.connectivity_override {
            self.is_online = forced;
            if forced {
                self.last_online = Some(chrono::Utc::now().to_rfc3339());
            }
            return forced;
        }

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

        if online {
            self.last_online = Some(chrono::Utc::now().to_rfc3339());
        }
        self.is_online = online;
        online
    }

    pub fn set_connectivity_override(&mut self, forced_online: Option<bool>) -> OfflineStatus {
        self.connectivity_override = forced_online;
        if let Some(is_online) = forced_online {
            self.is_online = is_online;
            if is_online {
                self.last_online = Some(chrono::Utc::now().to_rfc3339());
                self.last_sync_error = None;
            }
        }
        self.get_status()
    }

    /// Cache a response for a given task.
    pub fn cache_response(
        &mut self,
        db: &Connection,
        task: String,
        response: String,
    ) -> Result<CachedResponse, String> {
        let cached = CachedResponse {
            id: uuid::Uuid::new_v4().to_string(),
            task,
            response,
            cached_at: chrono::Utc::now().to_rfc3339(),
        };

        db.execute(
            "INSERT INTO offline_cache (id, task, response, cached_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![cached.id, cached.task, cached.response, cached.cached_at],
        )
        .map_err(|e| e.to_string())?;

        self.cache.push(cached.clone());
        if self.cache.len() > 500 {
            if let Some(evicted) = self.cache.first().cloned() {
                db.execute(
                    "DELETE FROM offline_cache WHERE id = ?1",
                    params![evicted.id],
                )
                .map_err(|e| e.to_string())?;
            }
            self.cache.remove(0);
        }

        Ok(cached)
    }

    /// Get a cached response for a task query (simple substring match).
    pub fn get_cached(&self, task: &str) -> Option<CachedResponse> {
        let query = task.to_lowercase();
        self.cache
            .iter()
            .rev()
            .find(|c| {
                c.task.to_lowercase().contains(&query) || query.contains(&c.task.to_lowercase())
            })
            .cloned()
    }

    /// Get all items pending sync.
    pub fn get_pending_sync(&self) -> Vec<PendingSyncItem> {
        self.pending.clone()
    }

    /// Queue an action for sync when connectivity is restored.
    pub fn queue_for_sync(
        &mut self,
        db: &Connection,
        action: String,
        payload: String,
    ) -> Result<PendingSyncItem, String> {
        let item = PendingSyncItem {
            id: uuid::Uuid::new_v4().to_string(),
            action,
            payload,
            queued_at: chrono::Utc::now().to_rfc3339(),
        };

        db.execute(
            "INSERT INTO offline_pending (id, action, payload, queued_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![item.id, item.action, item.payload, item.queued_at],
        )
        .map_err(|e| e.to_string())?;

        self.pending.push(item.clone());
        Ok(item)
    }

    pub fn can_sync(&self) -> Result<(), String> {
        if !self.is_online {
            return Err("Cannot sync: device is offline".to_string());
        }
        Ok(())
    }

    pub fn mark_sync_success(&mut self, db: &Connection, id: &str) -> Result<(), String> {
        self.pending.retain(|item| item.id != id);
        db.execute("DELETE FROM offline_pending WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        self.last_online = Some(chrono::Utc::now().to_rfc3339());
        self.last_sync_at = Some(chrono::Utc::now().to_rfc3339());
        self.last_sync_error = None;
        Ok(())
    }

    pub fn mark_sync_failure(&mut self, error: String) {
        self.last_sync_at = Some(chrono::Utc::now().to_rfc3339());
        self.last_sync_error = Some(error);
    }

    fn sync_state(&self) -> String {
        if !self.is_online {
            "offline".to_string()
        } else if self.last_sync_error.is_some() && !self.pending.is_empty() {
            "sync_failed".to_string()
        } else if !self.pending.is_empty() {
            "pending_sync".to_string()
        } else {
            "online".to_string()
        }
    }

    /// Get current offline status summary.
    pub fn get_status(&self) -> OfflineStatus {
        OfflineStatus {
            is_online: self.is_online,
            cached_responses: self.cache.len() as u32,
            pending_sync: self.pending.len() as u32,
            last_online: self.last_online.clone(),
            sync_state: self.sync_state(),
            last_sync_at: self.last_sync_at.clone(),
            last_sync_error: self.last_sync_error.clone(),
            connectivity_source: if self.connectivity_override.is_some() {
                "forced".to_string()
            } else {
                "detected".to_string()
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crossapp::CrossAppBridge;

    fn memory_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        OfflineManager::init_db(&conn).unwrap();
        conn
    }

    #[test]
    fn queue_and_reload_pending_items_from_sqlite() {
        let conn = memory_db();
        let mut manager = OfflineManager::new();
        manager
            .queue_for_sync(
                &conn,
                "crossapp_csv_workflow".to_string(),
                "email,subject,body,event_title,start_time,end_time\nalice@example.com,Hi,Body,Sync,2026-04-02T09:00:00,2026-04-02T09:30:00".to_string(),
            )
            .unwrap();
        manager
            .cache_response(&conn, "demo".to_string(), "cached".to_string())
            .unwrap();

        let mut reloaded = OfflineManager::new();
        reloaded.load_from_db(&conn).unwrap();

        assert_eq!(reloaded.get_pending_sync().len(), 1);
        assert!(reloaded.get_cached("demo").is_some());
    }

    #[tokio::test]
    async fn queued_crossapp_workflow_syncs_when_connectivity_returns() {
        let conn = memory_db();
        let mut manager = OfflineManager::new();
        manager.set_connectivity_override(Some(false));

        let csv = "email,subject,body,event_title,start_time,end_time,location\nalice@example.com,Weekly sync,Agenda attached,Weekly Sync,2026-04-01T09:00:00,2026-04-01T09:30:00,Room A";
        let queued = manager
            .queue_for_sync(&conn, "crossapp_csv_workflow".to_string(), csv.to_string())
            .unwrap();

        assert_eq!(manager.get_status().sync_state, "offline");
        assert_eq!(manager.get_pending_sync().len(), 1);

        manager.set_connectivity_override(Some(true));
        manager.can_sync().unwrap();

        let mut bridge = CrossAppBridge::new();
        let run = bridge.run_csv_to_email_calendar(csv).await.unwrap();
        manager
            .cache_response(
                &conn,
                "crossapp_csv_workflow".to_string(),
                serde_json::to_string(&run).unwrap(),
            )
            .unwrap();
        manager.mark_sync_success(&conn, &queued.id).unwrap();

        println!(
            "C16 demo status={} synced_records={} pending_after={}",
            manager.get_status().sync_state,
            run.records_succeeded,
            manager.get_pending_sync().len()
        );

        assert_eq!(run.status, "completed");
        assert_eq!(bridge.workflow_history().len(), 1);
        assert_eq!(manager.get_pending_sync().len(), 0);
        assert_eq!(manager.get_status().sync_state, "online");
    }

    #[test]
    fn sync_failure_state_is_honest() {
        let conn = memory_db();
        let mut manager = OfflineManager::new();
        manager.set_connectivity_override(Some(true));
        manager
            .queue_for_sync(
                &conn,
                "crossapp_csv_workflow".to_string(),
                "bad payload".to_string(),
            )
            .unwrap();

        manager.mark_sync_failure("CSV parse failed".to_string());
        let status = manager.get_status();
        assert_eq!(status.sync_state, "sync_failed");
        assert_eq!(status.last_sync_error.as_deref(), Some("CSV parse failed"));
        assert_eq!(status.pending_sync, 1);
    }
}
