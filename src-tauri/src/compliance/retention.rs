use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub retention_days: u32,
    pub auto_delete_enabled: bool,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            retention_days: 90,
            auto_delete_enabled: false,
        }
    }
}

impl RetentionPolicy {
    /// Apply retention policy — delete records older than retention_days
    pub fn apply(&self, conn: &Connection) -> Result<u64, String> {
        if !self.auto_delete_enabled || self.retention_days == 0 {
            return Ok(0);
        }

        let mut total = 0u64;
        let cutoff = format!("datetime('now', '-{} days')", self.retention_days);

        let tables_with_date = [
            ("tasks", "created_at"),
            ("feedback", "created_at"),
            ("audit_log", "timestamp"),
        ];

        for (table, date_col) in &tables_with_date {
            let exists: bool = conn
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{}'",
                        table
                    ),
                    [],
                    |row| row.get::<_, i64>(0).map(|n| n > 0),
                )
                .unwrap_or(false);

            if exists {
                let deleted = conn
                    .execute(
                        &format!("DELETE FROM {} WHERE {} < {}", table, date_col, cutoff),
                        [],
                    )
                    .map_err(|e| e.to_string())? as u64;
                total += deleted;
            }
        }

        Ok(total)
    }
}
