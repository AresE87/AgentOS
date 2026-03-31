// ── R144: Microtasks Marketplace ──────────────────────────────────
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MicrotaskStatus {
    Available,
    Claimed,
    InProgress,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Microtask {
    pub id: String,
    pub title: String,
    pub description: String,
    pub reward_amount: f64,
    pub deadline: Option<String>,
    pub status: MicrotaskStatus,
    pub assigned_to: Option<String>,
    pub poster_id: String,
    pub result: Option<String>,
    pub created_at: String,
}

pub struct MicrotaskMarket {
    tasks: Vec<Microtask>,
}

impl MicrotaskMarket {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    pub fn post_task(
        &mut self,
        title: String,
        description: String,
        reward_amount: f64,
        deadline: Option<String>,
        poster_id: String,
    ) -> Microtask {
        let task = Microtask {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            description,
            reward_amount,
            deadline,
            status: MicrotaskStatus::Available,
            assigned_to: None,
            poster_id,
            result: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.tasks.push(task.clone());
        task
    }

    pub fn claim_task(&mut self, task_id: &str, agent_id: String) -> Result<Microtask, String> {
        let task = self
            .tasks
            .iter_mut()
            .find(|t| t.id == task_id)
            .ok_or_else(|| "Task not found".to_string())?;
        if task.status != MicrotaskStatus::Available {
            return Err("Task is not available".to_string());
        }
        task.status = MicrotaskStatus::Claimed;
        task.assigned_to = Some(agent_id);
        Ok(task.clone())
    }

    pub fn complete_task(&mut self, task_id: &str, result: String) -> Result<Microtask, String> {
        let task = self
            .tasks
            .iter_mut()
            .find(|t| t.id == task_id)
            .ok_or_else(|| "Task not found".to_string())?;
        if task.status != MicrotaskStatus::Claimed {
            return Err("Task is not in claimed state".to_string());
        }
        task.status = MicrotaskStatus::Completed;
        task.result = Some(result);
        Ok(task.clone())
    }

    pub fn list_available(&self) -> Vec<&Microtask> {
        self.tasks
            .iter()
            .filter(|t| t.status == MicrotaskStatus::Available)
            .collect()
    }

    pub fn list_my_tasks(&self, agent_id: &str) -> Vec<&Microtask> {
        self.tasks
            .iter()
            .filter(|t| t.assigned_to.as_deref() == Some(agent_id))
            .collect()
    }
}
