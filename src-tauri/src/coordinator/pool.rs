use crate::agent_loop::{AgentLoopConfig, AgentRuntime};
use crate::brain::Gateway;
use crate::config::Settings;
use crate::coordinator::event_bus::{CoordinatorEvent, EventBus};
use crate::coordinator::specialists::{SpecialistProfile, SpecialistRegistry};
use crate::coordinator::types::*;
use crate::tools::{ExecutionMode, ToolContext, ToolRegistry};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use thiserror::Error;

pub struct AgentPool {
    specialists: Arc<SpecialistRegistry>,
    tool_registry: Arc<ToolRegistry>,
    gateway: Arc<tokio::sync::Mutex<Gateway>>,
    active_workers: tokio::sync::Mutex<HashMap<String, AgentWorker>>,
    session_store: Arc<crate::agent_loop::session::SessionStore>,
    db_path: PathBuf,
    app_data_dir: PathBuf,
    kill_switch: Arc<AtomicBool>,
}

#[derive(Clone)]
pub struct AgentWorker {
    pub id: String,
    pub assignment: AgentAssignment,
    pub runtime: AgentRuntime,
    pub status: WorkerStatus,
    pub execution_target: ExecutionTarget,
}

#[derive(Clone)]
pub enum WorkerStatus {
    Idle,
    Working { subtask_id: String },
    Completed,
    Failed { error: String },
}

pub struct SubtaskExecutionResult {
    pub output: String,
    pub cost: f64,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub elapsed_ms: u64,
    pub last_message: Option<String>,
}

#[derive(Debug, Error)]
pub enum PoolError {
    #[error("Unknown worker '{0}'")]
    UnknownWorker(String),
    #[error("Unknown specialist '{0}'")]
    UnknownSpecialist(String),
    #[error("Unknown tool '{0}'")]
    UnknownTool(String),
    #[error("Execution error: {0}")]
    ExecutionError(String),
}

impl AgentPool {
    pub fn new(
        specialists: Arc<SpecialistRegistry>,
        tool_registry: Arc<ToolRegistry>,
        gateway: Arc<tokio::sync::Mutex<Gateway>>,
        session_store: Arc<crate::agent_loop::session::SessionStore>,
        db_path: PathBuf,
        app_data_dir: PathBuf,
        kill_switch: Arc<AtomicBool>,
    ) -> Self {
        Self {
            specialists,
            tool_registry,
            gateway,
            active_workers: tokio::sync::Mutex::new(HashMap::new()),
            session_store,
            db_path,
            app_data_dir,
            kill_switch,
        }
    }

    pub async fn spawn_worker(&self, node: &DAGNode) -> Result<String, PoolError> {
        let specialist = node
            .assignment
            .specialist
            .as_deref()
            .map(|id| {
                self.specialists
                    .get(id)
                    .ok_or_else(|| PoolError::UnknownSpecialist(id.to_string()))
            })
            .transpose()?;

        let system_prompt = specialist
            .map(|profile| profile.system_prompt.clone())
            .unwrap_or_else(|| self.default_prompt_for_level(&node.assignment.level));

        let allowed_tools = if node.allowed_tools.is_empty() {
            specialist
                .map(|profile| profile.default_tools.clone())
                .unwrap_or_default()
        } else {
            node.allowed_tools.clone()
        };

        for tool in &allowed_tools {
            if self.tool_registry.get(tool).is_none() {
                return Err(PoolError::UnknownTool(tool.clone()));
            }
        }

        let model_tier = node
            .assignment
            .model_override
            .clone()
            .or_else(|| specialist.map(|profile| profile.default_model_tier.clone()))
            .unwrap_or_else(|| node.assignment.level.default_model_tier().to_string());

        let runtime =
            AgentRuntime::new_with_restrictions(system_prompt, allowed_tools, model_tier, 25);

        let worker_id = uuid::Uuid::new_v4().to_string();
        let worker = AgentWorker {
            id: worker_id.clone(),
            assignment: node.assignment.clone(),
            runtime,
            status: WorkerStatus::Idle,
            execution_target: node.execution_target.clone(),
        };

        self.active_workers
            .lock()
            .await
            .insert(worker_id.clone(), worker);
        Ok(worker_id)
    }

