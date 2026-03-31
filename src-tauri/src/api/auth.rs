use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub key: String,
    pub created_at: String,
    pub last_used: Option<String>,
    pub enabled: bool,
}

pub fn ensure_table(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS api_keys (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            key TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL,
            last_used TEXT,
            enabled INTEGER NOT NULL DEFAULT 1
        )",
    )
    .map_err(|e| e.to_string())
}

pub fn generate_api_key() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let hex: String = (0..32).map(|_| format!("{:x}", rng.gen::<u8>())).collect();
    format!("aos_{}", hex)
}

pub fn create_api_key(conn: &Connection, name: &str) -> Result<ApiKey, String> {
    ensure_table(conn)?;
    let id = uuid::Uuid::new_v4().to_string();
    let key = generate_api_key();
    let created_at = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO api_keys (id, name, key, created_at, enabled) VALUES (?1, ?2, ?3, ?4, 1)",
        rusqlite::params![id, name, key, created_at],
    )
    .map_err(|e| e.to_string())?;

    Ok(ApiKey {
        id,
        name: name.to_string(),
        key,
        created_at,
        last_used: None,
        enabled: true,
    })
}

pub fn list_api_keys(conn: &Connection) -> Result<Vec<ApiKey>, String> {
    ensure_table(conn)?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, key, created_at, last_used, enabled FROM api_keys ORDER BY created_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let keys = stmt
        .query_map([], |row| {
            Ok(ApiKey {
                id: row.get(0)?,
                name: row.get(1)?,
                key: row.get(2)?,
                created_at: row.get(3)?,
                last_used: row.get(4)?,
                enabled: row.get::<_, i64>(5)? != 0,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(keys)
}

pub fn revoke_api_key(conn: &Connection, id: &str) -> Result<(), String> {
    ensure_table(conn)?;
    conn.execute(
        "UPDATE api_keys SET enabled = 0 WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn validate_api_key(conn: &Connection, key: &str) -> Result<bool, String> {
    ensure_table(conn)?;

    let result: rusqlite::Result<i64> = conn.query_row(
        "SELECT enabled FROM api_keys WHERE key = ?1",
        rusqlite::params![key],
        |row| row.get(0),
    );

    match result {
        Ok(enabled) => {
            if enabled != 0 {
                // Update last_used
                let now = chrono::Utc::now().to_rfc3339();
                let _ = conn.execute(
                    "UPDATE api_keys SET last_used = ?1 WHERE key = ?2",
                    rusqlite::params![now, key],
                );
                Ok(true)
            } else {
                Ok(false)
            }
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
        Err(e) => Err(e.to_string()),
    }
}
