use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub id: String,
    pub created_at: String,
    pub message_count: u32,
    pub last_message_at: Option<String>,
}

pub struct SessionStore {
    sessions_dir: PathBuf,
}

impl SessionStore {
    pub fn new(sessions_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&sessions_dir).ok();
        Self { sessions_dir }
    }

    /// Append a single message (as JSONL) to a session file.
    pub fn append_message(
        &self,
        session_id: &str,
        message: &serde_json::Value,
    ) -> Result<(), String> {
        let path = self.sessions_dir.join(format!("{}.jsonl", session_id));
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        let line = serde_json::to_string(message).map_err(|e| e.to_string())?;
        writeln!(file, "{}", line).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Load all messages from a session file.
    pub fn load_session(&self, session_id: &str) -> Result<Vec<serde_json::Value>, String> {
        let path = self.sessions_dir.join(format!("{}.jsonl", session_id));
        let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        Ok(content
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect())
    }

    /// List all sessions with basic metadata.
    pub fn list_sessions(&self) -> Result<Vec<SessionMetadata>, String> {
        let mut sessions = vec![];
        if !self.sessions_dir.exists() {
            return Ok(sessions);
        }
        for entry in std::fs::read_dir(&self.sessions_dir)
            .map_err(|e| e.to_string())?
            .flatten()
        {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                let id = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let meta = std::fs::metadata(&path).ok();
                let content = std::fs::read_to_string(&path).unwrap_or_default();
                let line_count = content.lines().count() as u32;
                sessions.push(SessionMetadata {
                    id,
                    created_at: meta
                        .as_ref()
                        .and_then(|m| m.created().ok())
                        .map(|t| format!("{:?}", t))
                        .unwrap_or_default(),
                    message_count: line_count,
                    last_message_at: None,
                });
            }
        }
        Ok(sessions)
    }

    /// Delete a session file.
    pub fn delete_session(&self, session_id: &str) -> Result<(), String> {
        let path = self.sessions_dir.join(format!("{}.jsonl", session_id));
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
