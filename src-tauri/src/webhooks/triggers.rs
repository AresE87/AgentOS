use serde::{Deserialize, Serialize};
use rusqlite::Connection;
use chrono::Utc;

// ── Data structures ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookTrigger {
    pub id: String,
    pub name: String,
    pub secret: String,
    pub filter: Option<String>,
    pub task_template: String,
    pub created_at: String,
    pub last_triggered: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    pub trigger_id: String,
    pub payload: serde_json::Value,
    pub received_at: String,
    pub task_queued: String,
}

// ── Manager ──────────────────────────────────────────────────────────

pub struct WebhookManager;

impl WebhookManager {
    pub fn ensure_tables(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS webhook_triggers (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                secret TEXT NOT NULL,
                filter TEXT,
                task_template TEXT NOT NULL,
                created_at TEXT NOT NULL,
                last_triggered TEXT
            );
            CREATE TABLE IF NOT EXISTS webhook_events (
                id TEXT PRIMARY KEY,
                trigger_id TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                received_at TEXT NOT NULL,
                task_queued TEXT NOT NULL
            );"
        ).map_err(|e| e.to_string())
    }

    pub fn create_trigger(conn: &Connection, name: &str, task_template: &str, filter: Option<&str>) -> Result<WebhookTrigger, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let secret = format!("whsec_{}", uuid::Uuid::new_v4().to_string().replace('-', ""));
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO webhook_triggers (id, name, secret, filter, task_template, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, name, secret, filter, task_template, now],
        ).map_err(|e| e.to_string())?;

        Ok(WebhookTrigger {
            id,
            name: name.to_string(),
            secret,
            filter: filter.map(|s| s.to_string()),
            task_template: task_template.to_string(),
            created_at: now,
            last_triggered: None,
        })
    }

    pub fn list_triggers(conn: &Connection) -> Result<Vec<WebhookTrigger>, String> {
        let mut stmt = conn
            .prepare("SELECT id, name, secret, filter, task_template, created_at, last_triggered FROM webhook_triggers ORDER BY created_at DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt.query_map([], |row| {
            Ok(WebhookTrigger {
                id: row.get(0)?,
                name: row.get(1)?,
                secret: row.get(2)?,
                filter: row.get(3)?,
                task_template: row.get(4)?,
                created_at: row.get(5)?,
                last_triggered: row.get(6)?,
            })
        }).map_err(|e| e.to_string())?;

        let mut triggers = Vec::new();
        for row in rows {
            if let Ok(t) = row {
                triggers.push(t);
            }
        }
        Ok(triggers)
    }

    pub fn get_trigger(conn: &Connection, id: &str) -> Result<WebhookTrigger, String> {
        let mut stmt = conn
            .prepare("SELECT id, name, secret, filter, task_template, created_at, last_triggered FROM webhook_triggers WHERE id = ?1")
            .map_err(|e| e.to_string())?;

        stmt.query_row(rusqlite::params![id], |row| {
            Ok(WebhookTrigger {
                id: row.get(0)?,
                name: row.get(1)?,
                secret: row.get(2)?,
                filter: row.get(3)?,
                task_template: row.get(4)?,
                created_at: row.get(5)?,
                last_triggered: row.get(6)?,
            })
        }).map_err(|e| format!("Webhook trigger not found: {}", e))
    }

    pub fn delete_trigger(conn: &Connection, id: &str) -> Result<bool, String> {
        let affected = conn
            .execute("DELETE FROM webhook_triggers WHERE id = ?1", rusqlite::params![id])
            .map_err(|e| e.to_string())?;
        Ok(affected > 0)
    }

    /// Validate an incoming webhook signature against the stored secret.
    /// Uses simple HMAC-like comparison (secret must match header value).
    pub fn validate_secret(conn: &Connection, trigger_id: &str, signature: &str) -> Result<bool, String> {
        let trigger = Self::get_trigger(conn, trigger_id)?;
        // Simple secret comparison — in production use HMAC-SHA256
        Ok(trigger.secret == signature)
    }

    /// Process an incoming webhook: validate, record event, queue task.
    pub fn trigger(conn: &Connection, trigger_id: &str, payload: serde_json::Value) -> Result<WebhookEvent, String> {
        let trigger = Self::get_trigger(conn, trigger_id)?;
        let now = Utc::now().to_rfc3339();

        // Apply filter if set
        if let Some(ref filter_key) = trigger.filter {
            if !filter_key.is_empty() {
                if payload.get(filter_key).is_none() {
                    return Err(format!("Payload does not contain required filter key: {}", filter_key));
                }
            }
        }

        // Build task description from template + payload
        let task_desc = trigger.task_template.replace("{payload}", &payload.to_string());
        let event_id = uuid::Uuid::new_v4().to_string();
        let payload_json = serde_json::to_string(&payload).map_err(|e| e.to_string())?;

        conn.execute(
            "INSERT INTO webhook_events (id, trigger_id, payload_json, received_at, task_queued) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![event_id, trigger_id, payload_json, now, task_desc],
        ).map_err(|e| e.to_string())?;

        // Update last_triggered timestamp
        conn.execute(
            "UPDATE webhook_triggers SET last_triggered = ?1 WHERE id = ?2",
            rusqlite::params![now, trigger_id],
        ).map_err(|e| e.to_string())?;

        tracing::info!("Webhook triggered: {} -> task queued: {}", trigger.name, task_desc);

        Ok(WebhookEvent {
            trigger_id: trigger_id.to_string(),
            payload,
            received_at: now,
            task_queued: task_desc,
        })
    }
}
