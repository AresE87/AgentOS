use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingRecord {
    pub id: String,
    pub task_type: String,  // "shell", "vision", "browse", "chain"
    pub complexity: String, // "simple", "moderate", "complex"
    pub model_used: String,
    pub success: bool,
    pub feedback_rating: Option<i8>, // 1 or -1
    pub duration_ms: u64,
    pub token_count: u32,
    pub created_at: String,
    // NO task content — never store actual prompts/responses
}

pub struct TrainingCollector;

impl TrainingCollector {
    pub fn ensure_table(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS training_records (
                id TEXT PRIMARY KEY,
                task_type TEXT NOT NULL,
                complexity TEXT NOT NULL,
                model_used TEXT NOT NULL,
                success INTEGER NOT NULL,
                feedback_rating INTEGER,
                duration_ms INTEGER NOT NULL,
                token_count INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            )
        ",
        )
        .map_err(|e| e.to_string())
    }

    pub fn record(conn: &Connection, record: &TrainingRecord) -> Result<(), String> {
        conn.execute(
            "INSERT INTO training_records (id, task_type, complexity, model_used, success, feedback_rating, duration_ms, token_count, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                record.id, record.task_type, record.complexity, record.model_used,
                record.success as i32, record.feedback_rating, record.duration_ms as i64,
                record.token_count as i64, record.created_at
            ]
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_records(conn: &Connection, limit: usize) -> Result<Vec<TrainingRecord>, String> {
        let mut stmt = conn.prepare(
            &format!("SELECT id, task_type, complexity, model_used, success, feedback_rating, duration_ms, token_count, created_at FROM training_records ORDER BY created_at DESC LIMIT {}", limit)
        ).map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(TrainingRecord {
                    id: row.get(0)?,
                    task_type: row.get(1)?,
                    complexity: row.get(2)?,
                    model_used: row.get(3)?,
                    success: row.get::<_, i32>(4)? != 0,
                    feedback_rating: row.get(5)?,
                    duration_ms: row.get::<_, i64>(6)? as u64,
                    token_count: row.get::<_, i64>(7)? as u32,
                    created_at: row.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;

        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn get_summary(conn: &Connection) -> Result<TrainingSummary, String> {
        Self::ensure_table(conn)?;
        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM training_records", [], |r| r.get(0))
            .unwrap_or(0);
        let successful: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM training_records WHERE success = 1",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let with_feedback: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM training_records WHERE feedback_rating IS NOT NULL",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let positive: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM training_records WHERE feedback_rating > 0",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        Ok(TrainingSummary {
            total_records: total as u64,
            successful: successful as u64,
            success_rate: if total > 0 {
                successful as f64 / total as f64
            } else {
                0.0
            },
            with_feedback: with_feedback as u64,
            positive_feedback: positive as u64,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrainingSummary {
    pub total_records: u64,
    pub successful: u64,
    pub success_rate: f64,
    pub with_feedback: u64,
    pub positive_feedback: u64,
}
