use crate::agents::AgentRegistry;
use crate::brain::Gateway;
use crate::config::Settings;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, Semaphore};
use tokio::task::JoinSet;
use tokio::time::{timeout, Duration};

type RunnerFuture =
    Pin<Box<dyn Future<Output = Result<SwarmExecutionOutput, String>> + Send + 'static>>;
type SubtaskRunner = Arc<dyn Fn(SwarmSubtask, Settings) -> RunnerFuture + Send + Sync + 'static>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmSubtask {
    pub id: String,
    pub title: String,
    pub description: String,
    pub agent_name: String,
    pub status: String,
    pub result: Option<String>,
    pub error: Option<String>,
    pub source_hint: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub duration_ms: Option<u64>,
}

/// Result from an individual agent in the swarm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmResult {
    pub subtask_id: String,
    pub agent_name: String,
    pub output: String,
    pub confidence: f64,
    pub duration_ms: u64,
    pub status: String,
    pub error: Option<String>,
}

/// A task distributed across multiple agents in the swarm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmTask {
    pub id: String,
    pub description: String,
    pub assigned_agents: Vec<String>,
    /// Strategy: "parallel", "sequential", or "vote"
    pub strategy: String,
    pub status: String,
    pub subtasks: Vec<SwarmSubtask>,
    pub results: Vec<SwarmResult>,
    pub aggregated_result: Option<String>,
    pub errors: Vec<String>,
    pub max_concurrency: usize,
    pub timeout_ms: u64,
    pub cancel_requested: bool,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct SwarmExecutionPlan {
    task_id: String,
    strategy: String,
    max_concurrency: usize,
    timeout_ms: u64,
    subtasks: Vec<SwarmSubtask>,
}

#[derive(Debug, Clone)]
struct SwarmExecutionOutput {
    output: String,
    confidence: f64,
}

/// Coordinates a swarm of agents for parallel/sequential/vote-based execution.
pub struct SwarmCoordinator {
    tasks: HashMap<String, SwarmTask>,
}

