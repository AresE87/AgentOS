use crate::config::Settings;
use crate::coordinator::event_bus::{CoordinatorEvent, EventBus};
use crate::coordinator::pool::{AgentPool, PoolError, SubtaskExecutionResult};
use crate::coordinator::types::*;
use chrono::Utc;
use std::collections::HashSet;
use std::sync::Arc;
use thiserror::Error;
use tokio::task::JoinSet;

pub struct TaskScheduler {
    event_bus: Arc<EventBus>,
}

#[derive(Debug, Error)]
pub enum SchedulerError {
    #[error("No active mission to execute")]
    NoMission,
    #[error("Mission id mismatch; expected '{expected}', got '{actual}'")]
    MissionMismatch { expected: String, actual: String },
    #[error("Pool error: {0}")]
    Pool(#[from] PoolError),
    #[error("Task join error: {0}")]
    Join(String),
}

// v7 INTEGRATION POINT: select_target()
// When v7 lands, the scheduler will call this before spawning each worker:
//
// fn select_target(&self, node: &DAGNode, available_nodes: &[MeshNode]) -> ExecutionTarget {
//     if sandbox::SandboxManager::is_docker_available().await {
//         let cid = sandbox::WorkerContainer::start(&worker_id, workspace, port).await?;
//         ExecutionTarget::DockerLocal { container_id: cid }
//     } else if let Some(remote) = find_best_remote(available_nodes) {
//         ExecutionTarget::DockerRemote { node_id: remote.id, container_id: "".into() }
//     } else {
//         ExecutionTarget::Local
//     }
// }

impl TaskScheduler {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self { event_bus }
    }

