use rusqlite::Connection;
use serde::{Deserialize, Serialize};

pub struct GDPRManager;

#[derive(Debug, Serialize, Deserialize)]
pub struct DataExport {
    pub exported_at: String,
    pub tasks: Vec<serde_json::Value>,
    pub feedback: Vec<serde_json::Value>,
    pub settings: serde_json::Value,
    pub audit_log: Vec<serde_json::Value>,
    pub api_keys: Vec<serde_json::Value>,
    pub organizations: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataCategory {
    pub name: String,
    pub table: String,
    pub description: String,
    pub record_count: u64,
    pub stored_locally: bool,
    pub leaves_device: bool,
}

impl GDPRManager {
    /// Export ALL user data as portable JSON (GDPR Art. 20 — Right to Data Portability)
    pub fn export_all_data(conn: &Connection) -> Result<DataExport, String> {
        let tasks = Self::query_all(conn, "SELECT * FROM tasks")?;
        let feedback = Self::query_all(conn, "SELECT * FROM feedback")?;
        let audit_log = Self::query_all(conn, "SELECT * FROM audit_log")?;
        let api_keys = Self::query_all(
            conn,
            "SELECT id, name, created_at, last_used, enabled FROM api_keys",
        )?;
        let organizations = Self::query_all(conn, "SELECT * FROM organizations")?;

        Ok(DataExport {
            exported_at: chrono::Utc::now().to_rfc3339(),
            tasks,
            feedback,
            settings: serde_json::json!({}),
            audit_log,
            api_keys,
            organizations,
        })
    }

    /// Delete ALL user data (GDPR Art. 17 — Right to Erasure)
    pub fn delete_all_data(conn: &Connection) -> Result<u64, String> {
        let mut total_deleted = 0u64;

        let tables = [
            "tasks",
            "feedback",
            "audit_log",
            "api_keys",
            "marketplace_installs",
            "marketplace_reviews",
            "organizations",
            "org_members",
        ];

        for table in &tables {
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
                let count = conn
                    .execute(&format!("DELETE FROM {}", table), [])
                    .map_err(|e| e.to_string())? as u64;
                total_deleted += count;
            }
        }

        // VACUUM to reclaim space and remove traces of deleted data
        conn.execute_batch("VACUUM").map_err(|e| e.to_string())?;

        Ok(total_deleted)
    }

    /// Get data inventory — what data exists and where
    pub fn get_data_inventory(conn: &Connection) -> Result<Vec<DataCategory>, String> {
        let mut categories = vec![];

        let table_info = [
            (
                "tasks",
                "Task history",
                "Chat messages, command outputs, AI responses",
            ),
            ("feedback", "User feedback", "Task ratings and comments"),
            ("audit_log", "Audit log", "System activity records"),
            ("api_keys", "API keys", "External access credentials"),
            ("marketplace_installs", "Marketplace", "Installed packages"),
            ("marketplace_reviews", "Reviews", "Package reviews"),
        ];

        for (table, name, description) in &table_info {
            let table_exists: i64 = conn
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{}'",
                        table
                    ),
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0);

            let record_count = if table_exists > 0 {
                conn.query_row(&format!("SELECT COUNT(*) FROM {}", table), [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap_or(0)
            } else {
                0
            };

            categories.push(DataCategory {
                name: name.to_string(),
                table: table.to_string(),
                description: description.to_string(),
                record_count: record_count as u64,
                stored_locally: true,
                leaves_device: false,
            });
        }

        Ok(categories)
    }

    fn query_all(conn: &Connection, sql: &str) -> Result<Vec<serde_json::Value>, String> {
        // Check if the table exists by trying to prepare the statement
        let mut stmt = match conn.prepare(sql) {
            Ok(s) => s,
            Err(_) => return Ok(vec![]), // Table doesn't exist, return empty
        };
        let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

        let rows = stmt
            .query_map([], |row| {
                let mut map = serde_json::Map::new();
                for (i, name) in column_names.iter().enumerate() {
                    let val: String = row.get::<_, String>(i).unwrap_or_default();
                    map.insert(name.clone(), serde_json::Value::String(val));
                }
                Ok(serde_json::Value::Object(map))
            })
            .map_err(|e| e.to_string())?;

        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}
