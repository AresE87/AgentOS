use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
    pub escrow_tx_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct MicrotaskMarket {
    db_path: PathBuf,
}

impl MicrotaskMarket {
    pub fn new(db_path: PathBuf) -> Result<Self, String> {
        let market = Self { db_path };
        let conn = market.open()?;
        Self::ensure_tables(&conn)?;
        Ok(market)
    }

    fn open(&self) -> Result<Connection, String> {
        let conn = Connection::open(&self.db_path).map_err(|e| e.to_string())?;
        Self::ensure_tables(&conn)?;
        Ok(conn)
    }

    pub fn ensure_tables(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS microtasks (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                reward_amount REAL NOT NULL,
                deadline TEXT,
                status TEXT NOT NULL,
                assigned_to TEXT,
                poster_id TEXT NOT NULL,
                result TEXT,
                escrow_tx_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_microtasks_status ON microtasks(status, created_at DESC);",
        )
        .map_err(|e| e.to_string())
    }

    pub fn post_task(
        &self,
        title: String,
        description: String,
        reward_amount: f64,
        deadline: Option<String>,
        poster_id: String,
    ) -> Result<Microtask, String> {
        let conn = self.open()?;
        let now = chrono::Utc::now().to_rfc3339();
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO microtasks
             (id, title, description, reward_amount, deadline, status, assigned_to, poster_id, result, escrow_tx_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'available', NULL, ?6, NULL, NULL, ?7, ?7)",
            params![id, title, description, reward_amount, deadline, poster_id, now],
        )
        .map_err(|e| e.to_string())?;
        self.get_task(&id)?
            .ok_or_else(|| "Failed to reload microtask".to_string())
    }

    pub fn claim_task(&self, task_id: &str, agent_id: String) -> Result<Microtask, String> {
        let conn = self.open()?;
        let task = self
            .get_task(task_id)?
            .ok_or_else(|| "Task not found".to_string())?;
        if task.status != MicrotaskStatus::Available {
            return Err("Task is not available".to_string());
        }
        conn.execute(
            "UPDATE microtasks SET status = 'claimed', assigned_to = ?2, updated_at = ?3 WHERE id = ?1",
            params![task_id, agent_id, chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|e| e.to_string())?;
        self.get_task(task_id)?
            .ok_or_else(|| "Task not found".to_string())
    }

    pub fn complete_task(&self, task_id: &str, result: String) -> Result<Microtask, String> {
        let conn = self.open()?;
        let task = self
            .get_task(task_id)?
            .ok_or_else(|| "Task not found".to_string())?;
        if task.status != MicrotaskStatus::Claimed && task.status != MicrotaskStatus::InProgress {
            return Err("Task is not in a completable state".to_string());
        }
        conn.execute(
            "UPDATE microtasks SET status = 'completed', result = ?2, updated_at = ?3 WHERE id = ?1",
            params![task_id, result, chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|e| e.to_string())?;
        self.get_task(task_id)?
            .ok_or_else(|| "Task not found".to_string())
    }

    pub fn attach_escrow(&self, task_id: &str, escrow_tx_id: &str) -> Result<(), String> {
        let conn = self.open()?;
        conn.execute(
            "UPDATE microtasks SET escrow_tx_id = ?2, updated_at = ?3 WHERE id = ?1",
            params![task_id, escrow_tx_id, chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_available(&self) -> Result<Vec<Microtask>, String> {
        let conn = self.open()?;
        self.query_tasks(
            &conn,
            "SELECT id, title, description, reward_amount, deadline, status, assigned_to, poster_id, result, escrow_tx_id, created_at, updated_at
             FROM microtasks
             WHERE status = 'available'
             ORDER BY created_at DESC",
            params![],
        )
    }

    pub fn list_my_tasks(&self, agent_id: &str) -> Result<Vec<Microtask>, String> {
        let conn = self.open()?;
        self.query_tasks(
            &conn,
            "SELECT id, title, description, reward_amount, deadline, status, assigned_to, poster_id, result, escrow_tx_id, created_at, updated_at
             FROM microtasks
             WHERE assigned_to = ?1 OR poster_id = ?1
             ORDER BY updated_at DESC",
            params![agent_id],
        )
    }

    pub fn get_task(&self, task_id: &str) -> Result<Option<Microtask>, String> {
        let conn = self.open()?;
        conn.query_row(
            "SELECT id, title, description, reward_amount, deadline, status, assigned_to, poster_id, result, escrow_tx_id, created_at, updated_at
             FROM microtasks WHERE id = ?1",
            params![task_id],
            map_task,
        )
        .optional()
        .map_err(|e| e.to_string())
    }

    fn query_tasks<P>(&self, conn: &Connection, sql: &str, params: P) -> Result<Vec<Microtask>, String>
    where
        P: rusqlite::Params,
    {
        let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
        let tasks = stmt
            .query_map(params, map_task)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(tasks)
    }
}

fn map_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<Microtask> {
    Ok(Microtask {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        reward_amount: row.get(3)?,
        deadline: row.get(4)?,
        status: match row.get::<_, String>(5)?.as_str() {
            "claimed" => MicrotaskStatus::Claimed,
            "in_progress" => MicrotaskStatus::InProgress,
            "completed" => MicrotaskStatus::Completed,
            "cancelled" => MicrotaskStatus::Cancelled,
            _ => MicrotaskStatus::Available,
        },
        assigned_to: row.get(6)?,
        poster_id: row.get(7)?,
        result: row.get(8)?,
        escrow_tx_id: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn microtask_market_persists_publish_claim_complete() {
        let dir = tempdir().unwrap();
        let market = MicrotaskMarket::new(dir.path().join("microtasks.db")).unwrap();

        let task = market
            .post_task(
                "Classify invoices".to_string(),
                "Tag invoice PDFs by vendor".to_string(),
                25.0,
                None,
                "poster-1".to_string(),
            )
            .unwrap();
        let claimed = market.claim_task(&task.id, "agent-1".to_string()).unwrap();
        let completed = market
            .complete_task(&task.id, "Tagged 12 invoices".to_string())
            .unwrap();

        assert_eq!(claimed.status, MicrotaskStatus::Claimed);
        assert_eq!(completed.status, MicrotaskStatus::Completed);
        assert_eq!(market.list_available().unwrap().len(), 0);
        assert_eq!(market.list_my_tasks("agent-1").unwrap().len(), 1);
    }
}
