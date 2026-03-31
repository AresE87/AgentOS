use crate::types::{AgentAction, ExecutionMethod, StepRecord, TaskExecutionResult};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const PHASES: &[&str] = &[
    "classify",
    "route",
    "agent_select",
    "prompt_build",
    "llm_call",
    "parse_response",
    "execute",
    "verify",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceStep {
    pub id: String,
    pub trace_id: String,
    pub timestamp: String,
    pub phase: String,
    pub planned_action: String,
    pub agent_name: String,
    pub model: String,
    pub input_summary: String,
    pub output_summary: String,
    pub status: String,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub cost: f64,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTrace {
    pub id: String,
    pub task_id: String,
    pub agent_name: String,
    pub model: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub total_duration_ms: u64,
    pub total_cost: f64,
    pub steps: Vec<TraceStep>,
}

pub struct AgentDebugger {
    db_path: PathBuf,
}

impl AgentDebugger {
    pub fn new(db_path: PathBuf) -> Result<Self, String> {
        let debugger = Self { db_path };
        let conn = debugger.open()?;
        Self::init_db(&conn)?;
        Ok(debugger)
    }

    fn open(&self) -> Result<Connection, String> {
        let conn = Connection::open(&self.db_path)
            .map_err(|e| format!("Failed to open debugger DB: {}", e))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| format!("Failed to configure debugger DB: {}", e))?;
        Self::init_db(&conn)?;
        Ok(conn)
    }

    pub fn init_db(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS execution_traces (
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL,
                agent_name TEXT NOT NULL,
                model TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                total_duration_ms INTEGER NOT NULL DEFAULT 0,
                total_cost REAL NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS execution_trace_steps (
                id TEXT PRIMARY KEY,
                trace_id TEXT NOT NULL REFERENCES execution_traces(id) ON DELETE CASCADE,
                timestamp TEXT NOT NULL,
                phase TEXT NOT NULL,
                planned_action TEXT NOT NULL,
                agent_name TEXT NOT NULL,
                model TEXT NOT NULL,
                input_summary TEXT NOT NULL,
                output_summary TEXT NOT NULL,
                status TEXT NOT NULL,
                error TEXT,
                duration_ms INTEGER NOT NULL DEFAULT 0,
                cost REAL NOT NULL DEFAULT 0,
                evidence_json TEXT NOT NULL DEFAULT '[]'
            );
            CREATE INDEX IF NOT EXISTS idx_execution_traces_task ON execution_traces(task_id, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_execution_traces_status ON execution_traces(status, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_execution_traces_agent ON execution_traces(agent_name, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_execution_trace_steps_trace ON execution_trace_steps(trace_id, timestamp ASC);",
        )
        .map_err(|e| format!("Failed to initialize debugger tables: {}", e))
    }

    pub fn start_trace(
        &self,
        task_id: &str,
        agent_name: Option<&str>,
        model: Option<&str>,
    ) -> Result<String, String> {
        let conn = self.open()?;
        let trace_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO execution_traces (id, task_id, agent_name, model, status, created_at, updated_at, total_duration_ms, total_cost)
             VALUES (?1, ?2, ?3, ?4, 'running', ?5, ?5, 0, 0)",
            params![
                trace_id,
                task_id,
                agent_name.unwrap_or("agent"),
                model.unwrap_or("unknown"),
                now,
            ],
        )
        .map_err(|e| format!("Failed to create execution trace: {}", e))?;
        Ok(trace_id)
    }

    pub fn add_step(&self, trace_id: &str, step: TraceStep) -> Result<(), String> {
        let conn = self.open()?;
        self.ensure_trace_exists(&conn, trace_id)?;
        conn.execute(
            "INSERT INTO execution_trace_steps (
                id, trace_id, timestamp, phase, planned_action, agent_name, model,
                input_summary, output_summary, status, error, duration_ms, cost, evidence_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                step.id,
                trace_id,
                step.timestamp,
                step.phase,
                step.planned_action,
                step.agent_name,
                step.model,
                step.input_summary,
                step.output_summary,
                step.status,
                step.error,
                step.duration_ms as i64,
                step.cost,
                serde_json::to_string(&step.evidence).map_err(|e| e.to_string())?,
            ],
        )
        .map_err(|e| format!("Failed to store trace step: {}", e))?;
        conn.execute(
            "UPDATE execution_traces
             SET updated_at = ?2,
                 total_duration_ms = total_duration_ms + ?3,
                 total_cost = total_cost + ?4
             WHERE id = ?1",
            params![
                trace_id,
                chrono::Utc::now().to_rfc3339(),
                step.duration_ms as i64,
                step.cost,
            ],
        )
        .map_err(|e| format!("Failed to update trace totals: {}", e))?;
        Ok(())
    }

    pub fn finish_trace(&self, trace_id: &str, status: &str) -> Result<(), String> {
        let conn = self.open()?;
        self.ensure_trace_exists(&conn, trace_id)?;
        conn.execute(
            "UPDATE execution_traces SET status = ?2, updated_at = ?3 WHERE id = ?1",
            params![trace_id, status, chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|e| format!("Failed to finish trace: {}", e))?;
        Ok(())
    }

    pub fn get_trace(&self, trace_id: &str) -> Result<Option<ExecutionTrace>, String> {
        let conn = self.open()?;
        let trace = conn
            .query_row(
                "SELECT id, task_id, agent_name, model, status, created_at, updated_at, total_duration_ms, total_cost
                 FROM execution_traces WHERE id = ?1",
                params![trace_id],
                |row| {
                    Ok(ExecutionTrace {
                        id: row.get(0)?,
                        task_id: row.get(1)?,
                        agent_name: row.get(2)?,
                        model: row.get(3)?,
                        status: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                        total_duration_ms: row.get::<_, i64>(7)? as u64,
                        total_cost: row.get(8)?,
                        steps: Vec::new(),
                    })
                },
            )
            .optional()
            .map_err(|e| format!("Failed to load trace: {}", e))?;

        let Some(mut trace) = trace else {
            return Ok(None);
        };
        trace.steps = self.list_steps(&conn, &trace.id)?;
        Ok(Some(trace))
    }

    pub fn list_traces(
        &self,
        limit: usize,
        task_id: Option<&str>,
        agent_name: Option<&str>,
        status: Option<&str>,
    ) -> Result<Vec<ExecutionTrace>, String> {
        let conn = self.open()?;
        let mut stmt = conn
            .prepare("SELECT id FROM execution_traces ORDER BY created_at DESC")
            .map_err(|e| format!("Failed to prepare trace list query: {}", e))?;
        let ids = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| format!("Failed to query traces: {}", e))?;
        let mut traces = Vec::new();
        for id in ids.flatten() {
            if let Some(trace) = self.get_trace(&id)? {
                let matches_task = task_id.map(|value| trace.task_id == value).unwrap_or(true);
                let matches_agent =
                    agent_name.map(|value| trace.agent_name == value).unwrap_or(true);
                let matches_status = status.map(|value| trace.status == value).unwrap_or(true);
                if matches_task && matches_agent && matches_status {
                    traces.push(trace);
                    if traces.len() >= limit {
                        break;
                    }
                }
            }
        }
        Ok(traces)
    }

    pub fn record_task_execution(
        &self,
        task_id: &str,
        agent_name: &str,
        model: &str,
        result: &TaskExecutionResult,
    ) -> Result<ExecutionTrace, String> {
        let trace_id = self.start_trace(task_id, Some(agent_name), Some(model))?;
        for step in &result.steps {
            self.add_step(&trace_id, Self::trace_step_from_record(&trace_id, step, agent_name, model))?;
        }
        self.finish_trace(
            &trace_id,
            if result.success { "completed" } else { "failed" },
        )?;
        self.get_trace(&trace_id)?
            .ok_or_else(|| format!("Trace not found after recording: {}", trace_id))
    }

    pub fn record_runtime_error(
        &self,
        task_id: &str,
        agent_name: &str,
        model: &str,
        error: &str,
    ) -> Result<ExecutionTrace, String> {
        let trace_id = self.start_trace(task_id, Some(agent_name), Some(model))?;
        self.add_step(
            &trace_id,
            TraceStep {
                id: uuid::Uuid::new_v4().to_string(),
                trace_id: trace_id.clone(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                phase: "execute".to_string(),
                planned_action: "runtime_error".to_string(),
                agent_name: agent_name.to_string(),
                model: model.to_string(),
                input_summary: format!("Task {} failed before producing a result.", task_id),
                output_summary: String::new(),
                status: "failed".to_string(),
                error: Some(error.to_string()),
                duration_ms: 0,
                cost: 0.0,
                evidence: vec![error.to_string()],
            },
        )?;
        self.finish_trace(&trace_id, "failed")?;
        self.get_trace(&trace_id)?
            .ok_or_else(|| format!("Trace not found after recording: {}", trace_id))
    }

    fn ensure_trace_exists(&self, conn: &Connection, trace_id: &str) -> Result<(), String> {
        let exists: Option<String> = conn
            .query_row(
                "SELECT id FROM execution_traces WHERE id = ?1",
                params![trace_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("Failed to check trace existence: {}", e))?;
        if exists.is_none() {
            return Err(format!("Trace not found: {}", trace_id));
        }
        Ok(())
    }

    fn list_steps(&self, conn: &Connection, trace_id: &str) -> Result<Vec<TraceStep>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT id, trace_id, timestamp, phase, planned_action, agent_name, model,
                        input_summary, output_summary, status, error, duration_ms, cost, evidence_json
                 FROM execution_trace_steps
                 WHERE trace_id = ?1
                 ORDER BY timestamp ASC",
            )
            .map_err(|e| format!("Failed to prepare trace steps query: {}", e))?;

        let rows = stmt
            .query_map(params![trace_id], |row| {
                let evidence_json: String = row.get(13)?;
                Ok(TraceStep {
                    id: row.get(0)?,
                    trace_id: row.get(1)?,
                    timestamp: row.get(2)?,
                    phase: row.get(3)?,
                    planned_action: row.get(4)?,
                    agent_name: row.get(5)?,
                    model: row.get(6)?,
                    input_summary: row.get(7)?,
                    output_summary: row.get(8)?,
                    status: row.get(9)?,
                    error: row.get(10)?,
                    duration_ms: row.get::<_, i64>(11)? as u64,
                    cost: row.get(12)?,
                    evidence: serde_json::from_str(&evidence_json).unwrap_or_default(),
                })
            })
            .map_err(|e| format!("Failed to query trace steps: {}", e))?;
        Ok(rows.flatten().collect())
    }

    fn trace_step_from_record(
        trace_id: &str,
        record: &StepRecord,
        agent_name: &str,
        model: &str,
    ) -> TraceStep {
        let planned_action = action_label(&record.action);
        let output_summary = record
            .result
            .output
            .clone()
            .unwrap_or_else(|| planned_action.clone());
        let error = if record.result.success {
            None
        } else {
            Some(output_summary.clone())
        };

        let mut evidence = Vec::new();
        if let Some(path) = record
            .screenshot_path
            .clone()
            .or_else(|| record.result.screenshot_path.clone())
        {
            evidence.push(path);
        }
        evidence.push(format!("execution_method={}", method_label(record.result.method.clone())));

        TraceStep {
            id: uuid::Uuid::new_v4().to_string(),
            trace_id: trace_id.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            phase: "execute".to_string(),
            planned_action: planned_action.clone(),
            agent_name: agent_name.to_string(),
            model: model.to_string(),
            input_summary: format!("Step {} planned action {}", record.step_number, planned_action),
            output_summary,
            status: if record.result.success {
                "completed".to_string()
            } else {
                "failed".to_string()
            },
            error,
            duration_ms: record.result.duration_ms,
            cost: 0.0,
            evidence,
        }
    }
}

fn action_label(action: &AgentAction) -> String {
    match action {
        AgentAction::Click { x, y } => format!("click at {}, {}", x, y),
        AgentAction::DoubleClick { x, y } => format!("double click at {}, {}", x, y),
        AgentAction::RightClick { x, y } => format!("right click at {}, {}", x, y),
        AgentAction::Type { text } => format!("type '{}'", trim_text(text)),
        AgentAction::KeyCombo { keys } => format!("press {}", keys.join("+")),
        AgentAction::Scroll { delta, .. } => format!("scroll {}", delta),
        AgentAction::RunCommand { command, .. } => format!("run '{}'", trim_text(command)),
        AgentAction::Wait { ms } => format!("wait {} ms", ms),
        AgentAction::Screenshot => "capture screenshot".to_string(),
        AgentAction::TaskComplete { summary } => format!("complete task '{}'", trim_text(summary)),
    }
}

fn method_label(method: ExecutionMethod) -> &'static str {
    match method {
        ExecutionMethod::Api => "api",
        ExecutionMethod::Terminal => "terminal",
        ExecutionMethod::Screen => "screen",
    }
}

fn trim_text(text: &str) -> String {
    const MAX: usize = 120;
    if text.len() > MAX {
        format!("{}...", &text[..MAX])
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ExecutionResult, ExecutionMethod, StepRecord, TaskExecutionResult};

    fn sample_result(success: bool) -> TaskExecutionResult {
        TaskExecutionResult {
            task_id: "task-debug".to_string(),
            success,
            total_cost: 0.0,
            duration_ms: 6200,
            steps: vec![
                StepRecord {
                    step_number: 1,
                    action: AgentAction::RunCommand {
                        command: "whoami".to_string(),
                        shell: crate::types::ShellType::PowerShell,
                    },
                    result: ExecutionResult {
                        method: ExecutionMethod::Terminal,
                        success: true,
                        output: Some("eatrujil".to_string()),
                        screenshot_path: None,
                        duration_ms: 400,
                    },
                    screenshot_path: None,
                },
                StepRecord {
                    step_number: 2,
                    action: AgentAction::RunCommand {
                        command: "pwd".to_string(),
                        shell: crate::types::ShellType::PowerShell,
                    },
                    result: ExecutionResult {
                        method: ExecutionMethod::Terminal,
                        success: true,
                        output: Some("C:/workspace".to_string()),
                        screenshot_path: None,
                        duration_ms: 500,
                    },
                    screenshot_path: None,
                },
                StepRecord {
                    step_number: 3,
                    action: AgentAction::Screenshot,
                    result: ExecutionResult {
                        method: ExecutionMethod::Screen,
                        success: true,
                        output: Some("screen captured".to_string()),
                        screenshot_path: Some("shot-1.png".to_string()),
                        duration_ms: 1200,
                    },
                    screenshot_path: Some("shot-1.png".to_string()),
                },
                StepRecord {
                    step_number: 4,
                    action: AgentAction::Click { x: 320, y: 180 },
                    result: ExecutionResult {
                        method: ExecutionMethod::Screen,
                        success: true,
                        output: Some("clicked".to_string()),
                        screenshot_path: None,
                        duration_ms: 800,
                    },
                    screenshot_path: None,
                },
                StepRecord {
                    step_number: 5,
                    action: AgentAction::TaskComplete {
                        summary: if success {
                            "completed".to_string()
                        } else {
                            "failed".to_string()
                        },
                    },
                    result: ExecutionResult {
                        method: ExecutionMethod::Api,
                        success,
                        output: Some(if success {
                            "task finished".to_string()
                        } else {
                            "task failed".to_string()
                        }),
                        screenshot_path: None,
                        duration_ms: 3300,
                    },
                    screenshot_path: None,
                },
            ],
        }
    }

    #[test]
    fn record_task_execution_persists_five_real_steps_and_filters() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("debugger.db");
        let debugger = AgentDebugger::new(db_path.clone()).unwrap();

        let trace = debugger
            .record_task_execution("task-debug", "PC Controller", "anthropic/sonnet", &sample_result(true))
            .unwrap();

        assert_eq!(trace.task_id, "task-debug");
        assert_eq!(trace.steps.len(), 5);
        assert_eq!(trace.status, "completed");

        let listed = debugger
            .list_traces(10, Some("task-debug"), Some("PC Controller"), Some("completed"))
            .unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].steps.len(), 5);
    }

    #[test]
    fn runtime_errors_are_persisted_as_failed_traces() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("debugger_error.db");
        let debugger = AgentDebugger::new(db_path).unwrap();

        let trace = debugger
            .record_runtime_error("task-error", "PC Controller", "anthropic/sonnet", "Browser launch failed")
            .unwrap();

        assert_eq!(trace.status, "failed");
        assert_eq!(trace.steps.len(), 1);
        assert_eq!(trace.steps[0].error.as_deref(), Some("Browser launch failed"));
    }
}
