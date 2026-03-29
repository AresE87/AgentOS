use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackRecord {
    pub id: String,
    pub task_id: String,
    pub task_text: String,
    pub response_text: String,
    pub rating: i8, // 1 = thumbs up, -1 = thumbs down
    pub comment: Option<String>,
    pub model_used: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FeedbackStats {
    pub total: u32,
    pub positive: u32,
    pub negative: u32,
    pub positive_rate: f32,
}

pub struct FeedbackCollector;

impl FeedbackCollector {
    /// Ensure feedback table exists
    pub fn ensure_table(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS feedback (
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL,
                task_text TEXT NOT NULL,
                response_text TEXT NOT NULL,
                rating INTEGER NOT NULL,
                comment TEXT,
                model_used TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL
            )
        ",
        )
        .map_err(|e| e.to_string())
    }

    pub fn record(
        conn: &Connection,
        task_id: &str,
        task_text: &str,
        response_text: &str,
        rating: i8,
        comment: Option<&str>,
        model_used: &str,
    ) -> Result<FeedbackRecord, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO feedback (id, task_id, task_text, response_text, rating, comment, model_used, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                id,
                task_id,
                task_text,
                response_text,
                rating as i32,
                comment,
                model_used,
                now,
            ],
        )
        .map_err(|e| e.to_string())?;

        Ok(FeedbackRecord {
            id,
            task_id: task_id.to_string(),
            task_text: task_text.to_string(),
            response_text: response_text.to_string(),
            rating,
            comment: comment.map(|s| s.to_string()),
            model_used: model_used.to_string(),
            created_at: now,
        })
    }

    pub fn get_recent(conn: &Connection, limit: usize) -> Result<Vec<FeedbackRecord>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT id, task_id, task_text, response_text, rating, comment, model_used, created_at
                 FROM feedback ORDER BY created_at DESC LIMIT ?1",
            )
            .map_err(|e| e.to_string())?;

        let records = stmt
            .query_map(rusqlite::params![limit as i64], |row| {
                let rating_i32: i32 = row.get(4)?;
                Ok(FeedbackRecord {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    task_text: row.get(2)?,
                    response_text: row.get(3)?,
                    rating: rating_i32 as i8,
                    comment: row.get(5)?,
                    model_used: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        Ok(records)
    }

    pub fn get_stats(conn: &Connection) -> Result<FeedbackStats, String> {
        let total: u32 = conn
            .query_row("SELECT COUNT(*) FROM feedback", [], |row| row.get(0))
            .map_err(|e| e.to_string())?;

        let positive: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM feedback WHERE rating > 0",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        let negative: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM feedback WHERE rating < 0",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        let positive_rate = if total > 0 {
            positive as f32 / total as f32
        } else {
            0.0
        };

        Ok(FeedbackStats {
            total,
            positive,
            negative,
            positive_rate,
        })
    }
}
