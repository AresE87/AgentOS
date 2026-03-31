use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
    pub role: String,
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
    db_path: PathBuf,
}

impl CollabManager {
    pub fn new(db_path: PathBuf) -> Result<Self, String> {
        let manager = Self { db_path };
        let conn = manager.open()?;
        Self::ensure_tables(&conn)?;
        Ok(manager)
    }

    fn open(&self) -> Result<Connection, String> {
        let conn = Connection::open(&self.db_path).map_err(|e| e.to_string())?;
        Self::ensure_tables(&conn)?;
        Ok(conn)
    }

    pub fn ensure_tables(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS collab_sessions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                creator TEXT NOT NULL,
                task TEXT NOT NULL,
                status TEXT NOT NULL,
                shared_context TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS collab_participants (
                session_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                contributed_agents_json TEXT NOT NULL,
                role TEXT NOT NULL,
                joined_at TEXT NOT NULL,
                PRIMARY KEY (session_id, user_id)
            );
            CREATE TABLE IF NOT EXISTS collab_results (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                from_user TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                content TEXT NOT NULL,
                shared_at TEXT NOT NULL
            );",
        )
        .map_err(|e| e.to_string())
    }

    pub fn create_session(
        &self,
        name: String,
        creator: String,
        task: String,
        shared_context: String,
    ) -> Result<CollabSession, String> {
        let conn = self.open()?;
        let session_id = uuid::Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now().to_rfc3339();
        let owner_id = creator.clone();
        conn.execute(
            "INSERT INTO collab_sessions (id, name, creator, task, status, shared_context, created_at)
             VALUES (?1, ?2, ?3, ?4, 'active', ?5, ?6)",
            params![session_id, name, creator, task, shared_context, created_at],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO collab_participants (session_id, user_id, contributed_agents_json, role, joined_at)
             VALUES (?1, ?2, '[]', 'owner', ?3)",
            params![session_id, owner_id, created_at],
        )
        .map_err(|e| e.to_string())?;
        self.get_session(&session_id)?
            .ok_or_else(|| "Session not found after creation".to_string())
    }

    pub fn join_session(
        &self,
        session_id: &str,
        user_id: String,
        agents: Vec<String>,
    ) -> Result<CollabSession, String> {
        let conn = self.open()?;
        if self.is_participant_with_conn(&conn, session_id, &user_id)? {
            return Err("Already in this session".to_string());
        }
        conn.execute(
            "INSERT INTO collab_participants (session_id, user_id, contributed_agents_json, role, joined_at)
             VALUES (?1, ?2, ?3, 'contributor', ?4)",
            params![
                session_id,
                user_id,
                serde_json::to_string(&agents).map_err(|e| e.to_string())?,
                chrono::Utc::now().to_rfc3339()
            ],
        )
        .map_err(|e| e.to_string())?;
        self.get_session(session_id)?
            .ok_or_else(|| "Session not found".to_string())
    }

    pub fn list_sessions(&self, user_id: Option<&str>) -> Result<Vec<CollabSession>, String> {
        let conn = self.open()?;
        let sql = if user_id.is_some() {
            "SELECT DISTINCT s.id, s.name, s.creator, s.task, s.status, s.shared_context, s.created_at
             FROM collab_sessions s
             LEFT JOIN collab_participants p ON p.session_id = s.id
             WHERE s.creator = ?1 OR p.user_id = ?1
             ORDER BY s.created_at DESC"
        } else {
            "SELECT id, name, creator, task, status, shared_context, created_at
             FROM collab_sessions
             ORDER BY created_at DESC"
        };
        let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
        let session_ids = if let Some(user_id) = user_id {
            let rows = stmt
                .query_map(params![user_id], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            rows
        } else {
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            rows
        };

        let mut sessions = Vec::new();
        for session_id in session_ids {
            if let Some(session) = self.get_session(&session_id)? {
                sessions.push(session);
            }
        }
        Ok(sessions)
    }

    pub fn share_result(
        &self,
        session_id: &str,
        from_user: String,
        agent_id: String,
        content: String,
    ) -> Result<SharedResult, String> {
        let conn = self.open()?;
        if !self.is_participant_with_conn(&conn, session_id, &from_user)? {
            return Err("User is not allowed to share results in this session".to_string());
        }
        let result = SharedResult {
            from_user,
            agent_id,
            content,
            shared_at: chrono::Utc::now().to_rfc3339(),
        };
        conn.execute(
            "INSERT INTO collab_results (id, session_id, from_user, agent_id, content, shared_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                uuid::Uuid::new_v4().to_string(),
                session_id,
                result.from_user,
                result.agent_id,
                result.content,
                result.shared_at
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(result)
    }

    pub fn get_session(&self, session_id: &str) -> Result<Option<CollabSession>, String> {
        let conn = self.open()?;
        let row: Option<(String, String, String, String, String, String, String)> = conn
            .query_row(
                "SELECT id, name, creator, task, status, shared_context, created_at
                 FROM collab_sessions WHERE id = ?1",
                params![session_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let Some((id, name, creator, task, status, shared_context, created_at)) = row else {
            return Ok(None);
        };
        let participants = self.list_participants_with_conn(&conn, &id)?;
        let results = self.list_results_with_conn(&conn, &id)?;
        Ok(Some(CollabSession {
            id,
            name,
            creator,
            participants,
            task,
            status: SessionStatus::from_str(&status),
            shared_context,
            results,
            created_at,
        }))
    }

    fn is_participant_with_conn(
        &self,
        conn: &Connection,
        session_id: &str,
        user_id: &str,
    ) -> Result<bool, String> {
        let found: Option<String> = conn
            .query_row(
                "SELECT user_id FROM collab_participants WHERE session_id = ?1 AND user_id = ?2",
                params![session_id, user_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        Ok(found.is_some())
    }

    fn list_participants_with_conn(
        &self,
        conn: &Connection,
        session_id: &str,
    ) -> Result<Vec<Participant>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT user_id, contributed_agents_json, role, joined_at
                 FROM collab_participants
                 WHERE session_id = ?1
                 ORDER BY joined_at ASC",
            )
            .map_err(|e| e.to_string())?;
        let participants = stmt
            .query_map(params![session_id], |row| {
            let agents_json: String = row.get(1)?;
            Ok(Participant {
                user_id: row.get(0)?,
                contributed_agents: serde_json::from_str(&agents_json).unwrap_or_default(),
                role: row.get(2)?,
                joined_at: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
        Ok(participants)
    }

    fn list_results_with_conn(
        &self,
        conn: &Connection,
        session_id: &str,
    ) -> Result<Vec<SharedResult>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT from_user, agent_id, content, shared_at
                 FROM collab_results
                 WHERE session_id = ?1
                 ORDER BY shared_at ASC",
            )
            .map_err(|e| e.to_string())?;
        let results = stmt
            .query_map(params![session_id], |row| {
            Ok(SharedResult {
                from_user: row.get(0)?,
                agent_id: row.get(1)?,
                content: row.get(2)?,
                shared_at: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
        Ok(results)
    }
}

impl SessionStatus {
    fn from_str(value: &str) -> Self {
        match value {
            "completed" => SessionStatus::Completed,
            "archived" => SessionStatus::Archived,
            _ => SessionStatus::Active,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn collaboration_is_restricted_to_participants() {
        let dir = tempdir().unwrap();
        let manager = CollabManager::new(dir.path().join("collab.db")).unwrap();

        let session = manager
            .create_session(
                "Ops War Room".to_string(),
                "alice".to_string(),
                "triage queue".to_string(),
                "shared context".to_string(),
            )
            .unwrap();
        manager
            .join_session(&session.id, "bob".to_string(), vec!["agent-1".to_string()])
            .unwrap();

        assert!(manager
            .share_result(
                &session.id,
                "bob".to_string(),
                "agent-1".to_string(),
                "done".to_string()
            )
            .is_ok());
        assert!(manager
            .share_result(
                &session.id,
                "mallory".to_string(),
                "agent-9".to_string(),
                "forged".to_string()
            )
            .is_err());
        assert_eq!(manager.list_sessions(Some("bob")).unwrap().len(), 1);
    }
}