    pub async fn execute_subtask(
        &self,
        mission_id: &str,
        worker_id: &str,
        subtask: &DAGNode,
        context: Vec<(String, String)>,
        event_bus: &EventBus,
        app_handle: &tauri::AppHandle,
        settings: &Settings,
    ) -> Result<SubtaskExecutionResult, PoolError> {
        let worker = {
            let mut workers = self.active_workers.lock().await;
            let worker = workers
                .get_mut(worker_id)
                .ok_or_else(|| PoolError::UnknownWorker(worker_id.to_string()))?;
            worker.status = WorkerStatus::Working {
                subtask_id: subtask.id.clone(),
            };
            worker.clone()
        };

        let mut prompt = subtask.description.clone();
        if !context.is_empty() {
            prompt.push_str("\n\n--- Context from previous tasks ---\n");
            for (title, output) in &context {
                prompt.push_str(&format!("\n### Output from '{}':\n{}\n", title, output));
            }
        }

        event_bus.emit(CoordinatorEvent::SubtaskProgress {
            mission_id: mission_id.to_string(),
            subtask_id: subtask.id.clone(),
            progress: 0.05,
            message: "Agent initialized".to_string(),
        });

        let tool_names = worker
            .runtime
            .restricted_tools()
            .map(|tools| tools.to_vec())
            .unwrap_or_default();
        let tool_defs = self
            .tool_registry
            .definitions_for_tools(&tool_names)
            .into_iter()
            .map(|definition| {
                serde_json::json!({
                    "name": definition.name,
                    "description": definition.description,
                    "input_schema": definition.input_schema,
                })
            })
            .collect::<Vec<_>>();

        let task_id = format!("{}_{}", mission_id, subtask.id);
        let agent_name = worker
            .assignment
            .specialist_name
            .clone()
            .or_else(|| {
                worker
                    .assignment
                    .specialist
                    .as_ref()
                    .and_then(|id| self.specialists.get(id))
                    .map(|profile| profile.name.clone())
            })
            .unwrap_or_else(|| format!("{:?}", worker.assignment.level));

        let execution_mode = match &worker.execution_target {
            ExecutionTarget::Local => ExecutionMode::Host,
            ExecutionTarget::DockerLocal { container_id } => {
                ExecutionMode::Sandbox { container_id: container_id.clone() }
            }
            ExecutionTarget::DockerRemote { container_id, .. } => {
                ExecutionMode::Sandbox { container_id: container_id.clone() }
            }
        };

        let ctx = ToolContext {
            agent_name,
            task_id: task_id.clone(),
            db_path: self.db_path.clone(),
            app_data_dir: self.app_data_dir.clone(),
            kill_switch: self.kill_switch.clone(),
            execution_mode,
        };

        let system_prompt = worker
            .runtime
            .restricted_system_prompt()
            .unwrap_or("You are an AgentOS specialist.")
            .to_string();

        let gateway = self.gateway.lock().await;
        let started = std::time::Instant::now();
        let result = worker
            .runtime
            .run_turn(
                &prompt,
                &system_prompt,
                &tool_defs,
                &self.tool_registry,
                &ctx,
                &gateway,
                settings,
                &self.kill_switch,
                Some(app_handle),
                Some(self.session_store.as_ref()),
                Some(&task_id),
                Some((mission_id, subtask.id.as_str())),
            )
            .await;
        drop(gateway);

        let mut workers = self.active_workers.lock().await;
        let worker_entry = workers
            .get_mut(worker_id)
            .ok_or_else(|| PoolError::UnknownWorker(worker_id.to_string()))?;

        match result {
            Ok(turn) => {
                worker_entry.status = WorkerStatus::Completed;
                let elapsed_ms = started.elapsed().as_millis() as u64;
                let tokens_in = turn.total_input_tokens as u64;
                let tokens_out = turn.total_output_tokens as u64;
                let cost = estimate_cost(
                    worker
                        .runtime
                        .model_tier_override()
                        .unwrap_or(worker.assignment.level.default_model_tier()),
                    tokens_in,
                    tokens_out,
                );

                Ok(SubtaskExecutionResult {
                    output: turn.text.clone(),
                    cost,
                    tokens_in,
                    tokens_out,
                    elapsed_ms,
                    last_message: summarize_output(&turn.text),
                })
            }
            Err(error) => {
                worker_entry.status = WorkerStatus::Failed {
                    error: error.clone(),
                };
                Err(PoolError::ExecutionError(error))
            }
        }
    }

    pub fn get_available_specialists(&self) -> Vec<SpecialistProfile> {
        self.specialists.list()
    }

    fn default_prompt_for_level(&self, level: &AgentLevel) -> String {
        match level {
            AgentLevel::Junior => {
                "You are a careful junior operator. Follow instructions literally and keep outputs concise."
                    .to_string()
            }
            AgentLevel::Specialist => {
                "You are a focused specialist. Use the provided tools only when needed and return crisp, actionable output."
                    .to_string()
            }
            AgentLevel::Senior => {
                "You are a senior specialist. Make thoughtful decisions, explain tradeoffs briefly, and aim for production-quality results."
                    .to_string()
            }
            AgentLevel::Manager => {
                "You are a manager-level agent. Coordinate clearly, reason about dependencies, and structure outputs for handoff."
                    .to_string()
            }
            AgentLevel::Orchestrator => {
                "You are the orchestrator. Think in systems, keep context organized, and optimize the whole mission."
                    .to_string()
            }
        }
    }
}

fn estimate_cost(model_tier: &str, tokens_in: u64, tokens_out: u64) -> f64 {
    let (input_rate, output_rate) = match model_tier {
        "cheap" => (0.8_f64, 3.2_f64),
        "premium" => (15.0_f64, 75.0_f64),
        _ => (3.0_f64, 15.0_f64),
    };

    (tokens_in as f64 * input_rate + tokens_out as f64 * output_rate) / 1_000_000.0
}

fn summarize_output(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut summary = trimmed.lines().next().unwrap_or(trimmed).trim().to_string();
    if summary.len() > 160 {
        summary.truncate(157);
        summary.push_str("...");
    }
    Some(summary)
}
