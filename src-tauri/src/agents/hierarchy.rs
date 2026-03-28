use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentLevel {
    Junior,     // Simple tasks, cheap models (~$0.001)
    Specialist, // Domain-specific, mid-range (~$0.01)
    Senior,     // Complex analysis, premium (~$0.05)
    Manager,    // Multi-step orchestration (~$0.10)
}

impl AgentLevel {
    pub fn tier(&self) -> &str {
        match self {
            AgentLevel::Junior => "cheap",
            AgentLevel::Specialist => "standard",
            AgentLevel::Senior => "premium",
            AgentLevel::Manager => "premium",
        }
    }

    pub fn max_tokens(&self) -> u32 {
        match self {
            AgentLevel::Junior => 1024,
            AgentLevel::Specialist => 4096,
            AgentLevel::Senior => 8192,
            AgentLevel::Manager => 8192,
        }
    }

    pub fn temperature(&self) -> f64 {
        match self {
            AgentLevel::Junior => 0.3,
            AgentLevel::Specialist => 0.5,
            AgentLevel::Senior => 0.7,
            AgentLevel::Manager => 0.7,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfile {
    pub name: String,
    pub category: String,
    pub level: AgentLevel,
    pub system_prompt: String,
    pub tools: Vec<String>,
    pub keywords: Vec<String>,
}
