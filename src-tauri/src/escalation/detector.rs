use crate::memory::Database;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EscalationReason {
    LowConfidence,
    RepeatedRetries,
    FinancialAction,
    MissingCredentials,
    SystemUnavailable,
    UserRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HandoffStatus {
    PendingHandoff,
    AssignedToHuman,
    Resumed,
    CompletedByHuman,
}

impl HandoffStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            HandoffStatus::PendingHandoff => "pending_handoff",
            HandoffStatus::AssignedToHuman => "assigned_to_human",
            HandoffStatus::Resumed => "resumed",
            HandoffStatus::CompletedByHuman => "completed_by_human",
        }
    }

    pub fn from_str(status: &str) -> Result<Self, String> {
        match status {
            "pending_handoff" => Ok(HandoffStatus::PendingHandoff),
            "assigned_to_human" => Ok(HandoffStatus::AssignedToHuman),
            "resumed" => Ok(HandoffStatus::Resumed),
            "completed_by_human" => Ok(HandoffStatus::CompletedByHuman),
            other => Err(format!("Unknown handoff status: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffDraft {
    pub reason: EscalationReason,
    pub task_description: String,
    pub attempts: Vec<String>,
    pub analysis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffNote {
    pub id: String,
    pub author: String,
    pub note: String,
    pub status_after: HandoffStatus,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffEvent {
    pub id: String,
    pub event_type: String,
    pub actor: Option<String>,
    pub note: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HandoffContext {
    pub task_id: Option<String>,
    pub chain_id: Option<String>,
    pub original_input: Option<String>,
    pub task_status: Option<String>,
    pub task_output: Option<String>,
    pub task_steps: Vec<Value>,
    pub chain_subtasks: Vec<Value>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffPackage {
    pub id: String,
    pub reason: EscalationReason,
    pub task_description: String,
    pub attempts: Vec<String>,
    pub analysis: String,
    pub created_at: String,
    pub updated_at: String,
    pub status: HandoffStatus,
    pub assigned_to: Option<String>,
    pub context: HandoffContext,
    pub human_notes: Vec<HandoffNote>,
    pub audit_trail: Vec<HandoffEvent>,
}

pub struct EscalationDetector;

impl EscalationDetector {
    pub fn should_escalate(
        confidence: f64,
        retries: u32,
        task_type: &str,
    ) -> Option<EscalationReason> {
        if confidence < 0.3 {
            return Some(EscalationReason::LowConfidence);
        }
        if retries > 3 {
            return Some(EscalationReason::RepeatedRetries);
        }
        match task_type {
            "financial" | "payment" | "billing" => Some(EscalationReason::FinancialAction),
            "auth" | "credentials" => Some(EscalationReason::MissingCredentials),
            "system" | "desktop" | "os" => Some(EscalationReason::SystemUnavailable),
            _ => None,
        }
    }

    pub fn create_handoff(
        reason: EscalationReason,
        task: &str,
        attempts: Vec<String>,
    ) -> HandoffDraft {
        let analysis = match &reason {
            EscalationReason::LowConfidence => {
                "Agent confidence dropped below the safe threshold, so a human should review the case before execution continues.".to_string()
            }
            EscalationReason::RepeatedRetries => {
                "The agent retried this task multiple times without a reliable outcome.".to_string()
            }
            EscalationReason::FinancialAction => {
                "This task involves financial impact and requires explicit human review.".to_string()
            }
            EscalationReason::MissingCredentials => {
                "The agent is blocked on credentials or permissions that require human action.".to_string()
            }
            EscalationReason::SystemUnavailable => {
                "A required local or external system is unavailable, so human triage is needed.".to_string()
            }
            EscalationReason::UserRequest => {
                "The user explicitly requested human intervention.".to_string()
            }
        };

        HandoffDraft {
            reason,
            task_description: task.to_string(),
            attempts,
            analysis,
        }
    }
}

pub struct EscalationManager {
    db_path: PathBuf,
}

impl EscalationManager {
    pub fn new(db_path: PathBuf) -> Result<Self, String> {
        let manager = Self { db_path };
        let conn = manager.open()?;
        Self::init_db(&conn)?;
        Ok(manager)
    }

    fn open(&self) -> Result<Connection, String> {
        let conn = Connection::open(&self.db_path)
            .map_err(|e| format!("Failed to open handoff DB: {}", e))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| format!("Failed to configure handoff DB: {}", e))?;
        Self::init_db(&conn)?;
        Ok(conn)
    }

    pub fn init_db(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS human_handoffs (
                id TEXT PRIMARY KEY,
                reason TEXT NOT NULL,
                task_description TEXT NOT NULL,
                attempts_json TEXT NOT NULL,
                analysis TEXT NOT NULL,
                task_id TEXT,
                chain_id TEXT,
                original_input TEXT,
                task_status TEXT,
                task_output TEXT,
                task_steps_json TEXT NOT NULL DEFAULT '[]',
                chain_subtasks_json TEXT NOT NULL DEFAULT '[]',
                evidence_json TEXT NOT NULL DEFAULT '[]',
                status TEXT NOT NULL,
                assigned_to TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS human_handoff_notes (
                id TEXT PRIMARY KEY,
                handoff_id TEXT NOT NULL REFERENCES human_handoffs(id) ON DELETE CASCADE,
                author TEXT NOT NULL,
                note TEXT NOT NULL,
                status_after TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS human_handoff_events (
                id TEXT PRIMARY KEY,
                handoff_id TEXT NOT NULL REFERENCES human_handoffs(id) ON DELETE CASCADE,
                event_type TEXT NOT NULL,
                actor TEXT,
                note TEXT,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_human_handoffs_status ON human_handoffs(status);
            CREATE INDEX IF NOT EXISTS idx_human_handoffs_created ON human_handoffs(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_human_handoff_notes_handoff ON human_handoff_notes(handoff_id, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_human_handoff_events_handoff ON human_handoff_events(handoff_id, created_at DESC);",
        )
        .map_err(|e| format!("Failed to initialize handoff tables: {}", e))
    }

    pub fn create(
        &self,
        draft: HandoffDraft,
        task_id: Option<String>,
        chain_id: Option<String>,
        evidence: Vec<String>,
    ) -> Result<HandoffPackage, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let mut context = self.collect_context(task_id.as_deref(), chain_id.as_deref())?;
        if !evidence.is_empty() {
            context.evidence.extend(evidence);
        }

        let conn = self.open()?;
        conn.execute(
            "INSERT INTO human_handoffs (
                id, reason, task_description, attempts_json, analysis, task_id, chain_id,
                original_input, task_status, task_output, task_steps_json, chain_subtasks_json,
                evidence_json, status, assigned_to, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, NULL, ?15, ?15)",
            params![
                id,
                serde_json::to_string(&draft.reason).map_err(|e| e.to_string())?,
                draft.task_description,
                serde_json::to_string(&draft.attempts).map_err(|e| e.to_string())?,
                draft.analysis,
                context.task_id,
                context.chain_id,
                context.original_input,
                context.task_status,
                context.task_output,
                serde_json::to_string(&context.task_steps).map_err(|e| e.to_string())?,
                serde_json::to_string(&context.chain_subtasks).map_err(|e| e.to_string())?,
                serde_json::to_string(&context.evidence).map_err(|e| e.to_string())?,
                HandoffStatus::PendingHandoff.as_str(),
                now,
            ],
        )
        .map_err(|e| format!("Failed to store handoff: {}", e))?;

        self.record_event(
            &conn,
            &id,
            "pending_handoff",
            None,
            Some("Agent escalated the case with full runtime context."),
        )?;
        self.update_linked_runtime_status(
            &conn,
            context.task_id.as_deref(),
            context.chain_id.as_deref(),
            HandoffStatus::PendingHandoff.as_str(),
        )?;

        self.get(&id)?
            .ok_or_else(|| format!("Failed to reload handoff {}", id))
    }

    pub fn list(&self, status: Option<HandoffStatus>) -> Result<Vec<HandoffPackage>, String> {
        let conn = self.open()?;
        let mut list = Vec::new();
        if let Some(status) = status {
            let mut stmt = conn
                .prepare("SELECT id FROM human_handoffs WHERE status = ?1 ORDER BY created_at DESC")
                .map_err(|e| format!("Failed to query handoffs: {}", e))?;
            let ids = stmt
                .query_map(params![status.as_str()], |row| row.get::<_, String>(0))
                .map_err(|e| format!("Failed to map handoffs: {}", e))?;
            for id in ids.flatten() {
                if let Some(handoff) = self.get(&id)? {
                    list.push(handoff);
                }
            }
        } else {
            let mut stmt = conn
                .prepare("SELECT id FROM human_handoffs ORDER BY created_at DESC")
                .map_err(|e| format!("Failed to query handoffs: {}", e))?;
            let ids = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| format!("Failed to map handoffs: {}", e))?;
            for id in ids.flatten() {
                if let Some(handoff) = self.get(&id)? {
                    list.push(handoff);
                }
            }
        }
        Ok(list)
    }

    pub fn get(&self, id: &str) -> Result<Option<HandoffPackage>, String> {
        let conn = self.open()?;
        let row = conn
            .query_row(
                "SELECT id, reason, task_description, attempts_json, analysis, task_id, chain_id,
                        original_input, task_status, task_output, task_steps_json, chain_subtasks_json,
                        evidence_json, status, assigned_to, created_at, updated_at
                 FROM human_handoffs WHERE id = ?1",
                params![id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, Option<String>>(7)?,
                        row.get::<_, Option<String>>(8)?,
                        row.get::<_, Option<String>>(9)?,
                        row.get::<_, String>(10)?,
                        row.get::<_, String>(11)?,
                        row.get::<_, String>(12)?,
                        row.get::<_, String>(13)?,
                        row.get::<_, Option<String>>(14)?,
                        row.get::<_, String>(15)?,
                        row.get::<_, String>(16)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| format!("Failed to get handoff: {}", e))?;

        let Some((
            id,
            reason_json,
            task_description,
            attempts_json,
            analysis,
            task_id,
            chain_id,
            original_input,
            task_status,
            task_output,
            task_steps_json,
            chain_subtasks_json,
            evidence_json,
            status,
            assigned_to,
            created_at,
            updated_at,
        )) = row
        else {
            return Ok(None);
        };

        let reason: EscalationReason =
            serde_json::from_str(&reason_json).map_err(|e| e.to_string())?;
        let attempts: Vec<String> =
            serde_json::from_str(&attempts_json).map_err(|e| e.to_string())?;
        let task_steps: Vec<Value> =
            serde_json::from_str(&task_steps_json).map_err(|e| e.to_string())?;
        let chain_subtasks: Vec<Value> =
            serde_json::from_str(&chain_subtasks_json).map_err(|e| e.to_string())?;
        let evidence: Vec<String> =
            serde_json::from_str(&evidence_json).map_err(|e| e.to_string())?;

        Ok(Some(HandoffPackage {
            id: id.clone(),
            reason,
            task_description,
            attempts,
            analysis,
            created_at,
            updated_at,
            status: HandoffStatus::from_str(&status)?,
            assigned_to,
            context: HandoffContext {
                task_id,
                chain_id,
                original_input,
                task_status,
                task_output,
                task_steps,
                chain_subtasks,
                evidence,
            },
            human_notes: self.list_notes(&conn, &id)?,
            audit_trail: self.list_events(&conn, &id)?,
        }))
    }

    pub fn assign(
        &self,
        id: &str,
        assignee: &str,
        actor: &str,
        note: Option<&str>,
    ) -> Result<HandoffPackage, String> {
        let conn = self.open()?;
        self.ensure_exists(&conn, id)?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE human_handoffs SET status = ?2, assigned_to = ?3, updated_at = ?4 WHERE id = ?1",
            params![id, HandoffStatus::AssignedToHuman.as_str(), assignee, now],
        )
        .map_err(|e| format!("Failed to assign handoff: {}", e))?;

        if let Some(note) = note.filter(|value| !value.trim().is_empty()) {
            self.insert_note(
                &conn,
                id,
                actor,
                note,
                HandoffStatus::AssignedToHuman,
            )?;
        }
        self.record_event(
            &conn,
            id,
            "assigned_to_human",
            Some(actor),
            Some(&format!("Assigned to {}.", assignee)),
        )?;
        let linked = self.linked_ids(&conn, id)?;
        self.update_linked_runtime_status(
            &conn,
            linked.0.as_deref(),
            linked.1.as_deref(),
            HandoffStatus::AssignedToHuman.as_str(),
        )?;
        self.get(id)?
            .ok_or_else(|| format!("Failed to reload handoff {}", id))
    }

    pub fn add_note(&self, id: &str, author: &str, note: &str) -> Result<HandoffPackage, String> {
        let conn = self.open()?;
        let current = self
            .get(id)?
            .ok_or_else(|| format!("Handoff not found: {}", id))?;
        self.insert_note(&conn, id, author, note, current.status.clone())?;
        self.record_event(&conn, id, "note_added", Some(author), Some(note))?;
        conn.execute(
            "UPDATE human_handoffs SET updated_at = ?2 WHERE id = ?1",
            params![id, chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|e| format!("Failed to touch handoff: {}", e))?;
        self.get(id)?
            .ok_or_else(|| format!("Failed to reload handoff {}", id))
    }

    pub fn resume(&self, id: &str, author: &str, note: &str) -> Result<HandoffPackage, String> {
        self.transition(id, author, Some(note), HandoffStatus::Resumed, "resumed")
    }

    pub fn complete_by_human(
        &self,
        id: &str,
        author: &str,
        note: &str,
    ) -> Result<HandoffPackage, String> {
        self.transition(
            id,
            author,
            Some(note),
            HandoffStatus::CompletedByHuman,
            "completed_by_human",
        )
    }

    fn transition(
        &self,
        id: &str,
        author: &str,
        note: Option<&str>,
        status: HandoffStatus,
        event_type: &str,
    ) -> Result<HandoffPackage, String> {
        let conn = self.open()?;
        self.ensure_exists(&conn, id)?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE human_handoffs SET status = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, status.as_str(), now],
        )
        .map_err(|e| format!("Failed to update handoff: {}", e))?;

        if let Some(note) = note.filter(|value| !value.trim().is_empty()) {
            self.insert_note(&conn, id, author, note, status.clone())?;
        }
        self.record_event(&conn, id, event_type, Some(author), note)?;

        let linked = self.linked_ids(&conn, id)?;
        let runtime_status = match status {
            HandoffStatus::PendingHandoff => HandoffStatus::PendingHandoff.as_str(),
            HandoffStatus::AssignedToHuman => HandoffStatus::AssignedToHuman.as_str(),
            HandoffStatus::Resumed => "running",
            HandoffStatus::CompletedByHuman => "completed_by_human",
        };
        self.update_linked_runtime_status(
            &conn,
            linked.0.as_deref(),
            linked.1.as_deref(),
            runtime_status,
        )?;

        self.get(id)?
            .ok_or_else(|| format!("Failed to reload handoff {}", id))
    }

    fn collect_context(
        &self,
        task_id: Option<&str>,
        chain_id: Option<&str>,
    ) -> Result<HandoffContext, String> {
        let mut context = HandoffContext::default();
        let db = Database::new(Path::new(&self.db_path))
            .map_err(|e| format!("Failed to open runtime DB for handoff: {}", e))?;

        if let Some(task_id) = task_id {
            let conn = Connection::open(&self.db_path)
                .map_err(|e| format!("Failed to query task context: {}", e))?;
            let task = conn
                .query_row(
                    "SELECT input_text, status, output_text FROM tasks WHERE id = ?1",
                    params![task_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, Option<String>>(2)?,
                        ))
                    },
                )
                .optional()
                .map_err(|e| format!("Failed to load task context: {}", e))?;
            if let Some((input, status, output)) = task {
                context.task_id = Some(task_id.to_string());
                context.original_input = Some(input);
                context.task_status = Some(status);
                context.task_output = output;
                context.task_steps = db
                    .get_task_steps(task_id)
                    .map_err(|e| format!("Failed to load task steps: {}", e))?
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
            }
        }

        if let Some(chain_id) = chain_id {
            context.chain_id = Some(chain_id.to_string());
            context.chain_subtasks = db
                .get_chain_subtasks(chain_id)
                .map_err(|e| format!("Failed to load chain subtasks: {}", e))?
                .as_array()
                .cloned()
                .unwrap_or_default();
        }

        Ok(context)
    }

    fn ensure_exists(&self, conn: &Connection, id: &str) -> Result<(), String> {
        let exists: Option<String> = conn
            .query_row(
                "SELECT id FROM human_handoffs WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("Failed to check handoff existence: {}", e))?;
        if exists.is_none() {
            return Err(format!("Handoff not found: {}", id));
        }
        Ok(())
    }

    fn linked_ids(
        &self,
        conn: &Connection,
        id: &str,
    ) -> Result<(Option<String>, Option<String>), String> {
        conn.query_row(
            "SELECT task_id, chain_id FROM human_handoffs WHERE id = ?1",
            params![id],
            |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .map_err(|e| format!("Failed to load linked runtime ids: {}", e))
    }

    fn record_event(
        &self,
        conn: &Connection,
        handoff_id: &str,
        event_type: &str,
        actor: Option<&str>,
        note: Option<&str>,
    ) -> Result<(), String> {
        conn.execute(
            "INSERT INTO human_handoff_events (id, handoff_id, event_type, actor, note, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                uuid::Uuid::new_v4().to_string(),
                handoff_id,
                event_type,
                actor,
                note,
                chrono::Utc::now().to_rfc3339(),
            ],
        )
        .map_err(|e| format!("Failed to record handoff event: {}", e))?;
        Ok(())
    }

    fn insert_note(
        &self,
        conn: &Connection,
        handoff_id: &str,
        author: &str,
        note: &str,
        status_after: HandoffStatus,
    ) -> Result<(), String> {
        conn.execute(
            "INSERT INTO human_handoff_notes (id, handoff_id, author, note, status_after, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                uuid::Uuid::new_v4().to_string(),
                handoff_id,
                author,
                note,
                status_after.as_str(),
                chrono::Utc::now().to_rfc3339(),
            ],
        )
        .map_err(|e| format!("Failed to store human note: {}", e))?;
        Ok(())
    }

    fn list_notes(&self, conn: &Connection, handoff_id: &str) -> Result<Vec<HandoffNote>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT id, author, note, status_after, created_at
                 FROM human_handoff_notes
                 WHERE handoff_id = ?1
                 ORDER BY created_at ASC",
            )
            .map_err(|e| format!("Failed to query handoff notes: {}", e))?;
        let rows = stmt
            .query_map(params![handoff_id], |row| {
                Ok(HandoffNote {
                    id: row.get(0)?,
                    author: row.get(1)?,
                    note: row.get(2)?,
                    status_after: HandoffStatus::from_str(&row.get::<_, String>(3)?)
                        .map_err(|_| rusqlite::Error::ExecuteReturnedResults)?,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|e| format!("Failed to map handoff notes: {}", e))?;
        Ok(rows.flatten().collect())
    }

    fn list_events(
        &self,
        conn: &Connection,
        handoff_id: &str,
    ) -> Result<Vec<HandoffEvent>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT id, event_type, actor, note, created_at
                 FROM human_handoff_events
                 WHERE handoff_id = ?1
                 ORDER BY created_at ASC",
            )
            .map_err(|e| format!("Failed to query handoff events: {}", e))?;
        let rows = stmt
            .query_map(params![handoff_id], |row| {
                Ok(HandoffEvent {
                    id: row.get(0)?,
                    event_type: row.get(1)?,
                    actor: row.get(2)?,
                    note: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|e| format!("Failed to map handoff events: {}", e))?;
        Ok(rows.flatten().collect())
    }

    fn update_linked_runtime_status(
        &self,
        conn: &Connection,
        task_id: Option<&str>,
        chain_id: Option<&str>,
        status: &str,
    ) -> Result<(), String> {
        if let Some(task_id) = task_id {
            conn.execute(
                "UPDATE tasks SET status = ?2 WHERE id = ?1",
                params![task_id, status],
            )
            .map_err(|e| format!("Failed to update task handoff status: {}", e))?;
        }
        if let Some(chain_id) = chain_id {
            conn.execute(
                "UPDATE chains SET status = ?2 WHERE id = ?1",
                params![chain_id, status],
            )
            .map_err(|e| format!("Failed to update chain handoff status: {}", e))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::Database;

    fn temp_db_path() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn detector_still_classifies_escalation_reasons() {
        assert_eq!(
            EscalationDetector::should_escalate(0.1, 0, "general"),
            Some(EscalationReason::LowConfidence)
        );
        assert_eq!(
            EscalationDetector::should_escalate(0.8, 5, "general"),
            Some(EscalationReason::RepeatedRetries)
        );
        assert_eq!(
            EscalationDetector::should_escalate(0.8, 0, "billing"),
            Some(EscalationReason::FinancialAction)
        );
    }

    #[test]
    fn handoff_persists_context_notes_events_and_status_transitions() {
        let dir = temp_db_path();
        let db_path = dir.path().join("handoff.db");
        let db = Database::new(&db_path).unwrap();
        db.create_task_pending("task-1", "Open the billing dashboard")
            .unwrap();
        db.update_task_output("task-1", "Missing admin credentials")
            .unwrap();
        db.insert_task_step(
            "task-1",
            1,
            "navigate",
            "Opened billing settings",
            "",
            "screen",
            true,
            1200,
        )
        .unwrap();
        db.create_chain("chain-1", "Fix billing issue").unwrap();
        db.insert_chain_subtask("sub-1", "chain-1", 1, "Check billing page")
            .unwrap();

        let manager = EscalationManager::new(db_path.clone()).unwrap();
        let draft = EscalationDetector::create_handoff(
            EscalationReason::MissingCredentials,
            "Billing issue blocked on credentials",
            vec!["Tried stored tokens".to_string()],
        );

        let created = manager
            .create(
                draft,
                Some("task-1".to_string()),
                Some("chain-1".to_string()),
                vec!["403 from billing service".to_string()],
            )
            .unwrap();
        assert_eq!(created.status, HandoffStatus::PendingHandoff);
        assert_eq!(created.context.task_steps.len(), 1);
        assert_eq!(created.context.chain_subtasks.len(), 1);
        assert_eq!(created.context.evidence.len(), 1);

        let assigned = manager
            .assign(
                &created.id,
                "alice@example.com",
                "triage-bot",
                Some("Assigning to finance ops"),
            )
            .unwrap();
        assert_eq!(assigned.status, HandoffStatus::AssignedToHuman);
        assert_eq!(assigned.assigned_to.as_deref(), Some("alice@example.com"));

        let noted = manager
            .add_note(&created.id, "alice@example.com", "Credentials were refreshed.")
            .unwrap();
        assert_eq!(noted.human_notes.len(), 2);

        let resumed = manager
            .resume(
                &created.id,
                "alice@example.com",
                "Resume automation with the refreshed credentials.",
            )
            .unwrap();
        assert_eq!(resumed.status, HandoffStatus::Resumed);

        let completed = manager
            .complete_by_human(
                &created.id,
                "alice@example.com",
                "Human completed the billing change manually.",
            )
            .unwrap();
        assert_eq!(completed.status, HandoffStatus::CompletedByHuman);
        assert!(completed.audit_trail.len() >= 4);

        let reloaded = EscalationManager::new(db_path.clone())
            .unwrap()
            .get(&created.id)
            .unwrap()
            .unwrap();
        assert_eq!(reloaded.status, HandoffStatus::CompletedByHuman);
        assert_eq!(reloaded.human_notes.len(), 4);

        let conn = Connection::open(&db_path).unwrap();
        let task_status: String = conn
            .query_row(
                "SELECT status FROM tasks WHERE id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let chain_status: String = conn
            .query_row(
                "SELECT status FROM chains WHERE id = 'chain-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(task_status, "completed_by_human");
        assert_eq!(chain_status, "completed_by_human");
    }
}
