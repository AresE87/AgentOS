use crate::automation::scheduler::Trigger;
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
            CREATE INDEX IF NOT EXISTS idx_llm_task ON llm_calls(task_id);

            CREATE TABLE IF NOT EXISTS execution_traces (
                task_id TEXT PRIMARY KEY REFERENCES tasks(id),
                source TEXT NOT NULL DEFAULT 'pipeline',
                input_text TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'running',
                total_duration_ms INTEGER NOT NULL DEFAULT 0,
                total_cost REAL NOT NULL DEFAULT 0,
                total_tokens INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                finished_at TEXT
            );

            CREATE TABLE IF NOT EXISTS execution_trace_steps (
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL REFERENCES execution_traces(task_id),
                seq INTEGER NOT NULL,
                phase TEXT NOT NULL,
                input TEXT NOT NULL DEFAULT '',
                output TEXT NOT NULL DEFAULT '',
                decision TEXT NOT NULL DEFAULT '',
                duration_ms INTEGER NOT NULL DEFAULT 0,
                cost REAL NOT NULL DEFAULT 0,
                tokens INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_execution_traces_created ON execution_traces(created_at);
            CREATE INDEX IF NOT EXISTS idx_execution_trace_steps_task ON execution_trace_steps(task_id, seq);

            CREATE TABLE IF NOT EXISTS hardware_partners (
                id TEXT PRIMARY KEY,
                company TEXT NOT NULL,
                device_type TEXT NOT NULL,
                integration_level TEXT NOT NULL,
                certified INTEGER NOT NULL DEFAULT 0,
                certification_note TEXT,
                certification_evidence TEXT,
                contact_email TEXT,
                units_shipped INTEGER,
                registered_at TEXT NOT NULL DEFAULT (datetime('now')),
                certified_at TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_hardware_partners_registered ON hardware_partners(registered_at);
            CREATE INDEX IF NOT EXISTS idx_hardware_partners_certified ON hardware_partners(certified);

            CREATE TABLE IF NOT EXISTS chain_log (
                id          TEXT PRIMARY KEY,
                chain_id    TEXT NOT NULL,
                timestamp   TEXT NOT NULL DEFAULT (datetime('now')),
                agent_name  TEXT NOT NULL,
                agent_level TEXT NOT NULL,
                event_type  TEXT NOT NULL,
                message     TEXT NOT NULL,
                metadata    TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_chain_log_chain ON chain_log(chain_id);

            CREATE TABLE IF NOT EXISTS chains (
                id TEXT PRIMARY KEY,
                original_task TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'running',
                total_cost REAL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                completed_at TEXT
            );

            CREATE TABLE IF NOT EXISTS chain_subtasks (
                id TEXT PRIMARY KEY,
                chain_id TEXT NOT NULL,
                seq INTEGER NOT NULL,
                description TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'queued',
                agent_name TEXT,
                model TEXT,
                progress REAL DEFAULT 0,
                message TEXT DEFAULT '',
                cost REAL DEFAULT 0,
                duration_ms INTEGER DEFAULT 0,
                output TEXT DEFAULT '',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_chains_status ON chains(status);
            CREATE INDEX IF NOT EXISTS idx_chain_subtasks_chain ON chain_subtasks(chain_id);

            CREATE TABLE IF NOT EXISTS triggers (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                trigger_type TEXT NOT NULL,
                config TEXT NOT NULL,
                task_text TEXT NOT NULL,
                enabled INTEGER DEFAULT 1,
                last_run TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_triggers_enabled ON triggers(enabled);

            -- C1: Daily usage tracking for billing enforcement
            CREATE TABLE IF NOT EXISTS daily_usage (
                date TEXT PRIMARY KEY,
                tasks_count INTEGER NOT NULL DEFAULT 0,
                tokens_used INTEGER NOT NULL DEFAULT 0,
                plan_type TEXT NOT NULL DEFAULT 'free'
            );",
        )
    }

    /// Increment the daily task and token counters. Called on each task execution.
    pub fn increment_daily_usage(&self, tokens: i64) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO daily_usage (date, tasks_count, tokens_used, plan_type)
             VALUES (date('now'), 1, ?1, 'free')
             ON CONFLICT(date) DO UPDATE SET
                tasks_count = tasks_count + 1,
                tokens_used = tokens_used + ?1",
            params![tokens],
        )?;
        Ok(())
    }

    /// Get today's usage from the daily_usage table.
    pub fn get_daily_usage(&self) -> Result<(i64, i64), rusqlite::Error> {
        let result = self.conn.query_row(
            "SELECT COALESCE(tasks_count, 0), COALESCE(tokens_used, 0) FROM daily_usage WHERE date = date('now')",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        );
        match result {
            Ok(r) => Ok(r),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok((0, 0)),
            Err(e) => Err(e),
        }
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

    pub fn update_task_metrics(
        &self,
        task_id: &str,
        model_used: Option<&str>,
        provider: Option<&str>,
        tokens_in: u32,
        tokens_out: u32,
        cost: f64,
        duration_ms: u64,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE tasks
             SET model_used = COALESCE(?2, model_used),
                 provider = COALESCE(?3, provider),
                 tokens_in = ?4,
                 tokens_out = ?5,
                 cost = ?6,
                 duration_ms = ?7
             WHERE id = ?1",
            params![
                task_id,
                model_used,
                provider,
                tokens_in as i64,
                tokens_out as i64,
                cost,
                duration_ms as i64,
            ],
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

    pub fn ensure_execution_trace(
        &self,
        task_id: &str,
        input_text: &str,
        source: &str,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO execution_traces (task_id, input_text, source, status)
             VALUES (?1, ?2, ?3, 'running')
             ON CONFLICT(task_id) DO UPDATE SET
                input_text = excluded.input_text,
                source = excluded.source",
            params![task_id, input_text, source],
        )?;
        Ok(())
    }

    pub fn append_execution_trace_step(
        &self,
        task_id: &str,
        phase: &str,
        input: &str,
        output: &str,
        decision: &str,
        duration_ms: u64,
        cost: f64,
        tokens: u32,
    ) -> Result<(), rusqlite::Error> {
        let id = uuid::Uuid::new_v4().to_string();
        let seq: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM execution_trace_steps WHERE task_id = ?1",
            params![task_id],
            |row| row.get(0),
        )?;

        self.conn.execute(
            "INSERT INTO execution_trace_steps (id, task_id, seq, phase, input, output, decision, duration_ms, cost, tokens)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id,
                task_id,
                seq,
                phase,
                input,
                output,
                decision,
                duration_ms as i64,
                cost,
                tokens as i64,
            ],
        )?;
        Ok(())
    }

    pub fn finish_execution_trace(
        &self,
        task_id: &str,
        status: &str,
        total_duration_ms: u64,
        total_cost: f64,
        total_tokens: u32,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE execution_traces
             SET status = ?2,
                 total_duration_ms = ?3,
                 total_cost = ?4,
                 total_tokens = ?5,
                 finished_at = datetime('now')
             WHERE task_id = ?1",
            params![
                task_id,
                status,
                total_duration_ms as i64,
                total_cost,
                total_tokens as i64,
            ],
        )?;
        Ok(())
    }

    pub fn list_execution_traces(&self, limit: usize) -> Result<Value, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT t.task_id, t.input_text, t.status, t.total_duration_ms, t.total_cost, t.created_at,
                    CASE WHEN t.finished_at IS NOT NULL THEN 1 ELSE 0 END AS finished
             FROM execution_traces t
             ORDER BY t.created_at DESC
             LIMIT ?1",
        )?;

        let traces: Vec<Value> = stmt
            .query_map(params![limit as i64], |row| {
                Ok(json!({
                    "id": row.get::<_, String>(0)?,
                    "task_id": row.get::<_, String>(0)?,
                    "input_text": row.get::<_, String>(1)?,
                    "status": row.get::<_, String>(2)?,
                    "total_duration_ms": row.get::<_, i64>(3)?,
                    "total_cost": row.get::<_, f64>(4)?,
                    "created_at": row.get::<_, String>(5)?,
                    "finished": row.get::<_, i32>(6)? == 1,
                }))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(json!(traces))
    }

    pub fn get_execution_trace(&self, task_id: &str) -> Result<Value, rusqlite::Error> {
        let trace = self.conn.query_row(
            "SELECT t.task_id, t.input_text, t.status, t.total_duration_ms, t.total_cost, t.total_tokens,
                    t.created_at, CASE WHEN t.finished_at IS NOT NULL THEN 1 ELSE 0 END AS finished,
                    tk.output_text
             FROM execution_traces t
             LEFT JOIN tasks tk ON tk.id = t.task_id
             WHERE t.task_id = ?1",
            params![task_id],
            |row| {
                Ok(json!({
                    "id": row.get::<_, String>(0)?,
                    "task_id": row.get::<_, String>(0)?,
                    "input_text": row.get::<_, String>(1)?,
                    "status": row.get::<_, String>(2)?,
                    "total_duration_ms": row.get::<_, i64>(3)?,
                    "total_cost": row.get::<_, f64>(4)?,
                    "total_tokens": row.get::<_, i64>(5)?,
                    "created_at": row.get::<_, String>(6)?,
                    "finished": row.get::<_, i32>(7)? == 1,
                    "output_text": row.get::<_, Option<String>>(8)?,
                }))
            },
        )?;

        let mut stmt = self.conn.prepare(
            "SELECT seq, phase, input, output, decision, duration_ms, cost, tokens, created_at
             FROM execution_trace_steps
             WHERE task_id = ?1
             ORDER BY seq ASC",
        )?;

        let steps: Vec<Value> = stmt
            .query_map(params![task_id], |row| {
                Ok(json!({
                    "seq": row.get::<_, i64>(0)?,
                    "phase": row.get::<_, String>(1)?,
                    "input": row.get::<_, String>(2)?,
                    "output": row.get::<_, String>(3)?,
                    "decision": row.get::<_, String>(4)?,
                    "duration_ms": row.get::<_, i64>(5)?,
                    "cost": row.get::<_, f64>(6)?,
                    "tokens": row.get::<_, i64>(7)?,
                    "created_at": row.get::<_, String>(8)?,
                }))
            })?
            .filter_map(|r| r.ok())
            .collect();

        let mut trace_obj = trace;
        trace_obj["steps"] = json!(steps);
        Ok(trace_obj)
    }

    pub fn list_hardware_partners(&self) -> Result<Value, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company, device_type, integration_level, certified,
                    certification_note, certification_evidence, contact_email, units_shipped,
                    registered_at, certified_at
             FROM hardware_partners
             ORDER BY registered_at DESC",
        )?;

        let partners: Vec<Value> = stmt
            .query_map([], |row| {
                Ok(json!({
                    "id": row.get::<_, String>(0)?,
                    "company": row.get::<_, String>(1)?,
                    "device_type": row.get::<_, String>(2)?,
                    "integration_level": row.get::<_, String>(3)?,
                    "certified": row.get::<_, i32>(4)? == 1,
                    "certification_note": row.get::<_, Option<String>>(5)?,
                    "certification_evidence": row.get::<_, Option<String>>(6)?,
                    "contact_email": row.get::<_, Option<String>>(7)?,
                    "units_shipped": row.get::<_, Option<i64>>(8)?,
                    "registered_at": row.get::<_, String>(9)?,
                    "certified_at": row.get::<_, Option<String>>(10)?,
                }))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(json!(partners))
    }

    pub fn get_hardware_partner(&self, id: &str) -> Result<Option<Value>, rusqlite::Error> {
        let result = self.conn.query_row(
            "SELECT id, company, device_type, integration_level, certified,
                    certification_note, certification_evidence, contact_email, units_shipped,
                    registered_at, certified_at
             FROM hardware_partners
             WHERE id = ?1",
            params![id],
            |row| {
                Ok(json!({
                    "id": row.get::<_, String>(0)?,
                    "company": row.get::<_, String>(1)?,
                    "device_type": row.get::<_, String>(2)?,
                    "integration_level": row.get::<_, String>(3)?,
                    "certified": row.get::<_, i32>(4)? == 1,
                    "certification_note": row.get::<_, Option<String>>(5)?,
                    "certification_evidence": row.get::<_, Option<String>>(6)?,
                    "contact_email": row.get::<_, Option<String>>(7)?,
                    "units_shipped": row.get::<_, Option<i64>>(8)?,
                    "registered_at": row.get::<_, String>(9)?,
                    "certified_at": row.get::<_, Option<String>>(10)?,
                }))
            },
        );

        match result {
            Ok(partner) => Ok(Some(partner)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn register_hardware_partner(
        &self,
        company: &str,
        device_type: &str,
        integration_level: &str,
    ) -> Result<Value, rusqlite::Error> {
        let id = uuid::Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO hardware_partners (id, company, device_type, integration_level, certified)
             VALUES (?1, ?2, ?3, ?4, 0)",
            params![id, company, device_type, integration_level],
        )?;

        Ok(json!({
            "id": id,
            "company": company,
            "device_type": device_type,
            "integration_level": integration_level,
            "certified": false,
            "registered_at": chrono::Utc::now().to_rfc3339(),
        }))
    }

    pub fn certify_hardware_partner(
        &self,
        id: &str,
        certification_note: &str,
        certification_evidence: &str,
    ) -> Result<Value, rusqlite::Error> {
        self.conn.execute(
            "UPDATE hardware_partners
             SET certified = 1,
                 certification_note = ?2,
                 certification_evidence = ?3,
                 certified_at = datetime('now')
             WHERE id = ?1",
            params![id, certification_note, certification_evidence],
        )?;

        Ok(self
            .get_hardware_partner(id)?
            .unwrap_or_else(|| json!({ "id": id, "certified": true })))
    }

    pub fn get_usage_summary(&self) -> Result<Value, rusqlite::Error> {
        // Read from daily_usage table (persisted counters) for billing enforcement
        let (tasks_today, tokens_today) = self.get_daily_usage()?;
        let cost_today: f64 = self.conn.query_row(
            "SELECT COALESCE(SUM(cost), 0) FROM tasks WHERE date(created_at) = date('now')",
            [],
            |r| r.get(0),
        )?;

        Ok(json!({
            "tasks_today": tasks_today,
            "tokens_today": tokens_today,
            "cost_today": cost_today,
        }))
    }

    // ── Enhanced analytics ────────────────────────────────────

    pub fn get_analytics_by_period(&self, period: &str) -> Result<Value, rusqlite::Error> {
        let date_filter = match period {
            "today" => "date(created_at) = date('now')",
            "this_week" => "date(created_at) >= date('now', '-7 days')",
            "this_month" => "date(created_at) >= date('now', '-30 days')",
            _ => "1=1", // all time
        };

        let total: i64 = self.conn.query_row(
            &format!("SELECT COUNT(*) FROM tasks WHERE {}", date_filter), [], |r| r.get(0))?;
        let completed: i64 = self.conn.query_row(
            &format!("SELECT COUNT(*) FROM tasks WHERE status='completed' AND {}", date_filter), [], |r| r.get(0))?;
        let failed: i64 = self.conn.query_row(
            &format!("SELECT COUNT(*) FROM tasks WHERE status='failed' AND {}", date_filter), [], |r| r.get(0))?;
        let total_cost: f64 = self.conn.query_row(
            &format!("SELECT COALESCE(SUM(cost), 0) FROM tasks WHERE {}", date_filter), [], |r| r.get(0))?;
        let total_tokens: i64 = self.conn.query_row(
            &format!("SELECT COALESCE(SUM(tokens_in + tokens_out), 0) FROM tasks WHERE {}", date_filter), [], |r| r.get(0))?;
        let avg_latency: f64 = self.conn.query_row(
            &format!("SELECT COALESCE(AVG(duration_ms), 0) FROM tasks WHERE {}", date_filter), [], |r| r.get(0))?;

        let success_rate = if total > 0 { (completed as f64 / total as f64) * 100.0 } else { 0.0 };

        // Cost by provider
        let mut stmt = self.conn.prepare(
            &format!("SELECT COALESCE(provider, 'unknown'), SUM(cost) FROM tasks WHERE {} GROUP BY provider", date_filter))?;
        let cost_by_provider: Vec<Value> = stmt.query_map([], |row| {
            Ok(json!({ "provider": row.get::<_, String>(0)?, "cost": row.get::<_, f64>(1)? }))
        })?.filter_map(|r| r.ok()).collect();

        // Tasks by day (last 7 days)
        let mut stmt = self.conn.prepare(
            "SELECT date(created_at) as day, COUNT(*) as count
             FROM tasks WHERE date(created_at) >= date('now', '-7 days')
             GROUP BY date(created_at) ORDER BY day")?;
        let daily_tasks: Vec<Value> = stmt.query_map([], |row| {
            Ok(json!({ "day": row.get::<_, String>(0)?, "tasks": row.get::<_, i32>(1)? }))
        })?.filter_map(|r| r.ok()).collect();

        // Tasks by type
        let mut stmt = self.conn.prepare(
            &format!("SELECT COALESCE(task_type, 'unknown'), COUNT(*) FROM tasks WHERE {} GROUP BY task_type", date_filter))?;
        let tasks_by_type: Vec<Value> = stmt.query_map([], |row| {
            Ok(json!({ "name": row.get::<_, String>(0)?, "value": row.get::<_, i32>(1)? }))
        })?.filter_map(|r| r.ok()).collect();

        Ok(json!({
            "total_tasks": total,
            "completed": completed,
            "failed": failed,
            "success_rate": success_rate,
            "total_cost": total_cost,
            "total_tokens": total_tokens,
            "avg_latency_ms": avg_latency,
            "cost_by_provider": cost_by_provider,
            "daily_tasks": daily_tasks,
            "tasks_by_type": tasks_by_type,
        }))
    }

    /// Find repeated tasks (same input seen >= threshold times in last N days)
    pub fn get_repeated_tasks(&self, days: i32, threshold: i32) -> Result<Value, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT input_text, COUNT(*) as cnt
             FROM tasks
             WHERE date(created_at) >= date('now', ?1)
             GROUP BY input_text
             HAVING cnt >= ?2
             ORDER BY cnt DESC
             LIMIT 5")?;

        let repeated: Vec<Value> = stmt.query_map(
            params![format!("-{} days", days), threshold],
            |row| {
                Ok(json!({
                    "input": row.get::<_, String>(0)?,
                    "count": row.get::<_, i32>(1)?,
                }))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(json!(repeated))
    }

    // ── Chain log methods ─────────────────────────────────────

    pub fn insert_chain_event(
        &self,
        chain_id: &str,
        agent_name: &str,
        agent_level: &str,
        event_type: &str,
        message: &str,
        metadata: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        let id = uuid::Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO chain_log (id, chain_id, agent_name, agent_level, event_type, message, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, chain_id, agent_name, agent_level, event_type, message, metadata],
        )?;
        Ok(())
    }

    pub fn get_chain_log(&self, chain_id: &str) -> Result<Value, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, agent_name, agent_level, event_type, message, metadata
             FROM chain_log WHERE chain_id = ?1 ORDER BY timestamp ASC",
        )?;

        let events: Vec<Value> = stmt
            .query_map(params![chain_id], |row| {
                Ok(json!({
                    "id": row.get::<_, String>(0)?,
                    "timestamp": row.get::<_, String>(1)?,
                    "agent_name": row.get::<_, String>(2)?,
                    "agent_level": row.get::<_, String>(3)?,
                    "event_type": row.get::<_, String>(4)?,
                    "message": row.get::<_, String>(5)?,
                    "metadata": row.get::<_, Option<String>>(6)?,
                }))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(json!(events))
    }

    pub fn get_recent_chains(&self, limit: u32) -> Result<Value, rusqlite::Error> {
        // Get unique chain_ids from chain_log, most recent first
        let mut stmt = self.conn.prepare(
            "SELECT chain_id, MIN(timestamp) as started, MAX(timestamp) as ended,
                    COUNT(*) as event_count,
                    GROUP_CONCAT(DISTINCT agent_name) as agents
             FROM chain_log
             GROUP BY chain_id
             ORDER BY started DESC
             LIMIT ?1",
        )?;

        let chains: Vec<Value> = stmt
            .query_map(params![limit], |row| {
                Ok(json!({
                    "chain_id": row.get::<_, String>(0)?,
                    "started_at": row.get::<_, String>(1)?,
                    "ended_at": row.get::<_, String>(2)?,
                    "event_count": row.get::<_, i32>(3)?,
                    "agents": row.get::<_, String>(4)?,
                }))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(json!(chains))
    }

    pub fn insert_llm_call(
        &self,
        task_id: &str,
        provider: &str,
        model: &str,
        tokens_in: u32,
        tokens_out: u32,
        cost: f64,
        latency_ms: u64,
    ) -> Result<(), rusqlite::Error> {
        let call_id = uuid::Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO llm_calls (id, task_id, provider, model, tokens_in, tokens_out, cost, latency_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![call_id, task_id, provider, model, tokens_in, tokens_out, cost, latency_ms as i64],
        )?;
        Ok(())
    }

    // ── Chain orchestrator methods ─────────────────────────────

    pub fn create_chain(&self, chain_id: &str, original_task: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO chains (id, original_task, status) VALUES (?1, ?2, 'running')",
            params![chain_id, original_task],
        )?;
        Ok(())
    }

    pub fn insert_chain_subtask(
        &self,
        id: &str,
        chain_id: &str,
        seq: i32,
        description: &str,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO chain_subtasks (id, chain_id, seq, description, status)
             VALUES (?1, ?2, ?3, ?4, 'queued')",
            params![id, chain_id, seq, description],
        )?;
        Ok(())
    }

    pub fn get_chain_subtasks(&self, chain_id: &str) -> Result<Value, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, chain_id, seq, description, status, agent_name, model, progress, message, cost, duration_ms, output, created_at
             FROM chain_subtasks WHERE chain_id = ?1 ORDER BY seq ASC",
        )?;

        let subtasks: Vec<Value> = stmt
            .query_map(params![chain_id], |row| {
                Ok(json!({
                    "id": row.get::<_, String>(0)?,
                    "chain_id": row.get::<_, String>(1)?,
                    "seq": row.get::<_, i32>(2)?,
                    "description": row.get::<_, String>(3)?,
                    "status": row.get::<_, String>(4)?,
                    "agent_name": row.get::<_, Option<String>>(5)?,
                    "model": row.get::<_, Option<String>>(6)?,
                    "progress": row.get::<_, f64>(7)?,
                    "message": row.get::<_, Option<String>>(8)?,
                    "cost": row.get::<_, f64>(9)?,
                    "duration_ms": row.get::<_, i64>(10)?,
                    "output": row.get::<_, Option<String>>(11)?,
                    "created_at": row.get::<_, String>(12)?,
                }))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(json!(subtasks))
    }

    pub fn update_subtask_status(
        &self,
        subtask_id: &str,
        status: &str,
        message: &str,
        output: &str,
        cost: f64,
        duration_ms: u64,
        agent_name: &str,
        model: &str,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE chain_subtasks SET status = ?2, message = ?3, output = ?4, cost = ?5,
             duration_ms = ?6, agent_name = ?7, model = ?8,
             progress = CASE WHEN ?2 = 'done' THEN 1.0 WHEN ?2 = 'running' THEN 0.5 ELSE 0.0 END
             WHERE id = ?1",
            params![subtask_id, status, message, output, cost, duration_ms as i64, agent_name, model],
        )?;
        Ok(())
    }

    pub fn complete_chain(&self, chain_id: &str, total_cost: f64) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE chains SET status = 'completed', total_cost = ?2, completed_at = datetime('now') WHERE id = ?1",
            params![chain_id, total_cost],
        )?;
        Ok(())
    }

    pub fn query_active_chain(&self) -> Result<Option<Value>, rusqlite::Error> {
        // First try running chains
        let result = self.conn.query_row(
            "SELECT id, original_task, status, total_cost, created_at FROM chains WHERE status = 'running' ORDER BY created_at DESC LIMIT 1",
            [],
            |row| {
                Ok(json!({
                    "id": row.get::<_, String>(0)?,
                    "original_task": row.get::<_, String>(1)?,
                    "status": row.get::<_, String>(2)?,
                    "total_cost": row.get::<_, f64>(3)?,
                    "created_at": row.get::<_, String>(4)?,
                }))
            },
        );

        match result {
            Ok(chain) => Ok(Some(chain)),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                // Fall back to most recent completed chain
                let result = self.conn.query_row(
                    "SELECT id, original_task, status, total_cost, created_at FROM chains ORDER BY created_at DESC LIMIT 1",
                    [],
                    |row| {
                        Ok(json!({
                            "id": row.get::<_, String>(0)?,
                            "original_task": row.get::<_, String>(1)?,
                            "status": row.get::<_, String>(2)?,
                            "total_cost": row.get::<_, f64>(3)?,
                            "created_at": row.get::<_, String>(4)?,
                        }))
                    },
                );
                match result {
                    Ok(chain) => Ok(Some(chain)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(e),
                }
            }
            Err(e) => Err(e),
        }
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

    // ── R18: Trigger / automation methods ─────────────────────

    pub fn get_triggers(&self) -> Result<Vec<Trigger>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, trigger_type, config, task_text, enabled, last_run, created_at
             FROM triggers ORDER BY created_at DESC",
        )?;
        let triggers = stmt
            .query_map([], |row| {
                Ok(Trigger {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    trigger_type: row.get(2)?,
                    config: row.get(3)?,
                    task_text: row.get(4)?,
                    enabled: row.get::<_, i32>(5)? == 1,
                    last_run: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(triggers)
    }

    pub fn get_enabled_triggers(&self) -> Result<Vec<Trigger>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, trigger_type, config, task_text, enabled, last_run, created_at
             FROM triggers WHERE enabled = 1 ORDER BY created_at DESC",
        )?;
        let triggers = stmt
            .query_map([], |row| {
                Ok(Trigger {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    trigger_type: row.get(2)?,
                    config: row.get(3)?,
                    task_text: row.get(4)?,
                    enabled: row.get::<_, i32>(5)? == 1,
                    last_run: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(triggers)
    }

    pub fn create_trigger(
        &self,
        id: &str,
        name: &str,
        trigger_type: &str,
        config: &str,
        task_text: &str,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO triggers (id, name, trigger_type, config, task_text)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, name, trigger_type, config, task_text],
        )?;
        Ok(())
    }

    pub fn update_trigger(
        &self,
        id: &str,
        name: &str,
        config: &str,
        task_text: &str,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE triggers SET name = ?2, config = ?3, task_text = ?4 WHERE id = ?1",
            params![id, name, config, task_text],
        )?;
        Ok(())
    }

    pub fn delete_trigger(&self, id: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM triggers WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn toggle_trigger(&self, id: &str, enabled: bool) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE triggers SET enabled = ?2 WHERE id = ?1",
            params![id, enabled as i32],
        )?;
        Ok(())
    }

    /// Expose the underlying connection for sub-modules (e.g. MemoryStore)
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn update_trigger_last_run(&self, id: &str, last_run: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE triggers SET last_run = ?2 WHERE id = ?1",
            params![id, last_run],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    fn temp_db() -> (Database, PathBuf) {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        // Close the temp file so SQLite can open it
        drop(tmp);
        let db = Database::new(&path).unwrap();
        (db, path)
    }

    fn sample_response(task_id: &str) -> LLMResponse {
        LLMResponse {
            task_id: task_id.to_string(),
            content: "Hello, world!".to_string(),
            model: "anthropic/haiku".to_string(),
            provider: "anthropic".to_string(),
            tokens_in: 10,
            tokens_out: 20,
            cost: 0.001,
            duration_ms: 500,
        }
    }

    // ── Migration ──────────────────────────────────────────────

    #[test]
    fn migration_creates_tables() {
        let (db, _) = temp_db();
        // If we got here without error, tables were created
        // Verify by inserting and querying
        let tasks = db.get_tasks(10).unwrap();
        let arr = tasks.as_array().unwrap();
        assert_eq!(arr.len(), 0);
    }

    #[test]
    fn migration_is_idempotent() {
        let (db, path) = temp_db();
        // Opening again runs migrate() again — should not error
        let _db2 = Database::new(&path).unwrap();
        let tasks = db.get_tasks(10).unwrap();
        assert!(tasks.as_array().unwrap().is_empty());
    }

    // ── Insert and retrieve task ───────────────────────────────

    #[test]
    fn insert_task_and_get_by_list() {
        let (db, _) = temp_db();
        let resp = sample_response("task_001");
        db.insert_task("hello", &resp).unwrap();

        let tasks = db.get_tasks(10).unwrap();
        let arr = tasks.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["task_id"], "task_001");
        assert_eq!(arr[0]["input"], "hello");
        assert_eq!(arr[0]["output"], "Hello, world!");
        assert_eq!(arr[0]["status"], "completed");
        assert_eq!(arr[0]["model"], "anthropic/haiku");
        assert_eq!(arr[0]["provider"], "anthropic");
    }

    #[test]
    fn get_tasks_respects_limit() {
        let (db, _) = temp_db();
        for i in 0..5 {
            let resp = sample_response(&format!("task_{}", i));
            db.insert_task(&format!("input {}", i), &resp).unwrap();
        }
        let tasks = db.get_tasks(3).unwrap();
        assert_eq!(tasks.as_array().unwrap().len(), 3);
    }

    #[test]
    fn get_tasks_ordered_by_created_desc() {
        let (db, _) = temp_db();
        for i in 0..3 {
            let resp = sample_response(&format!("task_{}", i));
            db.insert_task(&format!("input {}", i), &resp).unwrap();
        }
        let tasks = db.get_tasks(10).unwrap();
        let arr = tasks.as_array().unwrap();
        // Last inserted should be first (DESC order)
        assert_eq!(arr[0]["task_id"], "task_2");
    }

    // ── Pending task lifecycle ──────────────────────────────────

    #[test]
    fn create_pending_and_update_status() {
        let (db, _) = temp_db();
        db.create_task_pending("task_p1", "do something").unwrap();

        let tasks = db.get_tasks(10).unwrap();
        let arr = tasks.as_array().unwrap();
        assert_eq!(arr[0]["status"], "running");

        db.update_task_status("task_p1", "completed").unwrap();
        let tasks = db.get_tasks(10).unwrap();
        let arr = tasks.as_array().unwrap();
        assert_eq!(arr[0]["status"], "completed");
    }

    #[test]
    fn update_task_output() {
        let (db, _) = temp_db();
        db.create_task_pending("task_p2", "do another thing").unwrap();
        db.update_task_output("task_p2", "Result: success").unwrap();

        let tasks = db.get_tasks(10).unwrap();
        let arr = tasks.as_array().unwrap();
        assert_eq!(arr[0]["output"], "Result: success");
    }

    // ── Task steps ─────────────────────────────────────────────

    #[test]
    fn insert_and_get_task_steps() {
        let (db, _) = temp_db();
        db.create_task_pending("task_s1", "multi-step task").unwrap();

        db.insert_task_step("task_s1", 1, "run_command", "echo hi", "", "terminal", true, 100).unwrap();
        db.insert_task_step("task_s1", 2, "click", "click button", "", "screen", true, 50).unwrap();

        let steps = db.get_task_steps("task_s1").unwrap();
        let arr = steps.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["step_number"], 1);
        assert_eq!(arr[0]["action_type"], "run_command");
        assert_eq!(arr[0]["success"], true);
        assert_eq!(arr[1]["step_number"], 2);
        assert_eq!(arr[1]["action_type"], "click");
    }

    #[test]
    fn steps_ordered_by_step_number() {
        let (db, _) = temp_db();
        db.create_task_pending("task_s2", "ordered steps").unwrap();
        // Insert out of order
        db.insert_task_step("task_s2", 3, "type", "typing", "", "screen", true, 30).unwrap();
        db.insert_task_step("task_s2", 1, "click", "first click", "", "screen", true, 10).unwrap();
        db.insert_task_step("task_s2", 2, "run_command", "cmd", "", "terminal", true, 20).unwrap();

        let steps = db.get_task_steps("task_s2").unwrap();
        let arr = steps.as_array().unwrap();
        assert_eq!(arr[0]["step_number"], 1);
        assert_eq!(arr[1]["step_number"], 2);
        assert_eq!(arr[2]["step_number"], 3);
    }

    // ── Analytics ──────────────────────────────────────────────

    #[test]
    fn analytics_empty_db() {
        let (db, _) = temp_db();
        let a = db.get_analytics().unwrap();
        assert_eq!(a["total_tasks"], 0);
        assert_eq!(a["success_rate"], 0.0);
        assert_eq!(a["total_cost"], 0.0);
        assert_eq!(a["total_tokens"], 0);
    }

    #[test]
    fn analytics_with_tasks() {
        let (db, _) = temp_db();
        let r1 = LLMResponse {
            task_id: "t1".into(), content: "ok".into(), model: "haiku".into(),
            provider: "anthropic".into(), tokens_in: 100, tokens_out: 200,
            cost: 0.01, duration_ms: 500,
        };
        let r2 = LLMResponse {
            task_id: "t2".into(), content: "ok".into(), model: "haiku".into(),
            provider: "anthropic".into(), tokens_in: 50, tokens_out: 150,
            cost: 0.005, duration_ms: 300,
        };
        db.insert_task("hello", &r1).unwrap();
        db.insert_task("world", &r2).unwrap();

        // Add a failed task
        db.create_task_pending("t3", "fail task").unwrap();
        db.update_task_status("t3", "failed").unwrap();

        let a = db.get_analytics().unwrap();
        assert_eq!(a["total_tasks"], 3);
        // 2 completed out of 3
        let rate = a["success_rate"].as_f64().unwrap();
        assert!((rate - 66.666).abs() < 1.0);
        let cost = a["total_cost"].as_f64().unwrap();
        assert!((cost - 0.015).abs() < 0.001);
        assert_eq!(a["total_tokens"], 500); // 100+200+50+150
    }

    // ── LLM calls ──────────────────────────────────────────────

    #[test]
    fn insert_llm_call() {
        let (db, _) = temp_db();
        db.create_task_pending("t_llm", "test llm").unwrap();
        db.insert_llm_call("t_llm", "anthropic", "haiku", 100, 200, 0.01, 500).unwrap();
        // No panic = success. The llm_calls table is not directly queried in the current API,
        // but insert_task already creates one, so we verify it doesn't error.
    }

    // ── Trigger CRUD ──────────────────────────────────────────

    #[test]
    fn create_and_list_triggers() {
        let (db, _) = temp_db();
        db.create_trigger("tr1", "Morning report", "cron", r#"{"cron":"0 9 * * *"}"#, "Generate daily report")
            .unwrap();
        db.create_trigger("tr2", "Hourly check", "cron", r#"{"cron":"0 * * * *"}"#, "Check system status")
            .unwrap();

        let triggers = db.get_triggers().unwrap();
        assert_eq!(triggers.len(), 2);
        let names: Vec<&str> = triggers.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"Morning report"));
        assert!(names.contains(&"Hourly check"));
    }

    #[test]
    fn toggle_trigger() {
        let (db, _) = temp_db();
        db.create_trigger("tr1", "Test", "cron", r#"{"cron":"*/5 * * * *"}"#, "test")
            .unwrap();

        let enabled = db.get_enabled_triggers().unwrap();
        assert_eq!(enabled.len(), 1);

        db.toggle_trigger("tr1", false).unwrap();
        let enabled = db.get_enabled_triggers().unwrap();
        assert_eq!(enabled.len(), 0);

        let all = db.get_triggers().unwrap();
        assert_eq!(all.len(), 1);
        assert!(!all[0].enabled);
    }

    #[test]
    fn update_trigger() {
        let (db, _) = temp_db();
        db.create_trigger("tr1", "Old Name", "cron", r#"{"cron":"*/5 * * * *"}"#, "old task")
            .unwrap();

        db.update_trigger("tr1", "New Name", r#"{"cron":"*/10 * * * *"}"#, "new task")
            .unwrap();

        let triggers = db.get_triggers().unwrap();
        assert_eq!(triggers[0].name, "New Name");
        assert_eq!(triggers[0].task_text, "new task");
    }

    #[test]
    fn delete_trigger() {
        let (db, _) = temp_db();
        db.create_trigger("tr1", "Test", "cron", r#"{"cron":"*/5 * * * *"}"#, "test")
            .unwrap();

        db.delete_trigger("tr1").unwrap();
        let triggers = db.get_triggers().unwrap();
        assert_eq!(triggers.len(), 0);
    }

    #[test]
    fn update_trigger_last_run() {
        let (db, _) = temp_db();
        db.create_trigger("tr1", "Test", "cron", r#"{"cron":"*/5 * * * *"}"#, "test")
            .unwrap();

        db.update_trigger_last_run("tr1", "2026-03-29T10:00:00Z").unwrap();

        let triggers = db.get_triggers().unwrap();
        assert_eq!(triggers[0].last_run, Some("2026-03-29T10:00:00Z".to_string()));
    }
}
