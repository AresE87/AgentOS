use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub content: String,
    pub category: String, // "conversation", "preference", "project", "correction", "person"
    pub importance: f64,  // 0.0-1.0
    pub access_count: u32,
    pub created_at: String,
    pub last_accessed: Option<String>,
}

pub struct MemoryStore;

/// Generate embedding via OpenAI API (text-embedding-3-small, 1536 dims).
/// This is a standalone async function that does NOT touch the database.
pub async fn get_embedding(text: &str, api_key: &str) -> Result<Vec<f32>, String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": "text-embedding-3-small",
        "input": text
    });

    let response = client
        .post("https://api.openai.com/v1/embeddings")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Embedding request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();
        return Err(format!("OpenAI API error {}: {}", status, body_text));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse embedding response: {}", e))?;

    let embedding = json["data"][0]["embedding"]
        .as_array()
        .ok_or_else(|| "No embedding in response".to_string())?
        .iter()
        .filter_map(|v| v.as_f64().map(|f| f as f32))
        .collect();

    Ok(embedding)
}

/// Cosine similarity between two vectors
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

/// Serialize f32 vec to bytes for BLOB storage (little-endian)
pub fn embedding_to_bytes(embedding: &[f32]) -> Vec<u8> {
    embedding.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Deserialize bytes from BLOB back to f32 vec
pub fn bytes_to_embedding(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

impl MemoryStore {
    pub fn ensure_table(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS agent_memories (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                category TEXT NOT NULL DEFAULT 'conversation',
                importance REAL NOT NULL DEFAULT 0.5,
                access_count INTEGER NOT NULL DEFAULT 0,
                embedding BLOB,
                created_at TEXT NOT NULL,
                last_accessed TEXT
            )
        ",
        )
        .map_err(|e| e.to_string())?;

        // Migration: add embedding column if missing (existing DBs)
        let has_embedding: bool = conn
            .prepare("SELECT embedding FROM agent_memories LIMIT 0")
            .is_ok();
        if !has_embedding {
            conn.execute_batch("ALTER TABLE agent_memories ADD COLUMN embedding BLOB")
                .ok(); // ignore if already exists
        }

        Ok(())
    }

    /// Store a memory (sync, no embedding)
    pub fn store(
        conn: &Connection,
        content: &str,
        category: &str,
        importance: f64,
    ) -> Result<Memory, String> {
        Self::ensure_table(conn)?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO agent_memories (id, content, category, importance, access_count, created_at) VALUES (?1, ?2, ?3, ?4, 0, ?5)",
            rusqlite::params![id, content, category, importance, now],
        )
        .map_err(|e| e.to_string())?;

        Ok(Memory {
            id,
            content: content.to_string(),
            category: category.to_string(),
            importance,
            access_count: 0,
            created_at: now,
            last_accessed: None,
        })
    }

    /// Store a memory with a pre-computed embedding blob
    pub fn store_with_embedding(
        conn: &Connection,
        content: &str,
        category: &str,
        importance: f64,
        embedding_blob: &[u8],
    ) -> Result<Memory, String> {
        Self::ensure_table(conn)?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO agent_memories (id, content, category, importance, access_count, embedding, created_at) VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6)",
            rusqlite::params![id, content, category, importance, embedding_blob, now],
        )
        .map_err(|e| e.to_string())?;

        Ok(Memory {
            id,
            content: content.to_string(),
            category: category.to_string(),
            importance,
            access_count: 0,
            created_at: now,
            last_accessed: None,
        })
    }

    /// Search memories by keyword (simple text matching) -- LIKE fallback
    pub fn search(conn: &Connection, query: &str, limit: usize) -> Result<Vec<Memory>, String> {
        Self::ensure_table(conn)?;
        let pattern = format!("%{}%", query);
        let mut stmt = conn.prepare(
            "SELECT id, content, category, importance, access_count, created_at, last_accessed FROM agent_memories WHERE content LIKE ?1 ORDER BY importance DESC, access_count DESC LIMIT ?2"
        ).map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(rusqlite::params![pattern, limit as i64], |row| {
                Ok(Memory {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    category: row.get(2)?,
                    importance: row.get(3)?,
                    access_count: row.get::<_, i64>(4)? as u32,
                    created_at: row.get(5)?,
                    last_accessed: row.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?;

        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Load all memories that have embeddings (sync DB read).
    /// Returns (Memory, embedding_bytes) pairs for caller to rank.
    pub fn load_embedded_memories(conn: &Connection) -> Result<Vec<(Memory, Vec<u8>)>, String> {
        Self::ensure_table(conn)?;
        let mut stmt = conn.prepare(
            "SELECT id, content, category, importance, access_count, created_at, last_accessed, embedding FROM agent_memories WHERE embedding IS NOT NULL"
        ).map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                let embedding_bytes: Vec<u8> = row.get(7)?;
                Ok((
                    Memory {
                        id: row.get(0)?,
                        content: row.get(1)?,
                        category: row.get(2)?,
                        importance: row.get(3)?,
                        access_count: row.get::<_, i64>(4)? as u32,
                        created_at: row.get(5)?,
                        last_accessed: row.get(6)?,
                    },
                    embedding_bytes,
                ))
            })
            .map_err(|e| e.to_string())?;

        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Rank pre-loaded memories against a query embedding using cosine similarity.
    /// Returns top N with similarity > 0.3.
    pub fn rank_by_similarity(
        memories_with_blobs: Vec<(Memory, Vec<u8>)>,
        query_embedding: &[f32],
        limit: usize,
    ) -> Vec<(Memory, f32)> {
        let mut scored: Vec<(Memory, f32)> = memories_with_blobs
            .into_iter()
            .map(|(mem, emb_bytes)| {
                let emb = bytes_to_embedding(&emb_bytes);
                let sim = cosine_similarity(query_embedding, &emb);
                (mem, sim)
            })
            .filter(|(_, sim)| *sim > 0.3)
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        scored
    }

    /// Build a semantic context snippet from stored embeddings.
    /// Returns None when no semantic matches are available.
    pub fn semantic_context(
        conn: &Connection,
        query_embedding: &[f32],
        max_memories: usize,
    ) -> Result<Option<String>, String> {
        let scored = Self::rank_by_similarity(
            Self::load_embedded_memories(conn)?,
            query_embedding,
            max_memories,
        );

        if scored.is_empty() {
            return Ok(None);
        }

        let memories: Vec<Memory> = scored.into_iter().map(|(memory, _)| memory).collect();
        let ids: Vec<String> = memories.iter().map(|memory| memory.id.clone()).collect();
        Self::update_access_counts(conn, &ids);

        Ok(Some(format_context_block(&memories, "semantic")))
    }

    /// Load memories without embeddings (for reindexing). Returns (id, content) pairs.
    pub fn load_unembedded_memories(conn: &Connection) -> Result<Vec<(String, String)>, String> {
        Self::ensure_table(conn)?;
        let mut stmt = conn
            .prepare("SELECT id, content FROM agent_memories WHERE embedding IS NULL")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rows)
    }

    /// Update embedding for a single memory by ID (sync)
    pub fn update_embedding(
        conn: &Connection,
        id: &str,
        embedding_blob: &[u8],
    ) -> Result<(), String> {
        conn.execute(
            "UPDATE agent_memories SET embedding = ?1 WHERE id = ?2",
            rusqlite::params![embedding_blob, id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Get all memories by category
    pub fn list_by_category(
        conn: &Connection,
        category: &str,
        limit: usize,
    ) -> Result<Vec<Memory>, String> {
        Self::ensure_table(conn)?;
        let mut stmt = conn.prepare(
            "SELECT id, content, category, importance, access_count, created_at, last_accessed FROM agent_memories WHERE category = ?1 ORDER BY created_at DESC LIMIT ?2"
        ).map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(rusqlite::params![category, limit as i64], |row| {
                Ok(Memory {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    category: row.get(2)?,
                    importance: row.get(3)?,
                    access_count: row.get::<_, i64>(4)? as u32,
                    created_at: row.get(5)?,
                    last_accessed: row.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?;

        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// List all memories (no category filter)
    pub fn list_all(conn: &Connection, limit: usize) -> Result<Vec<Memory>, String> {
        Self::ensure_table(conn)?;
        let mut stmt = conn.prepare(
            "SELECT id, content, category, importance, access_count, created_at, last_accessed FROM agent_memories ORDER BY created_at DESC LIMIT ?1"
        ).map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(rusqlite::params![limit as i64], |row| {
                Ok(Memory {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    category: row.get(2)?,
                    importance: row.get(3)?,
                    access_count: row.get::<_, i64>(4)? as u32,
                    created_at: row.get(5)?,
                    last_accessed: row.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?;

        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Get recent memories for context injection (sync LIKE-based fallback)
    pub fn get_relevant_context(
        conn: &Connection,
        query: &str,
        max_memories: usize,
    ) -> Result<String, String> {
        let memories = Self::search(conn, query, max_memories)?;
        if memories.is_empty() {
            return Ok(String::new());
        }

        // Update access counts
        for m in &memories {
            conn.execute(
                "UPDATE agent_memories SET access_count = access_count + 1, last_accessed = ?1 WHERE id = ?2",
                rusqlite::params![chrono::Utc::now().to_rfc3339(), m.id],
            )
            .ok();
        }

        Ok(format_context_block(&memories, "keyword"))
    }

    /// Update access counts for a batch of memories
    pub fn update_access_counts(conn: &Connection, memory_ids: &[String]) {
        let now = chrono::Utc::now().to_rfc3339();
        for id in memory_ids {
            conn.execute(
                "UPDATE agent_memories SET access_count = access_count + 1, last_accessed = ?1 WHERE id = ?2",
                rusqlite::params![now, id],
            )
            .ok();
        }
    }

    pub fn delete(conn: &Connection, id: &str) -> Result<(), String> {
        conn.execute(
            "DELETE FROM agent_memories WHERE id = ?1",
            rusqlite::params![id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn forget_all(conn: &Connection) -> Result<u64, String> {
        Self::ensure_table(conn)?;
        let count = conn
            .execute("DELETE FROM agent_memories", [])
            .map_err(|e| e.to_string())? as u64;
        Ok(count)
    }

    pub fn stats(conn: &Connection) -> Result<serde_json::Value, String> {
        Self::ensure_table(conn)?;
        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM agent_memories", [], |r| r.get(0))
            .unwrap_or(0);
        let with_embeddings: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM agent_memories WHERE embedding IS NOT NULL",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let categories: Vec<(String, i64)> = {
            let mut stmt = conn
                .prepare("SELECT category, COUNT(*) FROM agent_memories GROUP BY category")
                .map_err(|e| e.to_string())?;
            let rows: Vec<(String, i64)> = stmt
                .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            rows
        };

        Ok(serde_json::json!({
            "total_memories": total,
            "with_embeddings": with_embeddings,
            "categories": categories.into_iter().map(|(k, v)| serde_json::json!({"name": k, "count": v})).collect::<Vec<_>>()
        }))
    }
}

fn format_context_block(memories: &[Memory], method: &str) -> String {
    let context = memories
        .iter()
        .map(|memory| format!("[{}] {}", memory.category, memory.content))
        .collect::<Vec<_>>()
        .join("\n");
    format!("## Relevant memories ({method}):\n{context}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_conn() -> Connection {
        Connection::open_in_memory().unwrap()
    }

    #[test]
    fn semantic_context_prefers_high_similarity_memories() {
        let conn = temp_conn();
        let strong = embedding_to_bytes(&[1.0, 0.0, 0.0]);
        let weak = embedding_to_bytes(&[0.0, 1.0, 0.0]);

        MemoryStore::store_with_embedding(
            &conn,
            "Project alpha deadline is Friday",
            "project",
            0.9,
            &strong,
        )
        .unwrap();
        MemoryStore::store_with_embedding(&conn, "Buy groceries later", "personal", 0.4, &weak)
            .unwrap();

        let context = MemoryStore::semantic_context(&conn, &[0.95, 0.05, 0.0], 5)
            .unwrap()
            .unwrap();

        assert!(context.contains("Project alpha deadline is Friday"));
        assert!(!context.contains("Buy groceries later"));
        assert!(context.contains("semantic"));
    }

    #[test]
    fn keyword_context_marks_fallback_method() {
        let conn = temp_conn();
        MemoryStore::store(&conn, "Juan is my manager", "person", 0.8).unwrap();

        let context = MemoryStore::get_relevant_context(&conn, "manager", 5).unwrap();

        assert!(context.contains("Juan is my manager"));
        assert!(context.contains("keyword"));
    }
}
