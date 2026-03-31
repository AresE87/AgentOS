use crate::agents::AgentRegistry;
use crate::brain::Gateway;
use crate::config::Settings;
use crate::memory::Database;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmResult {
    pub agent_name: String,
    pub output: String,
    pub model: String,
    pub status: String,
    pub duration_ms: u64,
    pub cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmConsensus {
    pub agent_name: String,
    pub rationale: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmTask {
    pub id: String,
    pub description: String,
    pub assigned_agents: Vec<String>,
    pub strategy: String,
    pub status: String,
    pub chain_id: String,
    pub created_at: String,
    pub results: Vec<SwarmResult>,
    pub consensus: Option<SwarmConsensus>,
}

pub struct SwarmCoordinator {
    tasks: HashMap<String, SwarmTask>,
}

impl SwarmCoordinator {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
        }
    }

    pub fn create_swarm_task(
        &mut self,
        description: &str,
        agents: Vec<String>,
        strategy: &str,
    ) -> SwarmTask {
        let id = format!("swarm-{}", uuid::Uuid::new_v4());
        let chain_id = format!("chain-{}", id);
        let task = SwarmTask {
            id: id.clone(),
            description: description.to_string(),
            assigned_agents: agents,
            strategy: strategy.to_string(),
            status: "pending".to_string(),
            chain_id,
            created_at: chrono::Utc::now().to_rfc3339(),
            results: Vec::new(),
            consensus: None,
        };
        self.tasks.insert(id, task.clone());
        task
    }

    pub async fn execute(
        &mut self,
        task_id: &str,
        settings: &Settings,
        db_path: &Path,
    ) -> Result<SwarmTask, String> {
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| format!("Swarm task '{}' not found", task_id))?;

        if task.status != "pending" {
            return Err(format!("Task '{}' is already {}", task_id, task.status));
        }

        let gateway = Gateway::new(settings);
        let registry = AgentRegistry::new();
        let chain_id = task.chain_id.clone();
        let mut total_cost = 0.0;
        let mut prior_outputs: Vec<String> = Vec::new();

        task.status = "running".to_string();
        task.results.clear();
        task.consensus = None;

        if let Ok(db) = Database::new(db_path) {
            let _ = db.create_chain(&chain_id, &task.description);
            let _ = db.insert_chain_event(
                &chain_id,
                "Swarm",
                "orchestrator",
                "swarm_started",
                &format!(
                    "Swarm task started with {} assigned agents using {} strategy",
                    task.assigned_agents.len(),
                    task.strategy
                ),
                None,
            );
        }

        for (index, requested_agent) in task.assigned_agents.clone().iter().enumerate() {
            let subtask_id = format!("{}-{}", chain_id, index + 1);
            let profile = registry
                .get_by_name(requested_agent)
                .cloned()
                .unwrap_or_else(|| registry.find_best(&task.description).clone());
            let prompt = build_agent_prompt(&task.description, &task.strategy, &prior_outputs);

            if let Ok(db) = Database::new(db_path) {
                let _ = db.insert_chain_subtask(&subtask_id, &chain_id, index as i32, &task.description);
                let _ = db.update_subtask_status(
                    &subtask_id,
                    "running",
                    "Executing assigned agent response",
                    "",
                    0.0,
                    0,
                    &profile.name,
                    "",
                );
                let _ = db.insert_chain_event(
                    &chain_id,
                    &profile.name,
                    &format!("{:?}", profile.level),
                    "agent_started",
                    &format!("Executing swarm response for '{}'", task.description),
                    None,
                );
            }

            let started_at = Instant::now();
            let system_prompt = format!(
                "You are {}. {}\nReturn a direct working response for your assigned objective.",
                profile.name, profile.system_prompt
            );

            match gateway.complete_as_agent(&prompt, &system_prompt, settings).await {
                Ok(response) => {
                    let duration_ms = started_at.elapsed().as_millis() as u64;
                    total_cost += response.cost;

                    prior_outputs.push(format!("{}: {}", profile.name, response.content));
                    task.results.push(SwarmResult {
                        agent_name: profile.name.clone(),
                        output: response.content.clone(),
                        model: response.model.clone(),
                        status: "completed".to_string(),
                        duration_ms,
                        cost: response.cost,
                    });

                    if let Ok(db) = Database::new(db_path) {
                        let _ = db.update_subtask_status(
                            &subtask_id,
                            "done",
                            "Completed real agent response",
                            &response.content,
                            response.cost,
                            duration_ms,
                            &profile.name,
                            &response.model,
                        );
                        let _ = db.insert_chain_event(
                            &chain_id,
                            &profile.name,
                            &format!("{:?}", profile.level),
                            "agent_completed",
                            &format!("Completed response in {} ms", duration_ms),
                            None,
                        );
                    }
                }
                Err(error) => {
                    task.results.push(SwarmResult {
                        agent_name: profile.name.clone(),
                        output: error.clone(),
                        model: String::new(),
                        status: "failed".to_string(),
                        duration_ms: started_at.elapsed().as_millis() as u64,
                        cost: 0.0,
                    });

                    if let Ok(db) = Database::new(db_path) {
                        let _ = db.update_subtask_status(
                            &subtask_id,
                            "failed",
                            &format!("Agent execution failed: {}", error),
                            "",
                            0.0,
                            started_at.elapsed().as_millis() as u64,
                            &profile.name,
                            "",
                        );
                        let _ = db.insert_chain_event(
                            &chain_id,
                            &profile.name,
                            &format!("{:?}", profile.level),
                            "agent_failed",
                            &error,
                            None,
                        );
                    }
                }
            }
        }

        if task.strategy == "vote" {
            task.consensus = judge_consensus(&gateway, settings, &task.description, &task.results).await;
        }

        task.status = if task.results.iter().any(|result| result.status == "completed") {
            "completed".to_string()
        } else {
            "failed".to_string()
        };

        if let Ok(db) = Database::new(db_path) {
            let _ = db.complete_chain(&chain_id, total_cost);
            let _ = db.insert_chain_event(
                &chain_id,
                "Swarm",
                "orchestrator",
                "swarm_finished",
                &format!("Swarm task finished with status {}", task.status),
                None,
            );
        }

        Ok(task.clone())
    }

    pub fn get_results(&self, task_id: &str) -> Result<SwarmTask, String> {
        self.tasks
            .get(task_id)
            .cloned()
            .ok_or_else(|| format!("Swarm task '{}' not found", task_id))
    }

    pub fn list_tasks(&self) -> Vec<SwarmTask> {
        self.tasks.values().cloned().collect()
    }
}