impl SwarmCoordinator {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
        }
    }

    /// Create a new swarm task.
    pub fn create_swarm_task(
        &mut self,
        description: &str,
        agents: Vec<String>,
        strategy: &str,
        max_concurrency: usize,
        timeout_ms: u64,
    ) -> Result<SwarmTask, String> {
        if agents.is_empty() {
            return Err("Swarm requires at least one assigned agent.".to_string());
        }

        let strategy = normalize_strategy(strategy);
        let id = format!("swarm-{}", uuid::Uuid::new_v4());
        let subtasks = derive_subtasks(description, &agents);
        let task = SwarmTask {
            id: id.clone(),
            description: description.to_string(),
            assigned_agents: agents,
            strategy: strategy.to_string(),
            status: "pending".to_string(),
            subtasks,
            results: Vec::new(),
            aggregated_result: None,
            errors: Vec::new(),
            max_concurrency: max_concurrency.max(1),
            timeout_ms: timeout_ms.max(1),
            cancel_requested: false,
            created_at: now_rfc3339(),
            started_at: None,
            finished_at: None,
        };
        self.tasks.insert(id, task.clone());
        Ok(task)
    }

    pub(crate) fn start_execution(&mut self, task_id: &str) -> Result<SwarmExecutionPlan, String> {
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| format!("Swarm task '{}' not found", task_id))?;

        if task.status != "pending" {
            return Err(format!(
                "Task '{}' cannot start because it is currently '{}'",
                task_id, task.status
            ));
        }

        task.status = "running".to_string();
        task.started_at = Some(now_rfc3339());
        task.finished_at = None;
        task.cancel_requested = false;
        task.results.clear();
        task.errors.clear();
        task.aggregated_result = None;

        for subtask in &mut task.subtasks {
            subtask.status = "pending".to_string();
            subtask.result = None;
            subtask.error = None;
            subtask.started_at = None;
            subtask.finished_at = None;
            subtask.duration_ms = None;
        }

        Ok(SwarmExecutionPlan {
            task_id: task.id.clone(),
            strategy: task.strategy.clone(),
            max_concurrency: task.max_concurrency,
            timeout_ms: task.timeout_ms,
            subtasks: task.subtasks.clone(),
        })
    }

    pub fn mark_subtask_running(&mut self, task_id: &str, subtask_id: &str) -> Result<(), String> {
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| format!("Swarm task '{}' not found", task_id))?;
        let subtask = task
            .subtasks
            .iter_mut()
            .find(|item| item.id == subtask_id)
            .ok_or_else(|| format!("Swarm subtask '{}' not found", subtask_id))?;

        subtask.status = "running".to_string();
        subtask.started_at = Some(now_rfc3339());
        Ok(())
    }

    fn execution_plan(&self, task_id: &str) -> Result<SwarmExecutionPlan, String> {
        let task = self
            .tasks
            .get(task_id)
            .ok_or_else(|| format!("Swarm task '{}' not found", task_id))?;
        if task.status != "running" {
            return Err(format!(
                "Task '{}' cannot execute because it is currently '{}'",
                task_id, task.status
            ));
        }

        Ok(SwarmExecutionPlan {
            task_id: task.id.clone(),
            strategy: task.strategy.clone(),
            max_concurrency: task.max_concurrency,
            timeout_ms: task.timeout_ms,
            subtasks: task.subtasks.clone(),
        })
    }

    pub fn complete_subtask(&mut self, task_id: &str, result: SwarmResult) -> Result<(), String> {
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| format!("Swarm task '{}' not found", task_id))?;
        let subtask = task
            .subtasks
            .iter_mut()
            .find(|item| item.id == result.subtask_id)
            .ok_or_else(|| format!("Swarm subtask '{}' not found", result.subtask_id))?;

        subtask.status = result.status.clone();
        subtask.result = Some(result.output.clone());
        subtask.error = result.error.clone();
        subtask.duration_ms = Some(result.duration_ms);
        subtask.finished_at = Some(now_rfc3339());
        task.results.push(result);
        Ok(())
    }

    pub fn request_cancel(&mut self, task_id: &str) -> Result<SwarmTask, String> {
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| format!("Swarm task '{}' not found", task_id))?;

        task.cancel_requested = true;

        if task.status == "pending" {
            task.status = "cancelled".to_string();
            task.finished_at = Some(now_rfc3339());
            for subtask in &mut task.subtasks {
                if subtask.status == "pending" {
                    subtask.status = "cancelled".to_string();
                    subtask.error = Some("Cancelled before execution started.".to_string());
                    subtask.finished_at = Some(now_rfc3339());
                }
            }
        } else {
            for subtask in &mut task.subtasks {
                if subtask.status == "pending" {
                    subtask.status = "cancelled".to_string();
                    subtask.error = Some("Cancelled before execution started.".to_string());
                    subtask.finished_at = Some(now_rfc3339());
                }
            }
        }

        Ok(task.clone())
    }

    pub fn finalize_task(&mut self, task_id: &str) -> Result<SwarmTask, String> {
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| format!("Swarm task '{}' not found", task_id))?;

        let successful: Vec<SwarmResult> = task
            .results
            .iter()
            .filter(|result| result.status == "completed")
            .cloned()
            .collect();
        let failed = task
            .results
            .iter()
            .filter(|result| result.status != "completed")
            .count();

        task.aggregated_result = aggregate_results(&task.description, &task.strategy, &successful);
        task.errors = task
            .results
            .iter()
            .filter_map(|result| {
                result
                    .error
                    .as_ref()
                    .map(|err| format!("{}: {}", result.agent_name, err))
            })
            .collect();
        task.finished_at = Some(now_rfc3339());
        task.status = if task.cancel_requested {
            "cancelled".to_string()
        } else if successful.is_empty() && failed > 0 {
            "failed".to_string()
        } else if failed > 0 {
            "completed_with_errors".to_string()
        } else {
            "completed".to_string()
        };

        Ok(task.clone())
    }

    /// Get results for a swarm task.
    pub fn get_results(&self, task_id: &str) -> Result<SwarmTask, String> {
        self.tasks
            .get(task_id)
            .cloned()
            .ok_or_else(|| format!("Swarm task '{}' not found", task_id))
    }

    pub fn is_cancel_requested(&self, task_id: &str) -> Result<bool, String> {
        self.tasks
            .get(task_id)
            .map(|task| task.cancel_requested)
            .ok_or_else(|| format!("Swarm task '{}' not found", task_id))
    }

    /// Vote-based consensus: return the result with highest confidence.
    pub fn vote_consensus(results: &[SwarmResult]) -> Option<SwarmResult> {
        results
            .iter()
            .filter(|result| result.status == "completed")
            .max_by(|a, b| {
                a.confidence
                    .partial_cmp(&b.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }

    /// List all swarm tasks.
    pub fn list_tasks(&self) -> Vec<SwarmTask> {
        let mut tasks: Vec<SwarmTask> = self.tasks.values().cloned().collect();
        tasks.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        tasks
    }
}

pub async fn execute_swarm_task(
    coordinator: Arc<Mutex<SwarmCoordinator>>,
    task_id: String,
    settings: Settings,
) -> Result<(), String> {
    let plan = {
        let mut coordinator = coordinator.lock().await;
        coordinator.start_execution(&task_id)?
    };
    execute_swarm_task_inner(coordinator, plan, settings, production_runner()).await
}

pub async fn execute_started_swarm_task(
    coordinator: Arc<Mutex<SwarmCoordinator>>,
    task_id: String,
    settings: Settings,
) -> Result<(), String> {
    let plan = {
        let coordinator = coordinator.lock().await;
        coordinator.execution_plan(&task_id)?
    };
    execute_swarm_task_inner(coordinator, plan, settings, production_runner()).await
}

async fn execute_swarm_task_inner(
    coordinator: Arc<Mutex<SwarmCoordinator>>,
    plan: SwarmExecutionPlan,
    settings: Settings,
    runner: SubtaskRunner,
) -> Result<(), String> {
    let max_concurrency = match normalize_strategy(&plan.strategy) {
        "sequential" => 1,
        _ => plan.max_concurrency.max(1),
    };
    let semaphore = Arc::new(Semaphore::new(max_concurrency));
    let mut join_set = JoinSet::new();

    for subtask in plan.subtasks.clone() {
        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| "Failed to acquire swarm semaphore permit.".to_string())?;
        let coordinator = coordinator.clone();
        let runner = runner.clone();
        let task_id = plan.task_id.clone();
        let timeout_ms = plan.timeout_ms;
        let settings = settings.clone();

        join_set.spawn(async move {
            let _permit = permit;

            {
                let guard = coordinator.lock().await;
                if guard.is_cancel_requested(&task_id).unwrap_or(false) {
                    let cancelled = SwarmResult {
                        subtask_id: subtask.id.clone(),
                        agent_name: subtask.agent_name.clone(),
                        output: String::new(),
                        confidence: 0.0,
                        duration_ms: 0,
                        status: "cancelled".to_string(),
                        error: Some("Cancelled before subtask started.".to_string()),
                    };
                    drop(guard);
                    let mut guard = coordinator.lock().await;
                    let _ = guard.complete_subtask(&task_id, cancelled);
                    return;
                }
            }

            {
                let mut coordinator = coordinator.lock().await;
                let _ = coordinator.mark_subtask_running(&task_id, &subtask.id);
            }

            let started = Instant::now();
            let execution = timeout(
                timeout_ms_to_duration(timeout_ms),
                (runner)(subtask.clone(), settings),
            )
            .await;

            let result = match execution {
                Ok(Ok(output)) => SwarmResult {
                    subtask_id: subtask.id.clone(),
                    agent_name: subtask.agent_name.clone(),
                    output: output.output,
                    confidence: output.confidence,
                    duration_ms: started.elapsed().as_millis() as u64,
                    status: "completed".to_string(),
                    error: None,
                },
                Ok(Err(error)) => SwarmResult {
                    subtask_id: subtask.id.clone(),
                    agent_name: subtask.agent_name.clone(),
                    output: String::new(),
                    confidence: 0.0,
                    duration_ms: started.elapsed().as_millis() as u64,
                    status: "failed".to_string(),
                    error: Some(error),
                },
                Err(_) => SwarmResult {
                    subtask_id: subtask.id.clone(),
                    agent_name: subtask.agent_name.clone(),
                    output: String::new(),
                    confidence: 0.0,
                    duration_ms: timeout_ms,
                    status: "timed_out".to_string(),
                    error: Some(format!("Subtask timed out after {} ms.", timeout_ms)),
                },
            };

            let mut coordinator = coordinator.lock().await;
            let _ = coordinator.complete_subtask(&task_id, result);
        });
    }

    while let Some(joined) = join_set.join_next().await {
        if let Err(error) = joined {
            let mut coordinator = coordinator.lock().await;
            if let Some(task) = coordinator.tasks.get_mut(&plan.task_id) {
                task.errors
                    .push(format!("A swarm worker panicked or was cancelled: {error}"));
            }
        }
    }

    let mut coordinator = coordinator.lock().await;
    let _ = coordinator.finalize_task(&plan.task_id)?;
    Ok(())
}

fn production_runner() -> SubtaskRunner {
    Arc::new(|subtask, settings| {
        Box::pin(async move {
            let registry = AgentRegistry::new();
            let gateway = Gateway::new(&settings);
            execute_subtask(&subtask, &settings, &registry, &gateway).await
        })
    })
}

async fn execute_subtask(
    subtask: &SwarmSubtask,
    settings: &Settings,
    registry: &AgentRegistry,
    gateway: &Gateway,
) -> Result<SwarmExecutionOutput, String> {
    if settings.configured_providers().is_empty() {
        return Ok(execute_subtask_locally(subtask, registry));
    }

    let agent = registry
        .get_by_name(&subtask.agent_name)
        .cloned()
        .unwrap_or_else(|| registry.find_best(&subtask.description).clone());
    let prompt = format!(
        "Main task sub-assignment for AgentOS swarm.\n\nAssigned agent: {}\nSubtask title: {}\nSubtask description:\n{}\n\nReturn a concise section that can be merged into a final report. Include:\n1. Key findings\n2. Risks or blockers\n3. Recommended next action",
        agent.name, subtask.title, subtask.description
    );
    let response = gateway
        .complete_with_system(&prompt, Some(&agent.system_prompt), settings)
        .await?;

    Ok(SwarmExecutionOutput {
        output: response.content.trim().to_string(),
        confidence: confidence_for_agent(&agent.name, subtask.source_hint.is_some()),
    })
}

fn execute_subtask_locally(
    subtask: &SwarmSubtask,
    registry: &AgentRegistry,
) -> SwarmExecutionOutput {
    let agent = registry
        .get_by_name(&subtask.agent_name)
        .cloned()
        .unwrap_or_else(|| registry.find_best(&subtask.description).clone());
    let focus = specialization_focus(&agent.name);
    let source = subtask
        .source_hint
        .clone()
        .unwrap_or_else(|| "shared task context".to_string());
    let output = format!(
        "Section owner: {}\nFocus: {}\nSource: {}\nSummary: {}\nAction: Consolidate this section into the final swarm report.",
        agent.name,
        focus,
        source,
        summarize_text(&subtask.description)
    );

    SwarmExecutionOutput {
        output,
        confidence: confidence_for_agent(&agent.name, subtask.source_hint.is_some()),
    }
}

fn derive_subtasks(description: &str, agents: &[String]) -> Vec<SwarmSubtask> {
    let sources = extract_sources(description);
    let work_items: Vec<Option<String>> = if sources.is_empty() {
        vec![None; agents.len()]
    } else {
        sources.into_iter().map(Some).collect()
    };

    work_items
        .into_iter()
        .enumerate()
        .map(|(idx, source_hint)| {
            let agent_name = agents[idx % agents.len()].clone();
            let focus = specialization_focus(&agent_name);
            let source_line = source_hint
                .as_ref()
                .map(|source| format!("Source: {}\n", source))
                .unwrap_or_default();
            let title = source_hint
                .as_ref()
                .map(|source| format!("{} on {}", agent_name, source))
                .unwrap_or_else(|| format!("{} workstream {}", agent_name, idx + 1));
            let description = format!(
                "{}Focus: {}\nGoal: {}",
                source_line,
                focus,
                description.trim()
            );

            SwarmSubtask {
                id: format!("subtask-{}", uuid::Uuid::new_v4()),
                title,
                description,
                agent_name,
                status: "pending".to_string(),
                result: None,
                error: None,
                source_hint,
                started_at: None,
                finished_at: None,
                duration_ms: None,
            }
        })
        .collect()
}

fn aggregate_results(description: &str, strategy: &str, results: &[SwarmResult]) -> Option<String> {
    if results.is_empty() {
        return None;
    }

    if normalize_strategy(strategy) == "vote" {
        return SwarmCoordinator::vote_consensus(results)
            .map(|winner| format!("Consensus winner: {}\n{}", winner.agent_name, winner.output));
    }

    let mut sections = vec![format!("Swarm report for: {}", description.trim())];
    for result in results {
        sections.push(format!(
            "\n## {}\n{}",
            result.agent_name,
            result.output.trim()
        ));
    }
    Some(sections.join("\n"))
}

fn extract_sources(description: &str) -> Vec<String> {
    let lowered = description.to_lowercase();
    let marker = if let Some(index) = lowered.find("sources:") {
        Some(("sources:", index))
    } else if let Some(index) = lowered.find("fuentes:") {
        Some(("fuentes:", index))
    } else {
        None
    };

    if let Some((label, index)) = marker {
        let tail = &description[index + label.len()..];
        return split_sources(tail);
    }

    let lines: Vec<String> = description
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("- ") || line.starts_with("* "))
        .map(|line| {
            line.trim_start_matches("- ")
                .trim_start_matches("* ")
                .to_string()
        })
        .collect();
    if !lines.is_empty() {
        return lines;
    }

    Vec::new()
}

