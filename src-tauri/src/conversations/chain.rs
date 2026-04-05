use rusqlite::Connection;
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

    // ── SQLite persistence ──────────────────────────────────────────

    /// Ensure the conversation_chains table exists.
    pub fn ensure_table(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS conversation_chains (
                id TEXT PRIMARY KEY,
                topic TEXT NOT NULL,
                participants TEXT NOT NULL,
                messages TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL
            )"
        ).map_err(|e| e.to_string())
    }

    /// Persist (upsert) a conversation chain to SQLite.
    pub fn save_to_db(conn: &Connection, chain: &Self) -> Result<(), String> {
        Self::ensure_table(conn)?;

        let participants = serde_json::to_string(&chain.participants).unwrap_or_else(|_| "[]".into());
        let messages = serde_json::to_string(&chain.messages).unwrap_or_else(|_| "[]".into());

        conn.execute(
            "INSERT OR REPLACE INTO conversation_chains (id, topic, participants, messages, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![chain.id, chain.topic, participants, messages, chain.status, chain.created_at],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Load all persisted conversation chains from SQLite (most recent first, max 50).
    pub fn load_all(conn: &Connection) -> Vec<Self> {
        if Self::ensure_table(conn).is_err() {
            return Vec::new();
        }

        let mut stmt = match conn.prepare(
            "SELECT id, topic, participants, messages, status, created_at FROM conversation_chains ORDER BY created_at DESC LIMIT 50"
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let rows = match stmt.query_map([], |row| {
            let participants_str: String = row.get(2)?;
            let messages_str: String = row.get(3)?;
            Ok(ConversationChain {
                id: row.get(0)?,
                topic: row.get(1)?,
                participants: serde_json::from_str(&participants_str).unwrap_or_default(),
                messages: serde_json::from_str(&messages_str).unwrap_or_default(),
                max_rounds: MAX_ROUNDS,
                status: row.get(4)?,
                created_at: row.get(5)?,
            })
        }) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };

        rows.filter_map(|r| r.ok()).collect()
    }

    /// Delete a conversation chain from the database.
    pub fn delete_from_db(conn: &Connection, id: &str) -> Result<(), String> {
        Self::ensure_table(conn)?;
        conn.execute("DELETE FROM conversation_chains WHERE id = ?1", rusqlite::params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
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
