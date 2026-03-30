use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result from an individual agent in the swarm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmResult {
    pub agent_name: String,
    pub output: String,
    pub confidence: f64,
    pub duration_ms: u64,
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
    pub results: Vec<SwarmResult>,
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
    ) -> SwarmTask {
        let id = format!("swarm-{}", uuid::Uuid::new_v4());
        let task = SwarmTask {
            id: id.clone(),
            description: description.to_string(),
            assigned_agents: agents,
            strategy: strategy.to_string(),
            status: "pending".to_string(),
            results: Vec::new(),
        };
        self.tasks.insert(id, task.clone());
        task
    }

    /// Execute a swarm task (stub — simulates agent execution).
    pub fn execute(&mut self, task_id: &str) -> Result<SwarmTask, String> {
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| format!("Swarm task '{}' not found", task_id))?;

        if task.status != "pending" {
            return Err(format!("Task '{}' is already {}", task_id, task.status));
        }

        task.status = "running".to_string();

        // Simulate each agent producing a result
        let mut results = Vec::new();
        for (i, agent) in task.assigned_agents.iter().enumerate() {
            results.push(SwarmResult {
                agent_name: agent.clone(),
                output: format!(
                    "Agent '{}' completed subtask for: {} (strategy: {})",
                    agent, task.description, task.strategy
                ),
                confidence: 0.7 + (i as f64 * 0.05).min(0.29),
                duration_ms: 100 + (i as u64 * 50),
            });
        }

        task.results = results;
        task.status = "completed".to_string();

        Ok(task.clone())
    }

    /// Get results for a swarm task.
    pub fn get_results(&self, task_id: &str) -> Result<SwarmTask, String> {
        self.tasks
            .get(task_id)
            .cloned()
            .ok_or_else(|| format!("Swarm task '{}' not found", task_id))
    }

    /// Vote-based consensus: return the result with highest confidence.
    pub fn vote_consensus(results: &[SwarmResult]) -> Option<SwarmResult> {
        if results.is_empty() {
            return None;
        }
        results
            .iter()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal))
            .cloned()
    }

    /// List all swarm tasks.
    pub fn list_tasks(&self) -> Vec<SwarmTask> {
        self.tasks.values().cloned().collect()
    }
}
