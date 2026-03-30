use serde::{Deserialize, Serialize};
use rusqlite::Connection;
use chrono::Utc;

// ── Data structures ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingPair {
    pub instruction: String,
    pub input: String,
    pub output: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FineTuneConfig {
    pub base_model: String,
    pub epochs: u32,
    pub learning_rate: f64,
    pub method: String,       // "lora" or "full"
    pub dataset_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FineTuneJob {
    pub id: String,
    pub config: FineTuneConfig,
    pub status: String, // "preparing", "training", "completed", "failed"
    pub progress: f64,
    pub created_at: String,
}

// ── Manager ──────────────────────────────────────────────────────────

pub struct FineTuneManager;

impl FineTuneManager {
    pub fn ensure_tables(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS finetune_jobs (
                id TEXT PRIMARY KEY,
                base_model TEXT NOT NULL,
                epochs INTEGER NOT NULL DEFAULT 3,
                learning_rate REAL NOT NULL DEFAULT 0.0002,
                method TEXT NOT NULL DEFAULT 'lora',
                dataset_path TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'preparing',
                progress REAL NOT NULL DEFAULT 0.0,
                created_at TEXT NOT NULL
            )"
        ).map_err(|e| e.to_string())
    }

    /// Export training data from task history into instruction/input/output pairs.
    /// Uses anonymized metadata only (no raw prompts for privacy).
    pub fn export_training_data(conn: &Connection) -> Result<Vec<TrainingPair>, String> {
        // Pull from training_records + tasks table to build synthetic pairs
        let mut stmt = conn
            .prepare(
                "SELECT task_type, complexity, model_used, success, duration_ms
                 FROM training_records ORDER BY created_at DESC LIMIT 500"
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt.query_map([], |row| {
            let task_type: String = row.get(0)?;
            let complexity: String = row.get(1)?;
            let model_used: String = row.get(2)?;
            let success: bool = row.get(3)?;
            let duration_ms: i64 = row.get(4)?;

            Ok(TrainingPair {
                instruction: format!("Execute a {} task with {} complexity", task_type, complexity),
                input: format!("model={}, timeout={}ms", model_used, duration_ms),
                output: if success {
                    "Task completed successfully".to_string()
                } else {
                    "Task failed — retry with adjusted parameters".to_string()
                },
                category: task_type,
            })
        }).map_err(|e| e.to_string())?;

        let mut pairs = Vec::new();
        for row in rows {
            if let Ok(p) = row {
                pairs.push(p);
            }
        }
        Ok(pairs)
    }

    /// Preview a limited number of training pairs.
    pub fn preview_data(conn: &Connection, limit: usize) -> Result<Vec<TrainingPair>, String> {
        let all = Self::export_training_data(conn)?;
        Ok(all.into_iter().take(limit).collect())
    }

    /// Start a fine-tuning job (stub — logs intent and stores job record).
    pub fn start_job(conn: &Connection, config: FineTuneConfig) -> Result<FineTuneJob, String> {
        Self::ensure_tables(conn)?;

        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        // Validate method
        if config.method != "lora" && config.method != "full" {
            return Err(format!("Invalid method '{}': must be 'lora' or 'full'", config.method));
        }

        let job = FineTuneJob {
            id: id.clone(),
            config: config.clone(),
            status: "preparing".into(),
            progress: 0.0,
            created_at: now.clone(),
        };

        conn.execute(
            "INSERT INTO finetune_jobs (id, base_model, epochs, learning_rate, method, dataset_path, status, progress, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                id, config.base_model, config.epochs, config.learning_rate,
                config.method, config.dataset_path, "preparing", 0.0, now
            ],
        ).map_err(|e| e.to_string())?;

        tracing::info!(
            "Fine-tune job created: id={}, model={}, method={}, epochs={}",
            id, config.base_model, config.method, config.epochs
        );

        Ok(job)
    }

    pub fn get_job_status(conn: &Connection, id: &str) -> Result<FineTuneJob, String> {
        Self::ensure_tables(conn)?;

        let mut stmt = conn
            .prepare("SELECT id, base_model, epochs, learning_rate, method, dataset_path, status, progress, created_at FROM finetune_jobs WHERE id = ?1")
            .map_err(|e| e.to_string())?;

        stmt.query_row(rusqlite::params![id], |row| {
            Ok(FineTuneJob {
                id: row.get(0)?,
                config: FineTuneConfig {
                    base_model: row.get(1)?,
                    epochs: row.get::<_, u32>(2)?,
                    learning_rate: row.get(3)?,
                    method: row.get(4)?,
                    dataset_path: row.get(5)?,
                },
                status: row.get(6)?,
                progress: row.get(7)?,
                created_at: row.get(8)?,
            })
        }).map_err(|e| format!("Fine-tune job not found: {}", e))
    }

    pub fn list_jobs(conn: &Connection) -> Result<Vec<FineTuneJob>, String> {
        Self::ensure_tables(conn)?;

        let mut stmt = conn
            .prepare("SELECT id, base_model, epochs, learning_rate, method, dataset_path, status, progress, created_at FROM finetune_jobs ORDER BY created_at DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt.query_map([], |row| {
            Ok(FineTuneJob {
                id: row.get(0)?,
                config: FineTuneConfig {
                    base_model: row.get(1)?,
                    epochs: row.get::<_, u32>(2)?,
                    learning_rate: row.get(3)?,
                    method: row.get(4)?,
                    dataset_path: row.get(5)?,
                },
                status: row.get(6)?,
                progress: row.get(7)?,
                created_at: row.get(8)?,
            })
        }).map_err(|e| e.to_string())?;

        let mut jobs = Vec::new();
        for row in rows {
            if let Ok(j) = row {
                jobs.push(j);
            }
        }
        Ok(jobs)
    }
}