fn split_sources(text: &str) -> Vec<String> {
    text.split(['|', ';', '\n'])
        .flat_map(|chunk| chunk.split(','))
        .map(str::trim)
        .filter(|chunk| !chunk.is_empty())
        .map(|chunk| chunk.to_string())
        .collect()
}

fn summarize_text(text: &str) -> String {
    let trimmed = text
        .split_whitespace()
        .take(24)
        .collect::<Vec<_>>()
        .join(" ");
    if trimmed.is_empty() {
        "No content provided.".to_string()
    } else {
        trimmed
    }
}

fn normalize_strategy(strategy: &str) -> &str {
    match strategy.trim().to_lowercase().as_str() {
        "sequential" => "sequential",
        "vote" => "vote",
        _ => "parallel",
    }
}

fn specialization_focus(agent_name: &str) -> &'static str {
    let lowered = agent_name.to_lowercase();
    if lowered.contains("research") {
        "Extract evidence and key findings"
    } else if lowered.contains("data") || lowered.contains("analyst") {
        "Quantify metrics, trends, or signals"
    } else if lowered.contains("report") {
        "Synthesize an executive-ready section"
    } else if lowered.contains("project") || lowered.contains("manager") {
        "Identify risks, dependencies, and next actions"
    } else if lowered.contains("review") || lowered.contains("qa") {
        "Surface issues, regressions, or verification gaps"
    } else {
        "Produce a specialist contribution for the shared task"
    }
}

