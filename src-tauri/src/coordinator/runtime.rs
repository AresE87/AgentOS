use crate::brain::Gateway;
use crate::config::Settings;
use crate::coordinator::event_bus::{CoordinatorEvent, EventBus};
use crate::coordinator::planner::{PlannerError, TaskPlanner};
use crate::coordinator::pool::{AgentPool, PoolError};
use crate::coordinator::scheduler::{SchedulerError, TaskScheduler};
use crate::coordinator::specialists::SpecialistRegistry;
use crate::coordinator::templates::MissionTemplates;
use crate::coordinator::types::*;
use crate::tools::ToolRegistry;
use crate::AppState;
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::State;
use thiserror::Error;

pub struct CoordinatorRuntime {
    planner: TaskPlanner,
    scheduler: TaskScheduler,
    pool: Arc<AgentPool>,
    event_bus: Arc<EventBus>,
    active_mission: Arc<tokio::sync::Mutex<Option<Mission>>>,
    specialists: Arc<SpecialistRegistry>,
    tool_registry: Arc<ToolRegistry>,
    db_path: PathBuf,
    execution_lock: tokio::sync::Mutex<()>,
}

#[derive(Debug, Error)]
pub enum CoordinatorError {
    #[error("Planner error: {0}")]
    Planner(#[from] PlannerError),
    #[error("Pool error: {0}")]
    Pool(#[from] PoolError),
    #[error("Scheduler error: {0}")]
    Scheduler(#[from] SchedulerError),
    #[error("No active mission")]
    NoActiveMission,
    #[error("Mission '{0}' not found")]
    MissionNotFound(String),
    #[error("Subtask '{0}' not found")]
    SubtaskNotFound(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Invalid DAG: {0}")]
    InvalidDag(String),
    #[error("Storage error: {0}")]
    Storage(String),
}

#[derive(Debug, Deserialize)]
struct SubtaskPatch {
    title: Option<String>,
    description: Option<String>,
    allowed_tools: Option<Vec<String>>,
    assignment: Option<AgentAssignment>,
    status: Option<SubtaskStatus>,
}

impl CoordinatorRuntime {
    pub fn new(
        gateway: Arc<tokio::sync::Mutex<Gateway>>,
        tool_registry: Arc<ToolRegistry>,
        session_store: Arc<crate::agent_loop::session::SessionStore>,
        db_path: PathBuf,
        app_data_dir: PathBuf,
        kill_switch: Arc<AtomicBool>,
        app_handle: tauri::AppHandle,
    ) -> Self {
        let specialists = Arc::new(SpecialistRegistry::new());
        let event_bus = Arc::new(EventBus::new());
        event_bus.set_handle(app_handle);

        Self {
            planner: TaskPlanner::new(gateway.clone(), specialists.clone(), tool_registry.clone()),
            scheduler: TaskScheduler::new(event_bus.clone()),
            pool: Arc::new(AgentPool::new(
                specialists.clone(),
                tool_registry.clone(),
                gateway,
                session_store,
                db_path.clone(),
                app_data_dir,
                kill_switch,
            )),
            event_bus,
            active_mission: Arc::new(tokio::sync::Mutex::new(None)),
            specialists,
            tool_registry,
            db_path,
            execution_lock: tokio::sync::Mutex::new(()),
        }
    }

    pub async fn create_autopilot_mission(
        &self,
        description: &str,
        autonomy: AutonomyLevel,
        settings: &Settings,
    ) -> Result<Mission, CoordinatorError> {
        let mission_id = uuid::Uuid::new_v4().to_string();
        self.event_bus.emit(CoordinatorEvent::MissionPlanning {
            mission_id: mission_id.clone(),
        });

        let dag = self.planner.plan_auto(description, settings).await?;
        let mission = Mission {
            id: mission_id,
            title: extract_title(description),
            description: description.to_string(),
            mode: CoordinatorMode::Autopilot,
            autonomy,
            dag,
            status: MissionStatus::Ready,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            total_cost: 0.0,
            total_tokens: 0,
            total_elapsed_ms: 0,
        };

        self.save_mission(&mission)?;
        self.append_log(
            &mission.id,
            "info",
            "Coordinator",
            "orchestrator",
            None,
            "Mission created in autopilot mode",
            None,
        )?;

        self.event_bus.emit(CoordinatorEvent::MissionCreated {
            mission_id: mission.id.clone(),
            title: mission.title.clone(),
            mode: "autopilot".to_string(),
        });
        self.event_bus.emit(CoordinatorEvent::MissionPlanReady {
            mission_id: mission.id.clone(),
            node_count: mission.dag.nodes.len() as u32,
            edge_count: mission.dag.edges.len() as u32,
        });

        *self.active_mission.lock().await = Some(mission.clone());
        Ok(mission)
    }

    pub async fn create_commander_mission(
        &self,
        dag_json: serde_json::Value,
    ) -> Result<Mission, CoordinatorError> {
        let dag = self.planner.plan_manual(dag_json)?;
        let mission = Mission {
            id: uuid::Uuid::new_v4().to_string(),
            title: derive_commander_title(&dag),
            description: "Commander mission".to_string(),
            mode: CoordinatorMode::Commander,
            autonomy: AutonomyLevel::Full,
            dag,
            status: MissionStatus::Ready,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            total_cost: 0.0,
            total_tokens: 0,
            total_elapsed_ms: 0,
        };

        self.save_mission(&mission)?;
        self.append_log(
            &mission.id,
            "info",
            "Coordinator",
            "orchestrator",
            None,
            "Mission created in commander mode",
            None,
        )?;

        self.event_bus.emit(CoordinatorEvent::MissionCreated {
            mission_id: mission.id.clone(),
            title: mission.title.clone(),
            mode: "commander".to_string(),
        });
        self.event_bus.emit(CoordinatorEvent::MissionPlanReady {
            mission_id: mission.id.clone(),
            node_count: mission.dag.nodes.len() as u32,
            edge_count: mission.dag.edges.len() as u32,
        });

        *self.active_mission.lock().await = Some(mission.clone());
        Ok(mission)
    }

    pub async fn create_template_mission(
        &self,
        template_id: &str,
        context: &str,
    ) -> Result<Mission, CoordinatorError> {
        let dag = MissionTemplates::build(template_id, context).ok_or_else(|| {
            CoordinatorError::InvalidInput(format!("Unknown mission template '{}'", template_id))
        })?;

        let mission = Mission {
            id: uuid::Uuid::new_v4().to_string(),
            title: format_template_title(template_id, context),
            description: context.trim().to_string(),
            mode: CoordinatorMode::Commander,
            autonomy: AutonomyLevel::Full,
            dag,
            status: MissionStatus::Ready,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            total_cost: 0.0,
            total_tokens: 0,
            total_elapsed_ms: 0,
        };

        self.save_mission(&mission)?;
        self.append_log(
            &mission.id,
            "info",
            "Coordinator",
            "orchestrator",
            None,
            &format!("Mission created from template '{}'", template_id),
            Some(serde_json::json!({ "template_id": template_id, "context": context })),
        )?;

        self.event_bus.emit(CoordinatorEvent::MissionCreated {
            mission_id: mission.id.clone(),
            title: mission.title.clone(),
            mode: "commander".to_string(),
        });
        self.event_bus.emit(CoordinatorEvent::MissionPlanReady {
            mission_id: mission.id.clone(),
            node_count: mission.dag.nodes.len() as u32,
            edge_count: mission.dag.edges.len() as u32,
        });

        *self.active_mission.lock().await = Some(mission.clone());
        Ok(mission)
    }

    pub async fn start_mission(
        &self,
        mission_id: &str,
        settings: Settings,
        app_handle: tauri::AppHandle,
    ) -> Result<(), CoordinatorError> {
        let _execution = self.execution_lock.lock().await;

        {
            let active = self.active_mission.lock().await;
            let mission = active.as_ref().ok_or(CoordinatorError::NoActiveMission)?;
            if mission.id != mission_id {
                return Err(CoordinatorError::MissionNotFound(mission_id.to_string()));
            }
        }

        self.scheduler
            .execute(
                self.active_mission.clone(),
                mission_id,
                self.pool.clone(),
                settings,
                app_handle,
            )
            .await?;

        if let Some(mission) = self.active_mission.lock().await.clone() {
            self.save_mission(&mission)?;
        }

        Ok(())
    }

    pub async fn pause_mission(&self, mission_id: &str) -> Result<(), CoordinatorError> {
        let mut guard = self.active_mission.lock().await;
        let mission = ensure_active_mission_mut(&mut guard, mission_id)?;
        mission.status = MissionStatus::Paused;
        self.save_mission(mission)?;
        self.event_bus.emit(CoordinatorEvent::MissionPaused {
            mission_id: mission.id.clone(),
        });
        Ok(())
    }

    pub async fn cancel_mission(&self, mission_id: &str) -> Result<(), CoordinatorError> {
        let mut guard = self.active_mission.lock().await;
        let mission = ensure_active_mission_mut(&mut guard, mission_id)?;
        mission.status = MissionStatus::Cancelled;
        mission.completed_at = Some(Utc::now());
        self.save_mission(mission)?;
        self.event_bus.emit(CoordinatorEvent::MissionCancelled {
            mission_id: mission.id.clone(),
        });
        Ok(())
    }

    pub async fn retry_subtask(
        &self,
        mission_id: &str,
        subtask_id: &str,
    ) -> Result<(), CoordinatorError> {
        let mut guard = self.active_mission.lock().await;
        let mission = ensure_active_mission_mut(&mut guard, mission_id)?;
        let node = mission
            .dag
            .nodes
            .get_mut(subtask_id)
            .ok_or_else(|| CoordinatorError::SubtaskNotFound(subtask_id.to_string()))?;

        let attempt = node.retry_count.saturating_add(1);
        node.retry_count = attempt;
        node.status = SubtaskStatus::Queued;
        node.error = None;
        node.progress = 0.0;
        node.awaiting_approval = false;
        node.approved_to_run = mission.autonomy == AutonomyLevel::AskAlways;
        let emitted_mission_id = mission.id.clone();
        self.save_mission(mission)?;
        self.event_bus.emit(CoordinatorEvent::SubtaskRetrying {
            mission_id: emitted_mission_id,
            subtask_id: subtask_id.to_string(),
            attempt,
        });
        Ok(())
    }

    pub async fn add_subtask(
        &self,
        mission_id: &str,
        subtask_json: serde_json::Value,
    ) -> Result<String, CoordinatorError> {
        let mut node: DAGNode = serde_json::from_value(subtask_json)
            .map_err(|error| CoordinatorError::InvalidInput(error.to_string()))?;

        if node.id.trim().is_empty() {
            node.id = format!("node_{}", uuid::Uuid::new_v4().simple());
        }

        node.status = SubtaskStatus::Queued;
        node.started_at = None;
        node.completed_at = None;
        node.retry_count = 0;
        node.result = None;
        node.error = None;
        node.progress = 0.0;
        node.awaiting_approval = false;
        node.approved_to_run = false;

        let mut guard = self.active_mission.lock().await;
        let mission = ensure_active_mission_mut(&mut guard, mission_id)?;
        mission.dag.nodes.insert(node.id.clone(), node.clone());
        mission
            .dag
            .validate()
            .map_err(CoordinatorError::InvalidDag)?;
        self.save_mission(mission)?;
        self.event_bus.emit(CoordinatorEvent::NodeAdded {
            mission_id: mission.id.clone(),
            node_id: node.id.clone(),
        });
        Ok(node.id)
    }

    pub async fn remove_subtask(
        &self,
        mission_id: &str,
        subtask_id: &str,
    ) -> Result<(), CoordinatorError> {
        let mut guard = self.active_mission.lock().await;
        let mission = ensure_active_mission_mut(&mut guard, mission_id)?;
        mission
            .dag
            .nodes
            .remove(subtask_id)
            .ok_or_else(|| CoordinatorError::SubtaskNotFound(subtask_id.to_string()))?;
        mission
            .dag
            .edges
            .retain(|edge| edge.from != subtask_id && edge.to != subtask_id);
        self.save_mission(mission)?;
        self.event_bus.emit(CoordinatorEvent::NodeRemoved {
            mission_id: mission.id.clone(),
            node_id: subtask_id.to_string(),
        });
        Ok(())
    }

    pub async fn connect_subtasks(
        &self,
        mission_id: &str,
        from: &str,
        to: &str,
        edge_type: EdgeType,
    ) -> Result<(), CoordinatorError> {
        let mut guard = self.active_mission.lock().await;
        let mission = ensure_active_mission_mut(&mut guard, mission_id)?;
        mission.dag.edges.push(DAGEdge {
            from: from.to_string(),
            to: to.to_string(),
            edge_type,
        });

        if let Err(error) = mission.dag.validate() {
            mission
                .dag
                .edges
                .retain(|edge| !(edge.from == from && edge.to == to));
            return Err(CoordinatorError::InvalidDag(error));
        }

        self.save_mission(mission)?;
        self.event_bus.emit(CoordinatorEvent::EdgeAdded {
            mission_id: mission.id.clone(),
            from: from.to_string(),
            to: to.to_string(),
        });
        Ok(())
    }

    pub async fn disconnect_subtasks(
        &self,
        mission_id: &str,
        from: &str,
        to: &str,
    ) -> Result<(), CoordinatorError> {
        let mut guard = self.active_mission.lock().await;
        let mission = ensure_active_mission_mut(&mut guard, mission_id)?;
        mission
            .dag
            .edges
            .retain(|edge| !(edge.from == from && edge.to == to));
        self.save_mission(mission)?;
        self.event_bus.emit(CoordinatorEvent::EdgeRemoved {
            mission_id: mission.id.clone(),
            from: from.to_string(),
            to: to.to_string(),
        });
        Ok(())
    }

    pub async fn assign_agent(
        &self,
        mission_id: &str,
        subtask_id: &str,
        assignment: AgentAssignment,
    ) -> Result<(), CoordinatorError> {
        let mut guard = self.active_mission.lock().await;
        let mission = ensure_active_mission_mut(&mut guard, mission_id)?;
        let node = mission
            .dag
            .nodes
            .get_mut(subtask_id)
            .ok_or_else(|| CoordinatorError::SubtaskNotFound(subtask_id.to_string()))?;
        node.assignment = assignment;
        self.save_mission(mission)?;
        Ok(())
    }

    pub async fn update_position(
        &self,
        mission_id: &str,
        subtask_id: &str,
        x: f32,
        y: f32,
    ) -> Result<(), CoordinatorError> {
        let mut guard = self.active_mission.lock().await;
        let mission = ensure_active_mission_mut(&mut guard, mission_id)?;
        let node = mission
            .dag
            .nodes
            .get_mut(subtask_id)
            .ok_or_else(|| CoordinatorError::SubtaskNotFound(subtask_id.to_string()))?;
        node.position = Some(NodePosition { x, y });
        self.save_mission(mission)?;
        Ok(())
    }

    pub async fn update_subtask(
        &self,
        mission_id: &str,
        subtask_id: &str,
        patch: serde_json::Value,
    ) -> Result<(), CoordinatorError> {
        let patch: SubtaskPatch = serde_json::from_value(patch)
            .map_err(|error| CoordinatorError::InvalidInput(error.to_string()))?;

        let mut guard = self.active_mission.lock().await;
        let mission = ensure_active_mission_mut(&mut guard, mission_id)?;
        let node = mission
            .dag
            .nodes
            .get_mut(subtask_id)
            .ok_or_else(|| CoordinatorError::SubtaskNotFound(subtask_id.to_string()))?;

        if let Some(title) = patch.title {
            node.title = title;
        }
        if let Some(description) = patch.description {
            node.description = description;
        }
        if let Some(allowed_tools) = patch.allowed_tools {
            for tool in &allowed_tools {
                if self.tool_registry.get(tool).is_none() {
                    return Err(CoordinatorError::InvalidInput(format!(
                        "Unknown tool '{}'",
                        tool
                    )));
                }
            }
            node.allowed_tools = allowed_tools;
        }
        if let Some(assignment) = patch.assignment {
            if let Some(specialist_id) = assignment.specialist.as_deref() {
                if !self.specialists.exists(specialist_id) {
                    return Err(CoordinatorError::InvalidInput(format!(
                        "Unknown specialist '{}'",
                        specialist_id
                    )));
                }
            }
            node.assignment = assignment;
        }
        if let Some(status) = patch.status {
            node.status = status;
        }
        if node.status != SubtaskStatus::Paused {
            node.awaiting_approval = false;
        }
        if node.status != SubtaskStatus::Queued {
            node.approved_to_run = false;
        }

        self.save_mission(mission)?;
        Ok(())
    }

    pub async fn inject_message(
        &self,
        mission_id: &str,
        message: &str,
    ) -> Result<(), CoordinatorError> {
        let guard = self.active_mission.lock().await;
        let mission = ensure_active_mission_ref(&guard, mission_id)?;
        self.append_log(
            &mission.id,
            "decision",
            "User",
            "human",
            None,
            message,
            None,
        )?;
        Ok(())
    }

    pub async fn approve_step(
        &self,
        mission_id: &str,
        subtask_id: &str,
        approved: bool,
    ) -> Result<(), CoordinatorError> {
        let mut guard = self.active_mission.lock().await;
        let mission = ensure_active_mission_mut(&mut guard, mission_id)?;
        let node = mission
            .dag
            .nodes
            .get_mut(subtask_id)
            .ok_or_else(|| CoordinatorError::SubtaskNotFound(subtask_id.to_string()))?;

        if approved {
            if node.status == SubtaskStatus::Failed || node.error.is_some() {
                if node.retry_count < node.max_retries {
                    node.retry_count += 1;
                    self.event_bus.emit(CoordinatorEvent::SubtaskRetrying {
                        mission_id: mission.id.clone(),
                        subtask_id: subtask_id.to_string(),
                        attempt: node.retry_count,
                    });
                }
            }

            node.status = SubtaskStatus::Queued;
            node.error = None;
            node.progress = 0.0;
            node.awaiting_approval = false;
            node.approved_to_run = true;
            node.last_message = Some("Approved and queued for execution".to_string());
        } else {
            node.status = SubtaskStatus::Failed;
            node.error = Some("Execution rejected by user".to_string());
            node.awaiting_approval = false;
            node.approved_to_run = false;
            node.retry_count = node.max_retries;
            node.last_message = Some("Execution rejected by user".to_string());
        }

        self.append_log(
            &mission.id,
            "decision",
            "Coordinator",
            "orchestrator",
            Some(subtask_id),
            if approved {
                "Step approved"
            } else {
                "Step rejected"
            },
            Some(serde_json::json!({ "approved": approved })),
        )?;
        self.save_mission(mission)?;
        Ok(())
    }

    pub async fn get_mission(&self, mission_id: &str) -> Result<Mission, CoordinatorError> {
        if let Some(active) = self.active_mission.lock().await.clone() {
            if active.id == mission_id {
                return Ok(active);
            }
        }

        let conn = Connection::open(&self.db_path)
            .map_err(|error| CoordinatorError::Storage(error.to_string()))?;
        load_mission(&conn, mission_id)?
            .ok_or_else(|| CoordinatorError::MissionNotFound(mission_id.to_string()))
    }

    pub async fn activate_mission(&self, mission_id: &str) -> Result<Mission, CoordinatorError> {
        let conn = Connection::open(&self.db_path)
            .map_err(|error| CoordinatorError::Storage(error.to_string()))?;
        let mission = load_mission(&conn, mission_id)?
            .ok_or_else(|| CoordinatorError::MissionNotFound(mission_id.to_string()))?;
        *self.active_mission.lock().await = Some(mission.clone());
        Ok(mission)
    }

    pub async fn replace_mission_dag(
        &self,
        mission_id: &str,
        dag_json: serde_json::Value,
    ) -> Result<Mission, CoordinatorError> {
        let dag = self.planner.plan_manual(dag_json)?;
        let mut guard = self.active_mission.lock().await;
        let mission = ensure_active_mission_mut(&mut guard, mission_id)?;
        mission.dag = dag;
        self.save_mission(mission)?;
        Ok(mission.clone())
    }

    pub fn get_mission_history(&self) -> Result<Vec<MissionSummary>, CoordinatorError> {
        let conn = Connection::open(&self.db_path)
            .map_err(|error| CoordinatorError::Storage(error.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, title, mode, status, dag_json, total_cost, total_elapsed_ms, created_at
                 FROM missions
                 ORDER BY created_at DESC
                 LIMIT 20",
            )
            .map_err(|error| CoordinatorError::Storage(error.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let dag_json: String = row.get(4)?;
                let dag: TaskDAG = serde_json::from_str(&dag_json).unwrap_or_default();
                let completed_count = dag
                    .nodes
                    .values()
                    .filter(|node| node.status == SubtaskStatus::Completed)
                    .count() as u32;
                Ok(MissionSummary {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    mode: parse_mode(&row.get::<_, String>(2)?),
                    status: parse_mission_status(&row.get::<_, String>(3)?),
                    subtask_count: dag.nodes.len() as u32,
                    completed_count,
                    total_cost: row.get(5)?,
                    total_elapsed_ms: row.get::<_, i64>(6)? as u64,
                    created_at: parse_datetime(&row.get::<_, String>(7)?),
                })
            })
            .map_err(|error| CoordinatorError::Storage(error.to_string()))?;

        Ok(rows.filter_map(Result::ok).collect())
    }

    pub fn get_available_specialists(
        &self,
    ) -> Vec<crate::coordinator::specialists::SpecialistProfile> {
        self.specialists.list()
    }

    pub fn get_available_tools(&self) -> Vec<crate::tools::ToolDefinition> {
        self.tool_registry.definitions()
    }

    fn save_mission(&self, mission: &Mission) -> Result<(), CoordinatorError> {
        let conn = Connection::open(&self.db_path)
            .map_err(|error| CoordinatorError::Storage(error.to_string()))?;
        let dag_json = serde_json::to_string(&mission.dag)
            .map_err(|error| CoordinatorError::Storage(error.to_string()))?;

        conn.execute(
            "INSERT INTO missions (
                id, title, description, mode, autonomy, status, dag_json,
                total_cost, total_tokens, total_elapsed_ms, created_at, started_at, completed_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                description = excluded.description,
                mode = excluded.mode,
                autonomy = excluded.autonomy,
                status = excluded.status,
                dag_json = excluded.dag_json,
                total_cost = excluded.total_cost,
                total_tokens = excluded.total_tokens,
                total_elapsed_ms = excluded.total_elapsed_ms,
                created_at = excluded.created_at,
                started_at = excluded.started_at,
                completed_at = excluded.completed_at",
            params![
                mission.id,
                mission.title,
                mission.description,
                mode_to_str(&mission.mode),
                autonomy_to_str(&mission.autonomy),
                status_to_str(&mission.status),
                dag_json,
                mission.total_cost,
                mission.total_tokens as i64,
                mission.total_elapsed_ms as i64,
                mission.created_at.to_rfc3339(),
                mission.started_at.as_ref().map(|date| date.to_rfc3339()),
                mission.completed_at.as_ref().map(|date| date.to_rfc3339()),
            ],
        )
        .map_err(|error| CoordinatorError::Storage(error.to_string()))?;

        Ok(())
    }

    fn append_log(
        &self,
        mission_id: &str,
        event_type: &str,
        agent_name: &str,
        agent_level: &str,
        subtask_id: Option<&str>,
        message: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<(), CoordinatorError> {
        let conn = Connection::open(&self.db_path)
            .map_err(|error| CoordinatorError::Storage(error.to_string()))?;
        conn.execute(
            "INSERT INTO mission_log (id, mission_id, timestamp, event_type, agent_name, agent_level, subtask_id, message, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                uuid::Uuid::new_v4().to_string(),
                mission_id,
                Utc::now().to_rfc3339(),
                event_type,
                agent_name,
                agent_level,
                subtask_id,
                message,
                metadata.map(|value| value.to_string()),
            ],
        )
        .map_err(|error| CoordinatorError::Storage(error.to_string()))?;
        Ok(())
    }
}

fn extract_title(description: &str) -> String {
    let trimmed = description.trim();
    let first_line = trimmed.lines().next().unwrap_or(trimmed).trim();
    let mut title = first_line.chars().take(64).collect::<String>();
    if title.is_empty() {
        title = "Untitled mission".to_string();
    }
    title
}

fn derive_commander_title(dag: &TaskDAG) -> String {
    dag.nodes
        .values()
        .next()
        .map(|node| format!("Commander: {}", node.title))
        .unwrap_or_else(|| "Commander mission".to_string())
}

fn format_template_title(template_id: &str, context: &str) -> String {
    let template_name = match template_id {
        "market_research" => "Market Research",
        "code_review" => "Code Review + Tests",
        "content_pipeline" => "Content Pipeline",
        "due_diligence" => "Due Diligence",
        "email_campaign" => "Email Campaign",
        "design_sprint" => "Design Sprint",
        _ => "Template Mission",
    };

    if context.trim().is_empty() {
        template_name.to_string()
    } else {
        format!("{}: {}", template_name, context.trim())
    }
}

fn ensure_active_mission_mut<'a>(
    guard: &'a mut Option<Mission>,
    mission_id: &str,
) -> Result<&'a mut Mission, CoordinatorError> {
    let mission = guard.as_mut().ok_or(CoordinatorError::NoActiveMission)?;
    if mission.id != mission_id {
        return Err(CoordinatorError::MissionNotFound(mission_id.to_string()));
    }
    Ok(mission)
}

fn ensure_active_mission_ref<'a>(
    guard: &'a Option<Mission>,
    mission_id: &str,
) -> Result<&'a Mission, CoordinatorError> {
    let mission = guard.as_ref().ok_or(CoordinatorError::NoActiveMission)?;
    if mission.id != mission_id {
        return Err(CoordinatorError::MissionNotFound(mission_id.to_string()));
    }
    Ok(mission)
}

fn load_mission(conn: &Connection, mission_id: &str) -> Result<Option<Mission>, CoordinatorError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, title, description, mode, autonomy, status, dag_json, total_cost, total_tokens, total_elapsed_ms, created_at, started_at, completed_at
             FROM missions WHERE id = ?1",
        )
        .map_err(|error| CoordinatorError::Storage(error.to_string()))?;

    let result = stmt.query_row(params![mission_id], |row| {
        let dag_json: String = row.get(6)?;
        let dag: TaskDAG = serde_json::from_str(&dag_json).unwrap_or_default();
        Ok(Mission {
            id: row.get(0)?,
            title: row.get(1)?,
            description: row.get(2)?,
            mode: parse_mode(&row.get::<_, String>(3)?),
            autonomy: parse_autonomy(&row.get::<_, String>(4)?),
            status: parse_mission_status(&row.get::<_, String>(5)?),
            dag,
            total_cost: row.get(7)?,
            total_tokens: row.get::<_, i64>(8)? as u64,
            total_elapsed_ms: row.get::<_, i64>(9)? as u64,
            created_at: parse_datetime(&row.get::<_, String>(10)?),
            started_at: row
                .get::<_, Option<String>>(11)?
                .map(|value| parse_datetime(&value)),
            completed_at: row
                .get::<_, Option<String>>(12)?
                .map(|value| parse_datetime(&value)),
        })
    });

    match result {
        Ok(mission) => Ok(Some(mission)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(CoordinatorError::Storage(error.to_string())),
    }
}

