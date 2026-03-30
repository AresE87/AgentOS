// ── R143: Cross-User Collaboration ────────────────────────────────
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    Active,
    Completed,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    pub user_id: String,
    pub contributed_agents: Vec<String>,
    pub role: String, // "owner", "contributor", "viewer"
    pub joined_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedResult {
    pub from_user: String,
    pub agent_id: String,
    pub content: String,
    pub shared_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollabSession {
    pub id: String,
    pub name: String,
    pub creator: String,
    pub participants: Vec<Participant>,
    pub task: String,
    pub status: SessionStatus,
    pub shared_context: String,
    pub results: Vec<SharedResult>,
    pub created_at: String,
}

pub struct CollabManager {
    sessions: Vec<CollabSession>,
}

impl CollabManager {
    pub fn new() -> Self {
        Self { sessions: Vec::new() }
    }

    pub fn create_session(&mut self, name: String, creator: String, task: String, shared_context: String) -> CollabSession {
        let session = CollabSession {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            creator: creator.clone(),
            participants: vec![Participant {
                user_id: creator,
                contributed_agents: Vec::new(),
                role: "owner".to_string(),
                joined_at: chrono::Utc::now().to_rfc3339(),
            }],
            task,
            status: SessionStatus::Active,
            shared_context,
            results: Vec::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.sessions.push(session.clone());
        session
    }

    pub fn join_session(&mut self, session_id: &str, user_id: String, agents: Vec<String>) -> Result<CollabSession, String> {
        let session = self.sessions.iter_mut().find(|s| s.id == session_id)
            .ok_or_else(|| "Session not found".to_string())?;
        if session.participants.iter().any(|p| p.user_id == user_id) {
            return Err("Already in this session".to_string());
        }
        session.participants.push(Participant {
            user_id,
            contributed_agents: agents,
            role: "contributor".to_string(),
            joined_at: chrono::Utc::now().to_rfc3339(),
        });
        Ok(session.clone())
    }

    pub fn list_sessions(&self) -> Vec<&CollabSession> {
        self.sessions.iter().collect()
    }

    pub fn share_result(&mut self, session_id: &str, from_user: String, agent_id: String, content: String) -> Result<SharedResult, String> {
        let session = self.sessions.iter_mut().find(|s| s.id == session_id)
            .ok_or_else(|| "Session not found".to_string())?;
        let result = SharedResult {
            from_user,
            agent_id,
            content,
            shared_at: chrono::Utc::now().to_rfc3339(),
        };
        session.results.push(result.clone());
        Ok(result)
    }
}
