use serde::{Deserialize, Serialize};
use rusqlite::Connection;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub content: String,
    pub category: String,       // "conversation", "preference", "project", "correction", "person"
    pub importance: f64,        // 0.0-1.0
    pub access_count: u32,
    pub created_at: String,
    pub last_accessed: Option<String>,
}

pub struct MemoryStore;

impl MemoryStore {
    pub fn ensure_table(conn: &Connection) -> Result<(), String> {
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS agent_memories (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                category TEXT NOT NULL DEFAULT 'conversation',
                importance REAL NOT NULL DEFAULT 0.5,
                access_count INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                last_accessed TEXT
            )
        ").map_err(|e| e.to_string())
    }

    pub fn store(conn: &Connection, content: &str, category: &str, importance: f64) -> Result<Memory, String> {
        Self::ensure_table(conn)?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO agent_memories (id, content, category, importance, access_count, created_at) VALUES (?1, ?2, ?3, ?4, 0, ?5)",
            rusqlite::params![id, content, category, importance, now]
        ).map_err(|e| e.to_string())?;

        Ok(Memory { id, content: content.to_string(), category: category.to_string(), importance, access_count: 0, created_at: now, last_accessed: None })
    }

    /// Search memories by keyword (simple text matching)
    pub fn search(conn: &Connection, query: &str, limit: usize) -> Result<Vec<Memory>, String> {
        Self::ensure_table(conn)?;
        let pattern = format!("%{}%", query);
        let mut stmt = conn.prepare(
            "SELECT id, content, category, importance, access_count, created_at, last_accessed FROM agent_memories WHERE content LIKE ?1 ORDER BY importance DESC, access_count DESC LIMIT ?2"
        ).map_err(|e| e.to_string())?;

        let rows = stmt.query_map(rusqlite::params![pattern, limit as i64], |row| {
            Ok(Memory {
                id: row.get(0)?, content: row.get(1)?, category: row.get(2)?,
                importance: row.get(3)?, access_count: row.get::<_, i64>(4)? as u32,
                created_at: row.get(5)?, last_accessed: row.get(6)?,
            })
        }).map_err(|e| e.to_string())?;

        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Get all memories by category
    pub fn list_by_category(conn: &Connection, category: &str, limit: usize) -> Result<Vec<Memory>, String> {
        Self::ensure_table(conn)?;
        let mut stmt = conn.prepare(
            "SELECT id, content, category, importance, access_count, created_at, last_accessed FROM agent_memories WHERE category = ?1 ORDER BY created_at DESC LIMIT ?2"
        ).map_err(|e| e.to_string())?;

        let rows = stmt.query_map(rusqlite::params![category, limit as i64], |row| {
            Ok(Memory {
                id: row.get(0)?, content: row.get(1)?, category: row.get(2)?,
                importance: row.get(3)?, access_count: row.get::<_, i64>(4)? as u32,
                created_at: row.get(5)?, last_accessed: row.get(6)?,
            })
        }).map_err(|e| e.to_string())?;

        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// List all memories (no category filter)
    pub fn list_all(conn: &Connection, limit: usize) -> Result<Vec<Memory>, String> {
        Self::ensure_table(conn)?;
        let mut stmt = conn.prepare(
            "SELECT id, content, category, importance, access_count, created_at, last_accessed FROM agent_memories ORDER BY created_at DESC LIMIT ?1"
        ).map_err(|e| e.to_string())?;

        let rows = stmt.query_map(rusqlite::params![limit as i64], |row| {
            Ok(Memory {
                id: row.get(0)?, content: row.get(1)?, category: row.get(2)?,
                importance: row.get(3)?, access_count: row.get::<_, i64>(4)? as u32,
                created_at: row.get(5)?, last_accessed: row.get(6)?,
            })
        }).map_err(|e| e.to_string())?;

        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Get recent memories for context injection
    pub fn get_relevant_context(conn: &Connection, query: &str, max_memories: usize) -> Result<String, String> {
        let memories = Self::search(conn, query, max_memories)?;
        if memories.is_empty() {
            return Ok(String::new());
        }

        let context = memories.iter()
            .map(|m| format!("[{}] {}", m.category, m.content))
            .collect::<Vec<_>>()
            .join("\n");

        // Update access counts
        for m in &memories {
            conn.execute(
                "UPDATE agent_memories SET access_count = access_count + 1, last_accessed = ?1 WHERE id = ?2",
                rusqlite::params![chrono::Utc::now().to_rfc3339(), m.id]
            ).ok();
        }

        Ok(format!("## Relevant memories:\n{}", context))
    }

    pub fn delete(conn: &Connection, id: &str) -> Result<(), String> {
        conn.execute("DELETE FROM agent_memories WHERE id = ?1", rusqlite::params![id]).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn forget_all(conn: &Connection) -> Result<u64, String> {
        Self::ensure_table(conn)?;
        let count = conn.execute("DELETE FROM agent_memories", []).map_err(|e| e.to_string())? as u64;
        Ok(count)
    }

    pub fn stats(conn: &Connection) -> Result<serde_json::Value, String> {
        Self::ensure_table(conn)?;
        let total: i64 = conn.query_row("SELECT COUNT(*) FROM agent_memories", [], |r| r.get(0)).unwrap_or(0);
        let categories: Vec<(String, i64)> = {
            let mut stmt = conn.prepare("SELECT category, COUNT(*) FROM agent_memories GROUP BY category").map_err(|e| e.to_string())?;
            let rows: Vec<(String, i64)> = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            rows
        };

        Ok(serde_json::json!({
            "total_memories": total,
            "categories": categories.into_iter().map(|(k, v)| serde_json::json!({"name": k, "count": v})).collect::<Vec<_>>()
        }))
    }
}
