use crate::brain::LLMResponse;
use rusqlite::{params, Connection};
use serde_json::{json, Value};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(path: &std::path::Path) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                source TEXT NOT NULL DEFAULT 'dashboard',
                input_text TEXT NOT NULL,
                output_text TEXT,
                status TEXT NOT NULL DEFAULT 'completed',
                task_type TEXT,
                tier TEXT,
                complexity INTEGER,
                model_used TEXT,
                provider TEXT,
                tokens_in INTEGER DEFAULT 0,
                tokens_out INTEGER DEFAULT 0,
                cost REAL DEFAULT 0,
                duration_ms INTEGER DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                completed_at TEXT
            );

            CREATE TABLE IF NOT EXISTS task_steps (
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL REFERENCES tasks(id),
                step_number INTEGER NOT NULL,
                action_type TEXT NOT NULL,
                description TEXT,
                screenshot_path TEXT,
                execution_method TEXT,
                success INTEGER NOT NULL DEFAULT 1,
                duration_ms INTEGER DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS llm_calls (
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL REFERENCES tasks(id),
                provider TEXT NOT NULL,
                model TEXT NOT NULL,
                tokens_in INTEGER DEFAULT 0,
                tokens_out INTEGER DEFAULT 0,
                cost REAL DEFAULT 0,
                latency_ms INTEGER DEFAULT 0,
                success INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
            CREATE INDEX IF NOT EXISTS idx_tasks_created ON tasks(created_at);
            CREATE INDEX IF NOT EXISTS idx_steps_task ON task_steps(task_id);
            CREATE INDEX IF NOT EXISTS idx_llm_task ON llm_calls(task_id);",
        )
    }

    pub fn insert_task(&self, input: &str, response: &LLMResponse) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO tasks (id, input_text, output_text, status, model_used, provider, tokens_in, tokens_out, cost, duration_ms, completed_at)
             VALUES (?1, ?2, ?3, 'completed', ?4, ?5, ?6, ?7, ?8, ?9, datetime('now'))",
            params![
                response.task_id,
                input,
                response.content,
                response.model,
                response.provider,
                response.tokens_in,
                response.tokens_out,
                response.cost,
                response.duration_ms,
            ],
        )?;

        let call_id = uuid::Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO llm_calls (id, task_id, provider, model, tokens_in, tokens_out, cost, latency_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                call_id,
                response.task_id,
                response.provider,
                response.model,
                response.tokens_in,
                response.tokens_out,
                response.cost,
                response.duration_ms,
            ],
        )?;

        Ok(())
    }

    pub fn get_tasks(&self, limit: u32) -> Result<Value, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, input_text, output_text, status, model_used, provider, cost, duration_ms, created_at
             FROM tasks ORDER BY created_at DESC LIMIT ?1",
        )?;

        let tasks: Vec<Value> = stmt
            .query_map(params![limit], |row| {
                Ok(json!({
                    "task_id": row.get::<_, String>(0)?,
                    "input": row.get::<_, String>(1)?,
                    "output": row.get::<_, Option<String>>(2)?,
                    "status": row.get::<_, String>(3)?,
                    "model": row.get::<_, Option<String>>(4)?,
                    "provider": row.get::<_, Option<String>>(5)?,
                    "cost": row.get::<_, f64>(6)?,
                    "duration_ms": row.get::<_, i64>(7)?,
                    "created_at": row.get::<_, String>(8)?,
                }))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(json!(tasks))
    }

    pub fn get_analytics(&self) -> Result<Value, rusqlite::Error> {
        let total_tasks: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))?;
        let completed: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE status='completed'",
            [],
            |r| r.get(0),
        )?;
        let total_cost: f64 = self.conn.query_row(
            "SELECT COALESCE(SUM(cost), 0) FROM tasks",
            [],
            |r| r.get(0),
        )?;
        let total_tokens: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(tokens_in + tokens_out), 0) FROM tasks",
            [],
            |r| r.get(0),
        )?;

        let success_rate = if total_tasks > 0 {
            (completed as f64 / total_tasks as f64) * 100.0
        } else {
            0.0
        };

        Ok(json!({
            "total_tasks": total_tasks,
            "success_rate": success_rate,
            "total_cost": total_cost,
            "total_tokens": total_tokens,
        }))
    }

    // ── Phase 2: PC Control methods ──────────────────────────────

    pub fn create_task_pending(&self, task_id: &str, input: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO tasks (id, input_text, status) VALUES (?1, ?2, 'running')",
            params![task_id, input],
        )?;
        Ok(())
    }

    pub fn update_task_status(&self, task_id: &str, status: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE tasks SET status = ?2, completed_at = datetime('now') WHERE id = ?1",
            params![task_id, status],
        )?;
        Ok(())
    }

    pub fn update_task_output(&self, task_id: &str, output: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE tasks SET output_text = ?2 WHERE id = ?1",
            params![task_id, output],
        )?;
        Ok(())
    }

    pub fn insert_task_step(
        &self,
        task_id: &str,
        step_number: u32,
        action_type: &str,
        description: &str,
        screenshot_path: &str,
        execution_method: &str,
        success: bool,
        duration_ms: u64,
    ) -> Result<(), rusqlite::Error> {
        let step_id = uuid::Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO task_steps (id, task_id, step_number, action_type, description, screenshot_path, execution_method, success, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                step_id,
                task_id,
                step_number,
                action_type,
                description,
                screenshot_path,
                execution_method,
                success as i32,
                duration_ms as i64,
            ],
        )?;
        Ok(())
    }

    pub fn get_task_steps(&self, task_id: &str) -> Result<Value, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT step_number, action_type, description, screenshot_path, execution_method, success, duration_ms, created_at
             FROM task_steps WHERE task_id = ?1 ORDER BY step_number",
        )?;

        let steps: Vec<Value> = stmt
            .query_map(params![task_id], |row| {
                Ok(json!({
                    "step_number": row.get::<_, i32>(0)?,
                    "action_type": row.get::<_, String>(1)?,
                    "description": row.get::<_, Option<String>>(2)?,
                    "screenshot_path": row.get::<_, Option<String>>(3)?,
                    "execution_method": row.get::<_, Option<String>>(4)?,
                    "success": row.get::<_, i32>(5)? == 1,
                    "duration_ms": row.get::<_, i64>(6)?,
                    "created_at": row.get::<_, String>(7)?,
                }))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(json!(steps))
    }
}
