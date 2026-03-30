use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single step in a reasoning chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStep {
    pub step_number: u32,
    pub thought: String,
    pub conclusion: String,
    pub confidence: f64,
}

/// A complete reasoning chain from task to final answer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningChain {
    pub id: String,
    pub task_id: String,
    pub steps: Vec<ReasoningStep>,
    pub final_answer: String,
    pub total_steps: u32,
    pub created_at: String,
}

/// Engine that manages reasoning chains in memory
pub struct ReasoningEngine {
    chains: HashMap<String, ReasoningChain>,
    next_id: u64,
}

impl ReasoningEngine {
    pub fn new() -> Self {
        Self {
            chains: HashMap::new(),
            next_id: 1,
        }
    }

    /// Start a new reasoning chain for a task
    pub fn create_chain(&mut self, task_id: &str) -> ReasoningChain {
        let id = format!("rc-{}", self.next_id);
        self.next_id += 1;
        let chain = ReasoningChain {
            id: id.clone(),
            task_id: task_id.to_string(),
            steps: Vec::new(),
            final_answer: String::new(),
            total_steps: 0,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.chains.insert(id, chain.clone());
        chain
    }

    /// Add a reasoning step to an existing chain
    pub fn add_step(
        &mut self,
        chain_id: &str,
        thought: &str,
        conclusion: &str,
        confidence: f64,
    ) -> Result<ReasoningStep, String> {
        let chain = self
            .chains
            .get_mut(chain_id)
            .ok_or_else(|| format!("Chain {} not found", chain_id))?;

        let step = ReasoningStep {
            step_number: chain.steps.len() as u32 + 1,
            thought: thought.to_string(),
            conclusion: conclusion.to_string(),
            confidence: confidence.clamp(0.0, 1.0),
        };
        chain.steps.push(step.clone());
        chain.total_steps = chain.steps.len() as u32;
        Ok(step)
    }

    /// Finish a chain with a final answer
    pub fn finish_chain(&mut self, chain_id: &str, answer: &str) -> Result<ReasoningChain, String> {
        let chain = self
            .chains
            .get_mut(chain_id)
            .ok_or_else(|| format!("Chain {} not found", chain_id))?;
        chain.final_answer = answer.to_string();
        Ok(chain.clone())
    }

    /// Get a chain by id
    pub fn get_chain(&self, id: &str) -> Option<&ReasoningChain> {
        self.chains.get(id)
    }

    /// List chains, most recent first
    pub fn list_chains(&self, limit: usize) -> Vec<&ReasoningChain> {
        let mut chains: Vec<&ReasoningChain> = self.chains.values().collect();
        chains.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        chains.truncate(limit);
        chains
    }
}