fn build_agent_prompt(description: &str, strategy: &str, prior_outputs: &[String]) -> String {
    match strategy {
        "sequential" if !prior_outputs.is_empty() => format!(
            "Original objective: {}\n\nPrevious agent outputs:\n{}\n\nContinue from the prior work and improve or complete it.",
            description,
            prior_outputs.join("\n\n")
        ),
        "vote" => format!(
            "Original objective: {}\n\nRespond independently with your best complete answer. Do not mention voting.",
            description
        ),
        _ => format!(
            "Original objective: {}\n\nRespond independently with a direct execution-ready answer.",
            description
        ),
    }
}

async fn judge_consensus(
    gateway: &Gateway,
    settings: &Settings,
    description: &str,
    results: &[SwarmResult],
) -> Option<SwarmConsensus> {
    let successful: Vec<&SwarmResult> = results
        .iter()
        .filter(|result| result.status == "completed" && !result.output.trim().is_empty())
        .collect();

    if successful.is_empty() {
        return None;
    }

    if successful.len() == 1 {
        let single = successful[0];
        return Some(SwarmConsensus {
            agent_name: single.agent_name.clone(),
            rationale: "Only one successful agent response was available.".to_string(),
            model: single.model.clone(),
        });
    }

    let options = successful
        .iter()
        .map(|result| format!("Agent: {}\nOutput:\n{}", result.agent_name, result.output))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");

    let prompt = format!(
        "Objective: {}\n\nCompare these agent outputs and choose the strongest one. Return ONLY JSON in this shape: {{\"agent_name\":\"exact name\",\"rationale\":\"one short sentence\"}}\n\n{}",
        description,
        options
    );

    let response = gateway
        .complete_as_agent(
            &prompt,
            "You are a swarm judge. Pick the best response and explain briefly in one sentence.",
            settings,
        )
        .await
        .ok()?;

    let parsed: serde_json::Value = serde_json::from_str(response.content.trim()).ok()?;
    let agent_name = parsed.get("agent_name")?.as_str()?.to_string();
    let rationale = parsed
        .get("rationale")
        .and_then(|value| value.as_str())
        .unwrap_or("Chosen by real swarm judge step.")
        .to_string();

    Some(SwarmConsensus {
        agent_name,
        rationale,
        model: response.model,
    })
}
