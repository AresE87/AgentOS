use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EscrowStatus {
    Held,
    Released,
    Refunded,
    Disputed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscrowTransaction {
    pub id: String,
    pub payer: String,
    pub payee: String,
    pub amount: f64,
    pub status: EscrowStatus,
    pub task_description: String,
    pub microtask_id: Option<String>,
    pub dispute_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscrowEvent {
    pub transaction_id: String,
    pub event_type: String,
    pub detail: String,
    pub created_at: String,
}

pub struct EscrowManager {
    db_path: PathBuf,
}

impl EscrowManager {
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
            "CREATE TABLE IF NOT EXISTS escrows (
                id TEXT PRIMARY KEY,
                payer TEXT NOT NULL,
                payee TEXT NOT NULL,
                amount REAL NOT NULL,
                status TEXT NOT NULL,
                task_description TEXT NOT NULL,
                microtask_id TEXT,
                dispute_reason TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS escrow_events (
                id TEXT PRIMARY KEY,
                transaction_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                detail TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_escrow_user ON escrows(payer, payee, updated_at DESC);",
        )
        .map_err(|e| e.to_string())
    }

    pub fn create_escrow(
        &self,
        payer: String,
        payee: String,
        amount: f64,
        task_description: String,
        microtask_id: Option<String>,
    ) -> Result<EscrowTransaction, String> {
        let conn = self.open()?;
        let now = chrono::Utc::now().to_rfc3339();
        let tx = EscrowTransaction {
            id: uuid::Uuid::new_v4().to_string(),
            payer,
            payee,
            amount,
            status: EscrowStatus::Held,
            task_description,
            microtask_id,
            dispute_reason: None,
            created_at: now.clone(),
            updated_at: now,
        };
        conn.execute(
            "INSERT INTO escrows
             (id, payer, payee, amount, status, task_description, microtask_id, dispute_reason, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'held', ?5, ?6, NULL, ?7, ?7)",
            params![
                tx.id,
                tx.payer,
                tx.payee,
                tx.amount,
                tx.task_description,
                tx.microtask_id,
                tx.created_at
            ],
        )
        .map_err(|e| e.to_string())?;
        self.record_event_with_conn(&conn, &tx.id, "held", "Funds placed in escrow")?;
        Ok(tx)
    }

    pub fn release(&self, tx_id: &str) -> Result<EscrowTransaction, String> {
        let conn = self.open()?;
        let tx = self
            .get_transaction(tx_id)?
            .ok_or_else(|| "Transaction not found".to_string())?;
        if tx.status != EscrowStatus::Held {
            return Err(format!("Cannot release: status is {:?}", tx.status));
        }
        self.ensure_releaseable(&conn, &tx)?;
        conn.execute(
            "UPDATE escrows SET status = 'released', updated_at = ?2 WHERE id = ?1",
            params![tx_id, chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|e| e.to_string())?;
        self.record_event_with_conn(&conn, tx_id, "released", "Funds released to payee")?;
        self.get_transaction(tx_id)?
            .ok_or_else(|| "Transaction not found".to_string())
    }

    pub fn refund(&self, tx_id: &str) -> Result<EscrowTransaction, String> {
        let conn = self.open()?;
        let tx = self
            .get_transaction(tx_id)?
            .ok_or_else(|| "Transaction not found".to_string())?;
        if tx.status != EscrowStatus::Held && tx.status != EscrowStatus::Disputed {
            return Err(format!("Cannot refund: status is {:?}", tx.status));
        }
        conn.execute(
            "UPDATE escrows SET status = 'refunded', updated_at = ?2 WHERE id = ?1",
            params![tx_id, chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|e| e.to_string())?;
        self.record_event_with_conn(&conn, tx_id, "refunded", "Funds refunded to payer")?;
        self.get_transaction(tx_id)?
            .ok_or_else(|| "Transaction not found".to_string())
    }

    pub fn dispute(&self, tx_id: &str, reason: String) -> Result<EscrowTransaction, String> {
        let conn = self.open()?;
        let tx = self
            .get_transaction(tx_id)?
            .ok_or_else(|| "Transaction not found".to_string())?;
        if tx.status != EscrowStatus::Held {
            return Err(format!("Cannot dispute: status is {:?}", tx.status));
        }
        conn.execute(
            "UPDATE escrows SET status = 'disputed', dispute_reason = ?2, updated_at = ?3 WHERE id = ?1",
            params![tx_id, reason, chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|e| e.to_string())?;
        self.record_event_with_conn(&conn, tx_id, "disputed", "Escrow moved to dispute")?;
        self.get_transaction(tx_id)?
            .ok_or_else(|| "Transaction not found".to_string())
    }

    pub fn get_transaction(&self, tx_id: &str) -> Result<Option<EscrowTransaction>, String> {
        let conn = self.open()?;
        conn.query_row(
            "SELECT id, payer, payee, amount, status, task_description, microtask_id, dispute_reason, created_at, updated_at
             FROM escrows WHERE id = ?1",
            params![tx_id],
            map_transaction,
        )
        .optional()
        .map_err(|e| e.to_string())
    }

    pub fn list_transactions(&self, user_id: Option<&str>) -> Result<Vec<EscrowTransaction>, String> {
        let conn = self.open()?;
        let sql = if user_id.is_some() {
            "SELECT id, payer, payee, amount, status, task_description, microtask_id, dispute_reason, created_at, updated_at
             FROM escrows WHERE payer = ?1 OR payee = ?1 ORDER BY updated_at DESC"
        } else {
            "SELECT id, payer, payee, amount, status, task_description, microtask_id, dispute_reason, created_at, updated_at
             FROM escrows ORDER BY updated_at DESC"
        };
        let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
        if let Some(user_id) = user_id {
            let txs = stmt
                .query_map(params![user_id], map_transaction)
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            Ok(txs)
        } else {
            let txs = stmt
                .query_map([], map_transaction)
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            Ok(txs)
        }
    }

    pub fn history(&self, tx_id: &str) -> Result<Vec<EscrowEvent>, String> {
        let conn = self.open()?;
        let mut stmt = conn
            .prepare(
                "SELECT transaction_id, event_type, detail, created_at
                 FROM escrow_events
                 WHERE transaction_id = ?1
                 ORDER BY created_at ASC",
            )
            .map_err(|e| e.to_string())?;
        let events = stmt
            .query_map(params![tx_id], |row| {
            Ok(EscrowEvent {
                transaction_id: row.get(0)?,
                event_type: row.get(1)?,
                detail: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
        Ok(events)
    }

    fn ensure_releaseable(&self, conn: &Connection, tx: &EscrowTransaction) -> Result<(), String> {
        if let Some(microtask_id) = &tx.microtask_id {
            let row: Option<(String, Option<String>)> = conn
                .query_row(
                    "SELECT status, result FROM microtasks WHERE id = ?1",
                    params![microtask_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            match row {
                Some((status, result)) if status == "completed" && result.is_some() => Ok(()),
                Some((status, _)) => Err(format!(
                    "Linked microtask '{}' is not releasable from status '{}'",
                    microtask_id, status
                )),
                None => Err(format!("Linked microtask '{}' not found", microtask_id)),
            }
        } else {
            Ok(())
        }
    }

    fn record_event_with_conn(
        &self,
        conn: &Connection,
        tx_id: &str,
        event_type: &str,
        detail: &str,
    ) -> Result<(), String> {
        conn.execute(
            "INSERT INTO escrow_events (id, transaction_id, event_type, detail, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                uuid::Uuid::new_v4().to_string(),
                tx_id,
                event_type,
                detail,
                chrono::Utc::now().to_rfc3339()
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn map_transaction(row: &rusqlite::Row<'_>) -> rusqlite::Result<EscrowTransaction> {
    Ok(EscrowTransaction {
        id: row.get(0)?,
        payer: row.get(1)?,
        payee: row.get(2)?,
        amount: row.get(3)?,
        status: match row.get::<_, String>(4)?.as_str() {
            "released" => EscrowStatus::Released,
            "refunded" => EscrowStatus::Refunded,
            "disputed" => EscrowStatus::Disputed,
            _ => EscrowStatus::Held,
        },
        task_description: row.get(5)?,
        microtask_id: row.get(6)?,
        dispute_reason: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::economy::microtasks::MicrotaskMarket;
    use tempfile::tempdir;

    #[test]
    fn escrow_release_requires_completed_linked_microtask() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("market.db");
        let market = MicrotaskMarket::new(db_path.clone()).unwrap();
        let escrow = EscrowManager::new(db_path).unwrap();

        let task = market
            .post_task(
                "Verify emails".to_string(),
                "Clean CRM list".to_string(),
                15.0,
                None,
                "poster-1".to_string(),
            )
            .unwrap();
        let tx = escrow
            .create_escrow(
                "poster-1".to_string(),
                "agent-2".to_string(),
                15.0,
                "Clean CRM list".to_string(),
                Some(task.id.clone()),
            )
            .unwrap();

        assert!(escrow.release(&tx.id).is_err());

        market.claim_task(&task.id, "agent-2".to_string()).unwrap();
        market
            .complete_task(&task.id, "done".to_string())
            .unwrap();
        let released = escrow.release(&tx.id).unwrap();
        assert_eq!(released.status, EscrowStatus::Released);
        assert!(escrow.history(&tx.id).unwrap().len() >= 2);
    }
}