fn confidence_for_agent(agent_name: &str, has_explicit_source: bool) -> f64 {
    let mut score: f64 = if has_explicit_source { 0.78 } else { 0.68 };
    let lowered = agent_name.to_lowercase();
    if lowered.contains("senior") || lowered.contains("manager") || lowered.contains("review") {
        score += 0.08;
    }
    score.clamp(0.0, 0.95)
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

fn timeout_ms_to_duration(timeout_ms: u64) -> Duration {
    Duration::from_millis(timeout_ms.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::Barrier;

    #[test]
    fn create_task_derives_explicit_subtasks_from_sources() {
        let mut coordinator = SwarmCoordinator::new();
        let task = coordinator
            .create_swarm_task(
                "Build a report. Sources: sales.csv | support.log | roadmap.md",
                vec![
                    "Researcher".to_string(),
                    "Data Analyst".to_string(),
                    "Report Generator".to_string(),
                ],
                "parallel",
                3,
                5_000,
            )
            .unwrap();

        assert_eq!(task.subtasks.len(), 3);
        assert!(task
            .subtasks
            .iter()
            .all(|subtask| subtask.source_hint.is_some()));
    }

    #[tokio::test]
    async fn execute_swarm_task_runs_subtasks_concurrently_and_aggregates_results() {
        let coordinator = Arc::new(Mutex::new(SwarmCoordinator::new()));
        let task = {
            let mut coordinator = coordinator.lock().await;
            coordinator
                .create_swarm_task(
                    "Build a report. Sources: finance.csv | support.log | roadmap.md",
                    vec![
                        "Researcher".to_string(),
                        "Data Analyst".to_string(),
                        "Report Generator".to_string(),
                    ],
                    "parallel",
                    3,
                    5_000,
                )
                .unwrap()
        };

        let barrier = Arc::new(Barrier::new(3));
        let in_flight = Arc::new(AtomicUsize::new(0));
        let max_in_flight = Arc::new(AtomicUsize::new(0));
        let runner: SubtaskRunner = {
            let barrier = barrier.clone();
            let in_flight = in_flight.clone();
            let max_in_flight = max_in_flight.clone();
            Arc::new(move |subtask, _settings| {
                let barrier = barrier.clone();
                let in_flight = in_flight.clone();
                let max_in_flight = max_in_flight.clone();
                Box::pin(async move {
                    let current = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                    max_in_flight.fetch_max(current, Ordering::SeqCst);
                    barrier.wait().await;
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    in_flight.fetch_sub(1, Ordering::SeqCst);
                    Ok(SwarmExecutionOutput {
                        output: format!("Finished {}", subtask.title),
                        confidence: 0.8,
                    })
                })
            })
        };

        let plan = { coordinator.lock().await.start_execution(&task.id).unwrap() };
        execute_swarm_task_inner(coordinator.clone(), plan, Settings::default(), runner)
            .await
            .unwrap();

        let task = coordinator.lock().await.get_results(&task.id).unwrap();
        assert_eq!(task.status, "completed");
        assert_eq!(task.results.len(), 3);
        assert!(task.aggregated_result.is_some());
        assert!(max_in_flight.load(Ordering::SeqCst) >= 2);
        println!("Swarm demo status: {}", task.status);
        println!(
            "Swarm demo concurrency observed: {}",
            max_in_flight.load(Ordering::SeqCst)
        );
        println!(
            "Swarm demo aggregated result:\n{}",
            task.aggregated_result.clone().unwrap_or_default()
        );
        for subtask in &task.subtasks {
            println!(
                "Swarm demo subtask: {} | agent={} | status={} | result_present={}",
                subtask.title,
                subtask.agent_name,
                subtask.status,
                subtask.result.is_some()
            );
        }
    }

    #[tokio::test]
    async fn execute_swarm_task_records_partial_failures_honestly() {
        let coordinator = Arc::new(Mutex::new(SwarmCoordinator::new()));
        let task = {
            let mut coordinator = coordinator.lock().await;
            coordinator
                .create_swarm_task(
                    "Build a report. Sources: finance.csv | support.log",
                    vec!["Researcher".to_string(), "Data Analyst".to_string()],
                    "parallel",
                    2,
                    5_000,
                )
                .unwrap()
        };

        let runner: SubtaskRunner = Arc::new(move |subtask, _settings| {
            Box::pin(async move {
                if subtask.agent_name == "Data Analyst" {
                    Err("Source parsing failed".to_string())
                } else {
                    Ok(SwarmExecutionOutput {
                        output: "Good result".to_string(),
                        confidence: 0.82,
                    })
                }
            })
        });

        let plan = { coordinator.lock().await.start_execution(&task.id).unwrap() };
        execute_swarm_task_inner(coordinator.clone(), plan, Settings::default(), runner)
            .await
            .unwrap();

        let task = coordinator.lock().await.get_results(&task.id).unwrap();
        assert_eq!(task.status, "completed_with_errors");
        assert_eq!(task.results.len(), 2);
        assert_eq!(task.errors.len(), 1);
    }

    #[tokio::test]
    async fn execute_swarm_task_honors_cancellation_for_pending_subtasks() {
        let coordinator = Arc::new(Mutex::new(SwarmCoordinator::new()));
        let task = {
            let mut coordinator = coordinator.lock().await;
            coordinator
                .create_swarm_task(
                    "Build a report. Sources: one | two | three",
                    vec![
                        "Researcher".to_string(),
                        "Data Analyst".to_string(),
                        "Report Generator".to_string(),
                    ],
                    "parallel",
                    1,
                    5_000,
                )
                .unwrap()
        };

        let runner: SubtaskRunner = Arc::new(move |subtask, _settings| {
            Box::pin(async move {
                tokio::time::sleep(Duration::from_millis(40)).await;
                Ok(SwarmExecutionOutput {
                    output: format!("Finished {}", subtask.title),
                    confidence: 0.8,
                })
            })
        });

        let execution = tokio::spawn(execute_swarm_task_inner(
            coordinator.clone(),
            coordinator.lock().await.start_execution(&task.id).unwrap(),
            Settings::default(),
            runner,
        ));

        tokio::time::sleep(Duration::from_millis(5)).await;
        coordinator.lock().await.request_cancel(&task.id).unwrap();
        execution.await.unwrap().unwrap();

        let task = coordinator.lock().await.get_results(&task.id).unwrap();
        assert_eq!(task.status, "cancelled");
        assert!(task
            .subtasks
            .iter()
            .any(|subtask| subtask.status == "cancelled"));
    }

    #[tokio::test]
    async fn execute_swarm_task_marks_timeouts() {
        let coordinator = Arc::new(Mutex::new(SwarmCoordinator::new()));
        let task = {
            let mut coordinator = coordinator.lock().await;
            coordinator
                .create_swarm_task(
                    "Build a report. Sources: one",
                    vec!["Researcher".to_string()],
                    "parallel",
                    1,
                    10,
                )
                .unwrap()
        };

        let runner: SubtaskRunner = Arc::new(move |_subtask, _settings| {
            Box::pin(async move {
                tokio::time::sleep(Duration::from_millis(50)).await;
                Ok(SwarmExecutionOutput {
                    output: "Late result".to_string(),
                    confidence: 0.8,
                })
            })
        });

        let plan = { coordinator.lock().await.start_execution(&task.id).unwrap() };
        execute_swarm_task_inner(coordinator.clone(), plan, Settings::default(), runner)
            .await
            .unwrap();

        let task = coordinator.lock().await.get_results(&task.id).unwrap();
        assert_eq!(task.status, "failed");
        assert_eq!(task.results[0].status, "timed_out");
    }

    #[tokio::test]
    async fn execute_swarm_task_respects_sequential_strategy() {
        let coordinator = Arc::new(Mutex::new(SwarmCoordinator::new()));
        let task = {
            let mut coordinator = coordinator.lock().await;
            coordinator
                .create_swarm_task(
                    "Build a report. Sources: one | two | three",
                    vec![
                        "Researcher".to_string(),
                        "Data Analyst".to_string(),
                        "Report Generator".to_string(),
                    ],
                    "sequential",
                    3,
                    5_000,
                )
                .unwrap()
        };

        let in_flight = Arc::new(AtomicUsize::new(0));
        let max_in_flight = Arc::new(AtomicUsize::new(0));
        let runner: SubtaskRunner = {
            let in_flight = in_flight.clone();
            let max_in_flight = max_in_flight.clone();
            Arc::new(move |subtask, _settings| {
                let in_flight = in_flight.clone();
                let max_in_flight = max_in_flight.clone();
                Box::pin(async move {
                    let current = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                    max_in_flight.fetch_max(current, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    in_flight.fetch_sub(1, Ordering::SeqCst);
                    Ok(SwarmExecutionOutput {
                        output: format!("Finished {}", subtask.title),
                        confidence: 0.75,
                    })
                })
            })
        };

        let plan = coordinator.lock().await.start_execution(&task.id).unwrap();
        execute_swarm_task_inner(coordinator.clone(), plan, Settings::default(), runner)
            .await
            .unwrap();

        let task = coordinator.lock().await.get_results(&task.id).unwrap();
        assert_eq!(task.status, "completed");
        assert_eq!(max_in_flight.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn execute_swarm_task_uses_vote_consensus_for_aggregation() {
        let coordinator = Arc::new(Mutex::new(SwarmCoordinator::new()));
        let task = {
            let mut coordinator = coordinator.lock().await;
            coordinator
                .create_swarm_task(
                    "Build a report. Sources: one | two | three",
                    vec![
                        "Researcher".to_string(),
                        "Senior Reviewer".to_string(),
                        "Report Generator".to_string(),
                    ],
                    "vote",
                    3,
                    5_000,
                )
                .unwrap()
        };

        let runner: SubtaskRunner = Arc::new(move |subtask, _settings| {
            Box::pin(async move {
                let confidence = match subtask.agent_name.as_str() {
                    "Senior Reviewer" => 0.91,
                    "Report Generator" => 0.77,
                    _ => 0.63,
                };
                Ok(SwarmExecutionOutput {
                    output: format!("Winner candidate from {}", subtask.agent_name),
                    confidence,
                })
            })
        });

        let plan = coordinator.lock().await.start_execution(&task.id).unwrap();
        execute_swarm_task_inner(coordinator.clone(), plan, Settings::default(), runner)
            .await
            .unwrap();

        let task = coordinator.lock().await.get_results(&task.id).unwrap();
        let aggregated = task.aggregated_result.unwrap_or_default();
        assert!(aggregated.contains("Consensus winner: Senior Reviewer"));
        assert!(aggregated.contains("Winner candidate from Senior Reviewer"));
    }
}