fn parse_datetime(value: &str) -> chrono::DateTime<Utc> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn parse_mode(value: &str) -> CoordinatorMode {
    match value {
        "commander" => CoordinatorMode::Commander,
        _ => CoordinatorMode::Autopilot,
    }
}

fn parse_autonomy(value: &str) -> AutonomyLevel {
    match value {
        "full" => AutonomyLevel::Full,
        "ask_always" => AutonomyLevel::AskAlways,
        _ => AutonomyLevel::AskOnError,
    }
}

fn parse_mission_status(value: &str) -> MissionStatus {
    match value {
        "ready" => MissionStatus::Ready,
        "running" => MissionStatus::Running,
        "paused" => MissionStatus::Paused,
        "completed" => MissionStatus::Completed,
        "failed" => MissionStatus::Failed,
        "cancelled" => MissionStatus::Cancelled,
        _ => MissionStatus::Planning,
    }
}

fn mode_to_str(mode: &CoordinatorMode) -> &'static str {
    match mode {
        CoordinatorMode::Autopilot => "autopilot",
        CoordinatorMode::Commander => "commander",
    }
}

fn autonomy_to_str(autonomy: &AutonomyLevel) -> &'static str {
    match autonomy {
        AutonomyLevel::Full => "full",
        AutonomyLevel::AskOnError => "ask_on_error",
        AutonomyLevel::AskAlways => "ask_always",
    }
}

