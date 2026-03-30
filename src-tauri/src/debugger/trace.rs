use serde::{Deserialize, Serialize};

/// Phases of agent execution pipeline
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

/// A single step in an execution trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceStep {
    pub phase: String,
    pub input: String,
    pub output: String,
    pub decision: String,
    pub duration_ms: u64,
    pub cost: f64,
    pub tokens: u32,
}

/// A complete execution trace for a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTrace {
    pub id: String,
    pub task_id: String,
    pub steps: Vec<TraceStep>,
    pub total_duration_ms: u64,
    pub total_cost: f64,
    pub created_at: String,
    pub finished: bool,
}

/// In-memory debugger that records execution traces
pub struct AgentDebugger {
    traces: Vec<ExecutionTrace>,
}

impl AgentDebugger {
    pub fn new() -> Self {
        Self {
            traces: Vec::new(),
        }
    }

    /// Start a new trace for a task, returns the trace ID
    pub fn start_trace(&mut self, task_id: &str) -> String {
        let trace_id = uuid::Uuid::new_v4().to_string();
        let trace = ExecutionTrace {
            id: trace_id.clone(),
            task_id: task_id.to_string(),
            steps: Vec::new(),
            total_duration_ms: 0,
            total_cost: 0.0,
            created_at: chrono::Utc::now().to_rfc3339(),
            finished: false,
        };
        self.traces.push(trace);
        trace_id
    }

    /// Add a step to an existing trace
    pub fn add_step(&mut self, trace_id: &str, step: TraceStep) -> Result<(), String> {
        let trace = self
            .traces
            .iter_mut()
            .find(|t| t.id == trace_id)
            .ok_or_else(|| format!("Trace not found: {}", trace_id))?;

        if trace.finished {
            return Err("Trace already finished".to_string());
        }

        trace.total_duration_ms += step.duration_ms;
        trace.total_cost += step.cost;
        trace.steps.push(step);
        Ok(())
    }

    /// Finish a trace (no more steps can be added)
    pub fn finish_trace(&mut self, trace_id: &str) -> Result<(), String> {
        let trace = self
            .traces
            .iter_mut()
            .find(|t| t.id == trace_id)
            .ok_or_else(|| format!("Trace not found: {}", trace_id))?;
        trace.finished = true;
        Ok(())
    }

    /// Get a trace by ID
    pub fn get_trace(&self, trace_id: &str) -> Option<&ExecutionTrace> {
        self.traces.iter().find(|t| t.id == trace_id)
    }

    /// List recent traces
    pub fn list_traces(&self, limit: usize) -> Vec<&ExecutionTrace> {
        let start = if self.traces.len() > limit {
            self.traces.len() - limit
        } else {
            0
        };
        self.traces[start..].iter().collect()
    }
}
