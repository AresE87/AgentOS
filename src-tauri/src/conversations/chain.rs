use serde::{Deserialize, Serialize};

const MAX_ROUNDS: usize = 5;
const MAX_MESSAGES: usize = 15;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: String,
    pub from_agent: String,
    pub to_agent: String,
    pub message_type: String, // "request", "response", "review", "approval"
    pub content: String,
    pub context: Option<String>,
    pub requires_response: bool,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationChain {
    pub id: String,
    pub topic: String,
    pub participants: Vec<String>,
    pub messages: Vec<AgentMessage>,
    pub max_rounds: usize,
    pub status: String, // "active", "completed", "consensus", "timeout"
    pub created_at: String,
}

impl ConversationChain {
    pub fn new(topic: &str, participants: Vec<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            topic: topic.to_string(),
            participants,
            messages: vec![],
            max_rounds: MAX_ROUNDS,
            status: "active".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn add_message(&mut self, msg: AgentMessage) -> Result<(), String> {
        if self.messages.len() >= MAX_MESSAGES {
            self.status = "timeout".to_string();
            return Err("Max messages reached".into());
        }
        self.messages.push(msg);
        Ok(())
    }

    pub fn current_round(&self) -> usize {
        // Count how many full exchanges have happened
        let exchanges = self.messages.len() / self.participants.len().max(1);
        exchanges.min(self.max_rounds)
    }

    pub fn is_complete(&self) -> bool {
        self.status != "active" || self.current_round() >= self.max_rounds
    }

    pub fn mark_complete(&mut self, reason: &str) {
        self.status = reason.to_string();
    }

    pub fn get_context_for_agent(&self, _agent_name: &str) -> String {
        // Build context from all previous messages for this agent
        self.messages
            .iter()
            .map(|m| format!("[{}->{}]: {}", m.from_agent, m.to_agent, m.content))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn last_message(&self) -> Option<&AgentMessage> {
        self.messages.last()
    }

    pub fn summary(&self) -> String {
        format!(
            "{} -- {} messages, {} rounds, status: {}",
            self.topic,
            self.messages.len(),
            self.current_round(),
            self.status
        )
    }
}

/// Pre-built conversation patterns
pub fn review_pattern() -> Vec<String> {
    vec!["Programmer".into(), "Code Reviewer".into()]
}

pub fn research_pattern() -> Vec<String> {
    vec!["Researcher".into(), "Analyst".into()]
}

pub fn design_pattern() -> Vec<String> {
    vec!["Designer".into(), "Developer".into()]
}
