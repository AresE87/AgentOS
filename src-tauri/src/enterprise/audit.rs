use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: String,
    pub event_type: String,
    pub user_id: String,
    pub org_id: Option<String>,
    pub details: String,
    pub ip_address: Option<String>,
}

pub struct AuditLog;

impl AuditLog {
    pub fn ensure_table(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS audit_log (
                id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                event_type TEXT NOT NULL,
                user_id TEXT NOT NULL DEFAULT 'local',
                org_id TEXT,
                details TEXT NOT NULL DEFAULT '{}',
                ip_address TEXT
            )",
        )
        .map_err(|e| e.to_string())
    }

    pub fn log(
        conn: &Connection,
        event_type: &str,
        details: serde_json::Value,
    ) -> Result<(), String> {
        Self::log_with_user(conn, event_type, "local", None, details)
    }

    pub fn log_with_user(
        conn: &Connection,
        event_type: &str,
        user_id: &str,
        org_id: Option<&str>,
        details: serde_json::Value,
    ) -> Result<(), String> {
        let id = uuid::Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().to_rfc3339();
        let details_str = details.to_string();

        conn.execute(
            "INSERT INTO audit_log (id, timestamp, event_type, user_id, org_id, details)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, timestamp, event_type, user_id, org_id, details_str],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_recent(conn: &Connection, limit: usize) -> Result<Vec<AuditEntry>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT id, timestamp, event_type, user_id, org_id, details, ip_address
                 FROM audit_log
                 ORDER BY timestamp DESC
                 LIMIT ?1",
            )
            .map_err(|e| e.to_string())?;

        let entries = stmt
            .query_map(rusqlite::params![limit as i64], |row| {
                Ok(AuditEntry {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    event_type: row.get(2)?,
                    user_id: row.get(3)?,
                    org_id: row.get(4)?,
                    details: row.get(5)?,
                    ip_address: row.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        Ok(entries)
    }

    pub fn get_by_event_type(
        conn: &Connection,
        event_type: &str,
        limit: usize,
    ) -> Result<Vec<AuditEntry>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT id, timestamp, event_type, user_id, org_id, details, ip_address
                 FROM audit_log
                 WHERE event_type = ?1
                 ORDER BY timestamp DESC
                 LIMIT ?2",
            )
            .map_err(|e| e.to_string())?;

        let entries = stmt
            .query_map(rusqlite::params![event_type, limit as i64], |row| {
                Ok(AuditEntry {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    event_type: row.get(2)?,
                    user_id: row.get(3)?,
                    org_id: row.get(4)?,
                    details: row.get(5)?,
                    ip_address: row.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        Ok(entries)
    }

    pub fn export_csv(entries: &[AuditEntry]) -> String {
        let mut csv = String::from("timestamp,event_type,user_id,details\n");
        for e in entries {
            // Escape double quotes in fields by doubling them
            let timestamp = e.timestamp.replace('"', "\"\"");
            let event_type = e.event_type.replace('"', "\"\"");
            let user_id = e.user_id.replace('"', "\"\"");
            let details = e.details.replace('"', "\"\"");
            csv.push_str(&format!(
                "\"{}\",\"{}\",\"{}\",\"{}\"\n",
                timestamp, event_type, user_id, details
            ));
        }
        csv
    }
}