    pub async fn execute(
        &self,
        mission_state: Arc<tokio::sync::Mutex<Option<Mission>>>,
        mission_id: &str,
        pool: Arc<AgentPool>,
        settings: Settings,
        app_handle: tauri::AppHandle,
    ) -> Result<(), SchedulerError> {
        {
            let mut guard = mission_state.lock().await;
            let mission = guard.as_mut().ok_or(SchedulerError::NoMission)?;
            if mission.id != mission_id {
                return Err(SchedulerError::MissionMismatch {
                    expected: mission.id.clone(),
                    actual: mission_id.to_string(),
                });
            }

            mission.status = MissionStatus::Running;
            mission.started_at = Some(Utc::now());
        }

        self.event_bus.emit(CoordinatorEvent::MissionStarted {
            mission_id: mission_id.to_string(),
        });

        let started = std::time::Instant::now();
        let mut join_set = JoinSet::<(String, Result<SubtaskExecutionResult, PoolError>)>::new();
        let mut running = HashSet::<String>::new();

        loop {
            {
                let mission_status = mission_state
                    .lock()
                    .await
                    .as_ref()
                    .map(|mission| mission.status.clone())
                    .ok_or(SchedulerError::NoMission)?;

                match mission_status {
                    MissionStatus::Paused => {
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        continue;
                    }
                    MissionStatus::Cancelled => {
                        self.event_bus.emit(CoordinatorEvent::MissionCancelled {
                            mission_id: mission_id.to_string(),
                        });
                        break;
                    }
                    _ => {}
                }
            }

            let ready_nodes = self
                .prepare_ready_nodes(&mission_state, mission_id, &running)
                .await?;

            for node_id in ready_nodes {
                let (node, context) = {
                    let guard = mission_state.lock().await;
                    let mission = guard.as_ref().ok_or(SchedulerError::NoMission)?;
                    let node = mission
                        .dag
                        .nodes
                        .get(&node_id)
                        .ok_or_else(|| SchedulerError::Join(format!("Missing node '{}'", node_id)))?
                        .clone();
                    let context = mission.dag.gather_inputs(&node_id);
                    (node, context)
                };

                let worker_id = pool.spawn_worker(&node).await?;
                let pool_ref = pool.clone();
                let event_bus = self.event_bus.clone();
                let settings_clone = settings.clone();
                let handle = app_handle.clone();
                let mid = mission_id.to_string();
                let nid = node_id.clone();
                running.insert(node_id.clone());

                join_set.spawn(async move {
                    let result = pool_ref
                        .execute_subtask(
                            &mid,
                            &worker_id,
                            &node,
                            context,
                            &event_bus,
                            &handle,
                            &settings_clone,
                        )
                        .await;
                    (nid, result)
                });
            }

            {
                let mut guard = mission_state.lock().await;
                let mission = guard.as_mut().ok_or(SchedulerError::NoMission)?;

                if mission.dag.is_complete() && running.is_empty() {
                    mission.status = MissionStatus::Completed;
                    mission.completed_at = Some(Utc::now());
                    mission.total_elapsed_ms = started.elapsed().as_millis() as u64;

                    self.event_bus.emit(CoordinatorEvent::MissionCompleted {
                        mission_id: mission.id.clone(),
                        total_cost: mission.total_cost,
                        total_elapsed_ms: mission.total_elapsed_ms,
                    });
                    break;
                }

                if running.is_empty() && mission.dag.has_fatal_failure() {
                    let retriable = self.find_retriable_nodes(&mission.dag);
                    if retriable.is_empty() {
                        mission.status = MissionStatus::Failed;
                        self.event_bus.emit(CoordinatorEvent::MissionFailed {
                            mission_id: mission.id.clone(),
                            error: "One or more subtasks failed without recovery".to_string(),
                        });
                        break;
                    }

                    for node_id in retriable {
                        if let Some(node) = mission.dag.nodes.get_mut(&node_id) {
                            if mission.autonomy == AutonomyLevel::Full {
                                node.status = SubtaskStatus::Retrying;
                                node.retry_count += 1;
                                node.error = None;
                                node.progress = 0.0;
                                node.awaiting_approval = false;
                                node.approved_to_run = false;

                                self.event_bus.emit(CoordinatorEvent::SubtaskRetrying {
                                    mission_id: mission.id.clone(),
                                    subtask_id: node_id.clone(),
                                    attempt: node.retry_count,
                                });

                                node.status = SubtaskStatus::Queued;
                            } else if !node.awaiting_approval {
                                node.status = SubtaskStatus::Paused;
                                node.awaiting_approval = true;
                                node.approved_to_run = false;
                                node.last_message =
                                    Some("Retry awaiting approval".to_string());

                                self.event_bus.emit(CoordinatorEvent::ApprovalRequested {
                                    mission_id: mission.id.clone(),
                                    subtask_id: node_id.clone(),
                                    question: format!(
                                        "Retry '{}' after the previous failure?",
                                        node.title
                                    ),
                                });
                            }
                        }
                    }
                    continue;
                }
            }

            if running.is_empty() {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                continue;
            }

            tokio::select! {
                maybe_joined = join_set.join_next() => {
                    match maybe_joined {
                        Some(Ok((node_id, result))) => {
                            running.remove(&node_id);
                            self.apply_result(
                                &mission_state,
                                mission_id,
                                &node_id,
                                result,
                                started.elapsed().as_millis() as u64,
                            ).await?;
                        }
                        Some(Err(error)) => {
                            return Err(SchedulerError::Join(error.to_string()));
                        }
                        None => {
                            tokio::time::sleep(tokio::time::Duration::from_millis(25)).await;
                        }
                    }
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {}
            }
        }

        Ok(())
    }

    async fn prepare_ready_nodes(
        &self,
        mission_state: &Arc<tokio::sync::Mutex<Option<Mission>>>,
        mission_id: &str,
        running: &HashSet<String>,
    ) -> Result<Vec<String>, SchedulerError> {
        let mut guard = mission_state.lock().await;
        let mission = guard.as_mut().ok_or(SchedulerError::NoMission)?;
        if mission.id != mission_id {
            return Err(SchedulerError::MissionMismatch {
                expected: mission.id.clone(),
                actual: mission_id.to_string(),
            });
        }

        let ready_candidates = mission
            .dag
            .ready_nodes()
            .into_iter()
            .filter(|node_id| !running.contains(node_id))
            .collect::<Vec<_>>();
        let mut ready = Vec::new();

        for node_id in ready_candidates {
            if let Some(node) = mission.dag.nodes.get_mut(&node_id) {
                if mission.autonomy == AutonomyLevel::AskAlways && !node.approved_to_run {
                    if !node.awaiting_approval {
                        node.awaiting_approval = true;
                        node.status = SubtaskStatus::Paused;
                        node.last_message = Some("Awaiting commander approval".to_string());

                        self.event_bus.emit(CoordinatorEvent::ApprovalRequested {
                            mission_id: mission.id.clone(),
                            subtask_id: node_id.clone(),
                            question: format!(
                                "Execute '{}' with {}?",
                                node.title,
                                node.assignment
                                    .specialist_name
                                    .clone()
                                    .unwrap_or_else(|| format!("{:?}", node.assignment.level))
                            ),
                        });
                    }
                    continue;
                }

                node.awaiting_approval = false;
                node.approved_to_run = false;
                node.status = SubtaskStatus::Running;
                node.started_at = Some(Utc::now());
                node.progress = 0.05;

                let agent_name = node
                    .assignment
                    .specialist_name
                    .clone()
                    .unwrap_or_else(|| format!("{:?}", node.assignment.level));

                self.event_bus.emit(CoordinatorEvent::SubtaskStarted {
                    mission_id: mission.id.clone(),
                    subtask_id: node_id.clone(),
                    agent_name,
                    agent_level: format!("{:?}", node.assignment.level).to_lowercase(),
                });

                ready.push(node_id);
            }
        }

        Ok(ready)
    }

    async fn apply_result(
        &self,
        mission_state: &Arc<tokio::sync::Mutex<Option<Mission>>>,
        mission_id: &str,
        node_id: &str,
        execution_result: Result<SubtaskExecutionResult, PoolError>,
        elapsed_ms: u64,
    ) -> Result<(), SchedulerError> {
        let mut guard = mission_state.lock().await;
        let mission = guard.as_mut().ok_or(SchedulerError::NoMission)?;
        let node = mission
            .dag
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| SchedulerError::Join(format!("Missing node '{}'", node_id)))?;

        match execution_result {
            Ok(output) => {
                node.status = SubtaskStatus::Completed;
                node.result = Some(output.output.clone());
                node.completed_at = Some(Utc::now());
                node.progress = 1.0;
                node.last_message = output.last_message;
                node.cost = output.cost;
                node.tokens_in = output.tokens_in;
                node.tokens_out = output.tokens_out;
                node.elapsed_ms = output.elapsed_ms;
                node.awaiting_approval = false;
                node.approved_to_run = false;

                mission.total_cost += output.cost;
                mission.total_tokens += output.tokens_in + output.tokens_out;
                mission.total_elapsed_ms = elapsed_ms;

                self.event_bus.emit(CoordinatorEvent::SubtaskCompleted {
                    mission_id: mission.id.clone(),
                    subtask_id: node_id.to_string(),
                    cost: output.cost,
                    tokens: output.tokens_in + output.tokens_out,
                    elapsed_ms: output.elapsed_ms,
                });
            }
            Err(error) => {
                node.status = if mission.autonomy == AutonomyLevel::Full {
                    SubtaskStatus::Failed
                } else {
                    SubtaskStatus::Paused
                };
                node.error = Some(error.to_string());
                node.progress = 1.0;
                node.awaiting_approval = mission.autonomy != AutonomyLevel::Full;
                node.approved_to_run = false;
                node.last_message = Some(error.to_string());
                mission.total_elapsed_ms = elapsed_ms;

                self.event_bus.emit(CoordinatorEvent::SubtaskFailed {
                    mission_id: mission.id.clone(),
                    subtask_id: node_id.to_string(),
                    error: error.to_string(),
                });

                if mission.autonomy != AutonomyLevel::Full {
                    self.event_bus.emit(CoordinatorEvent::ApprovalRequested {
                        mission_id: mission_id.to_string(),
                        subtask_id: node_id.to_string(),
                        question: format!(
                            "Task '{}' failed with: {}. Retry it?",
                            node.title, error
                        ),
                    });
                }
            }
        }

        let completed = mission
            .dag
            .nodes
            .values()
            .filter(|node| node.status == SubtaskStatus::Completed)
            .count() as u32;
        let total = mission.dag.nodes.len() as u32;

        self.event_bus.emit(CoordinatorEvent::MissionProgress {
            mission_id: mission.id.clone(),
            completed,
            total,
            cost: mission.total_cost,
            elapsed_ms: mission.total_elapsed_ms,
        });

        Ok(())
    }

    fn find_retriable_nodes(&self, dag: &TaskDAG) -> Vec<String> {
        dag.nodes
            .iter()
            .filter(|(_, node)| {
                node.status == SubtaskStatus::Failed && node.retry_count < node.max_retries
            })
            .map(|(id, _)| id.clone())
            .collect()
    }
}
