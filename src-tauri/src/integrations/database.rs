use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;
use uuid::Uuid;

// ── Data types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub id: String,
    pub name: String,
    pub db_type: String, // "postgresql", "mysql", "sqlite"
    pub connection_string: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    pub name: String,
    pub columns: Vec<ColumnInfo>,
    pub row_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub row_count: u64,
    pub duration_ms: u64,
}

// ── Safety: validate read-only queries ──────────────────────────────────

fn validate_query(sql: &str, read_only: bool) -> Result<(), String> {
    if !read_only {
        return Ok(());
    }
    let upper = sql.trim().to_uppercase();
    let blocked = [
        "INSERT", "UPDATE", "DELETE", "DROP", "ALTER", "CREATE", "TRUNCATE", "REPLACE",
    ];
    for keyword in &blocked {
        if upper.starts_with(keyword) {
            return Err(format!(
                "Query blocked: {} statements are not allowed on read-only connections",
                keyword
            ));
        }
    }
    Ok(())
}

// ── DatabaseManager ─────────────────────────────────────────────────────

pub struct DatabaseManager {
    configs: HashMap<String, DatabaseConfig>,
}

impl DatabaseManager {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
        }
    }

    pub fn add_connection(&mut self, mut config: DatabaseConfig) -> DatabaseConfig {
        if config.id.is_empty() {
            config.id = Uuid::new_v4().to_string();
        }
        let out = config.clone();
        self.configs.insert(config.id.clone(), config);
        out
    }

    pub fn remove_connection(&mut self, id: &str) -> Result<bool, String> {
        Ok(self.configs.remove(id).is_some())
    }

    pub fn list_connections(&self) -> Vec<DatabaseConfig> {
        self.configs.values().cloned().collect()
    }

    pub fn test_connection(&self, id: &str) -> Result<bool, String> {
        let config = self.configs.get(id).ok_or("Connection not found")?;
        match config.db_type.as_str() {
            "sqlite" => {
                let conn = Connection::open(&config.connection_string)
                    .map_err(|e| format!("SQLite connection failed: {}", e))?;
                conn.execute_batch("PRAGMA journal_mode=WAL;")
                    .map_err(|e| format!("SQLite WAL setup failed: {}", e))?;
                conn.execute_batch("SELECT 1")
                    .map_err(|e| format!("SQLite test query failed: {}", e))?;
                // Integrity check
                let integrity: String = conn
                    .query_row("PRAGMA integrity_check", [], |row| row.get(0))
                    .unwrap_or_else(|_| "error".to_string());
                if integrity != "ok" {
                    return Err(format!("SQLite integrity check failed: {}", integrity));
                }
                Ok(true)
            }
            other => Err(format!("Unsupported database type: '{}'. Only SQLite is supported.", other)),
        }
    }

    pub fn list_tables(&self, id: &str) -> Result<Vec<TableInfo>, String> {
        let config = self.configs.get(id).ok_or("Connection not found")?;
        match config.db_type.as_str() {
            "sqlite" => {
                let conn = Connection::open(&config.connection_string)
                    .map_err(|e| format!("SQLite open failed: {}", e))?;
                conn.execute_batch("PRAGMA journal_mode=WAL;").ok();

                let mut stmt = conn
                    .prepare(
                        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
                    )
                    .map_err(|e| e.to_string())?;

                let table_names: Vec<String> = stmt
                    .query_map([], |row| row.get(0))
                    .map_err(|e| e.to_string())?
                    .filter_map(|r| r.ok())
                    .collect();

                let mut tables = Vec::new();
                for tname in table_names {
                    // Get column info via PRAGMA
                    let pragma_sql = format!("PRAGMA table_info(\"{}\")", tname);
                    let mut col_stmt = conn.prepare(&pragma_sql).map_err(|e| e.to_string())?;
                    let columns: Vec<ColumnInfo> = col_stmt
                        .query_map([], |row| {
                            Ok(ColumnInfo {
                                name: row.get(1)?,
                                data_type: row.get(2)?,
                                nullable: {
                                    let notnull: i32 = row.get(3)?;
                                    notnull == 0
                                },
                            })
                        })
                        .map_err(|e| e.to_string())?
                        .filter_map(|r| r.ok())
                        .collect();

                    // Get row count
                    let count_sql = format!("SELECT COUNT(*) FROM \"{}\"", tname);
                    let row_count: u64 = conn
                        .query_row(&count_sql, [], |row| row.get(0))
                        .unwrap_or(0);

                    tables.push(TableInfo {
                        name: tname,
                        columns,
                        row_count,
                    });
                }

                Ok(tables)
            }
            other => Err(format!("Unsupported database type: '{}'. Only SQLite is supported.", other)),
        }
    }

    pub fn execute_query(&self, id: &str, sql: &str) -> Result<QueryResult, String> {
        let config = self.configs.get(id).ok_or("Connection not found")?;

        // Safety check for read-only connections
        validate_query(sql, config.read_only)?;

        match config.db_type.as_str() {
            "sqlite" => {
                let conn = Connection::open(&config.connection_string)
                    .map_err(|e| format!("SQLite open failed: {}", e))?;
                conn.execute_batch("PRAGMA journal_mode=WAL;").ok();

                let start = Instant::now();
                let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;

                let col_count = stmt.column_count();
                let columns: Vec<String> = (0..col_count)
                    .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
                    .collect();

                let rows_result: Vec<Vec<String>> = stmt
                    .query_map([], |row| {
                        let mut vals = Vec::new();
                        for i in 0..col_count {
                            let val: String = row
                                .get::<_, rusqlite::types::Value>(i)
                                .map(|v| match v {
                                    rusqlite::types::Value::Null => "NULL".to_string(),
                                    rusqlite::types::Value::Integer(n) => n.to_string(),
                                    rusqlite::types::Value::Real(f) => f.to_string(),
                                    rusqlite::types::Value::Text(s) => s,
                                    rusqlite::types::Value::Blob(b) => {
                                        format!("[blob {} bytes]", b.len())
                                    }
                                })
                                .unwrap_or_else(|_| "ERROR".to_string());
                            vals.push(val);
                        }
                        Ok(vals)
                    })
                    .map_err(|e| e.to_string())?
                    .filter_map(|r| r.ok())
                    .collect();

                let duration_ms = start.elapsed().as_millis() as u64;
                let row_count = rows_result.len() as u64;

                Ok(QueryResult {
                    columns,
                    rows: rows_result,
                    row_count,
                    duration_ms,
                })
            }
            other => Err(format!("Unsupported database type: '{}'. Only SQLite is supported.", other)),
        }
    }
}