fn status_to_str(status: &MissionStatus) -> &'static str {
    match status {
        MissionStatus::Planning => "planning",
        MissionStatus::Ready => "ready",
        MissionStatus::Running => "running",
        MissionStatus::Paused => "paused",
        MissionStatus::Completed => "completed",
        MissionStatus::Failed => "failed",
        MissionStatus::Cancelled => "cancelled",
    }
}

fn parse_mode_arg(mode: &str) -> CoordinatorMode {
    match mode.trim().to_ascii_lowercase().as_str() {
        "commander" => CoordinatorMode::Commander,
        _ => CoordinatorMode::Autopilot,
    }
}

fn parse_autonomy_arg(autonomy: &str) -> AutonomyLevel {
    match autonomy.trim().to_ascii_lowercase().as_str() {
        "full" => AutonomyLevel::Full,
        "ask_always" => AutonomyLevel::AskAlways,
        _ => AutonomyLevel::AskOnError,
    }
}

fn parse_edge_type_arg(value: &str) -> EdgeType {
    match value.trim().to_ascii_lowercase().as_str() {
        "conditional" => EdgeType::Conditional,
        "dependency" => EdgeType::Dependency,
        _ => EdgeType::DataFlow,
    }
}

#[tauri::command]
pub async fn cmd_create_mission(
    state: State<'_, AppState>,
    description: String,
    mode: String,
    autonomy: String,
) -> Result<serde_json::Value, String> {
    let runtime = state.coordinator_runtime.clone();
    let settings = state
        .settings
        .lock()
        .map_err(|error| error.to_string())?
        .clone();

    let mission = match parse_mode_arg(&mode) {
        CoordinatorMode::Commander => {
            return Err("Use cmd_create_mission_manual for commander missions".to_string())
        }
        CoordinatorMode::Autopilot => {
            runtime
                .create_autopilot_mission(&description, parse_autonomy_arg(&autonomy), &settings)
                .await
        }
    }
    .map_err(|error| error.to_string())?;

    serde_json::to_value(mission).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cmd_create_mission_manual(
    state: State<'_, AppState>,
    dag_json: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mission = state
        .coordinator_runtime
        .create_commander_mission(dag_json)
        .await
        .map_err(|error| error.to_string())?;
    serde_json::to_value(mission).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cmd_create_mission_from_template(
    state: State<'_, AppState>,
    template_id: String,
    context: String,
) -> Result<serde_json::Value, String> {
    let mission = state
        .coordinator_runtime
        .create_template_mission(&template_id, &context)
        .await
        .map_err(|error| error.to_string())?;
    serde_json::to_value(mission).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cmd_start_mission(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
    mission_id: String,
) -> Result<(), String> {
    let runtime = state.coordinator_runtime.clone();
    let settings = state
        .settings
        .lock()
        .map_err(|error| error.to_string())?
        .clone();

    tauri::async_runtime::spawn(async move {
        let _ = runtime
            .start_mission(&mission_id, settings, app_handle)
            .await;
    });
    Ok(())
}

#[tauri::command]
pub async fn cmd_pause_mission(
    state: State<'_, AppState>,
    mission_id: String,
) -> Result<(), String> {
    state
        .coordinator_runtime
        .pause_mission(&mission_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cmd_cancel_mission(
    state: State<'_, AppState>,
    mission_id: String,
) -> Result<(), String> {
    state
        .coordinator_runtime
        .cancel_mission(&mission_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cmd_retry_subtask(
    state: State<'_, AppState>,
    mission_id: String,
    subtask_id: String,
) -> Result<(), String> {
    state
        .coordinator_runtime
        .retry_subtask(&mission_id, &subtask_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cmd_add_subtask(
    state: State<'_, AppState>,
    mission_id: String,
    subtask: serde_json::Value,
) -> Result<String, String> {
    state
        .coordinator_runtime
        .add_subtask(&mission_id, subtask)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cmd_remove_subtask(
    state: State<'_, AppState>,
    mission_id: String,
    subtask_id: String,
) -> Result<(), String> {
    state
        .coordinator_runtime
        .remove_subtask(&mission_id, &subtask_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cmd_connect_subtasks(
    state: State<'_, AppState>,
    mission_id: String,
    from_id: String,
    to_id: String,
    edge_type: String,
) -> Result<(), String> {
    state
        .coordinator_runtime
        .connect_subtasks(
            &mission_id,
            &from_id,
            &to_id,
            parse_edge_type_arg(&edge_type),
        )
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cmd_disconnect_subtasks(
    state: State<'_, AppState>,
    mission_id: String,
    from_id: String,
    to_id: String,
) -> Result<(), String> {
    state
        .coordinator_runtime
        .disconnect_subtasks(&mission_id, &from_id, &to_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cmd_assign_agent(
    state: State<'_, AppState>,
    mission_id: String,
    subtask_id: String,
    assignment: serde_json::Value,
) -> Result<(), String> {
    let assignment: AgentAssignment =
        serde_json::from_value(assignment).map_err(|error| error.to_string())?;
    state
        .coordinator_runtime
        .assign_agent(&mission_id, &subtask_id, assignment)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cmd_update_subtask_position(
    state: State<'_, AppState>,
    mission_id: String,
    subtask_id: String,
    x: f32,
    y: f32,
) -> Result<(), String> {
    state
        .coordinator_runtime
        .update_position(&mission_id, &subtask_id, x, y)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cmd_update_subtask(
    state: State<'_, AppState>,
    mission_id: String,
    subtask_id: String,
    patch: serde_json::Value,
) -> Result<(), String> {
    state
        .coordinator_runtime
        .update_subtask(&mission_id, &subtask_id, patch)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cmd_inject_mission_message(
    state: State<'_, AppState>,
    mission_id: String,
    message: String,
) -> Result<(), String> {
    state
        .coordinator_runtime
        .inject_message(&mission_id, &message)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cmd_approve_step(
    state: State<'_, AppState>,
    mission_id: String,
    subtask_id: String,
    approved: bool,
) -> Result<(), String> {
    state
        .coordinator_runtime
        .approve_step(&mission_id, &subtask_id, approved)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cmd_get_mission(
    state: State<'_, AppState>,
    mission_id: String,
) -> Result<serde_json::Value, String> {
    let mission = state
        .coordinator_runtime
        .get_mission(&mission_id)
        .await
        .map_err(|error| error.to_string())?;
    serde_json::to_value(mission).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cmd_activate_mission(
    state: State<'_, AppState>,
    mission_id: String,
) -> Result<serde_json::Value, String> {
    let mission = state
        .coordinator_runtime
        .activate_mission(&mission_id)
        .await
        .map_err(|error| error.to_string())?;
    serde_json::to_value(mission).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cmd_replace_mission_dag(
    state: State<'_, AppState>,
    mission_id: String,
    dag_json: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mission = state
        .coordinator_runtime
        .replace_mission_dag(&mission_id, dag_json)
        .await
        .map_err(|error| error.to_string())?;
    serde_json::to_value(mission).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cmd_get_mission_history(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    serde_json::to_value(
        state
            .coordinator_runtime
            .get_mission_history()
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cmd_get_available_specialists(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    serde_json::to_value(state.coordinator_runtime.get_available_specialists())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cmd_get_available_tools(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    serde_json::to_value(state.coordinator_runtime.get_available_tools())
        .map_err(|error| error.to_string())
}
