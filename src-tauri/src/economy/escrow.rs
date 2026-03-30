// ── R145: Escrow System ──────────────────────────────────────────
use serde::{Deserialize, Serialize};

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
    pub dispute_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct EscrowManager {
    transactions: Vec<EscrowTransaction>,
}

impl EscrowManager {
    pub fn new() -> Self {
        Self { transactions: Vec::new() }
    }

    pub fn create_escrow(&mut self, payer: String, payee: String, amount: f64, task_description: String) -> EscrowTransaction {
        let now = chrono::Utc::now().to_rfc3339();
        let tx = EscrowTransaction {
            id: uuid::Uuid::new_v4().to_string(),
            payer,
            payee,
            amount,
            status: EscrowStatus::Held,
            task_description,
            dispute_reason: None,
            created_at: now.clone(),
            updated_at: now,
        };
        self.transactions.push(tx.clone());
        tx
    }

    pub fn release(&mut self, tx_id: &str) -> Result<EscrowTransaction, String> {
        let tx = self.transactions.iter_mut().find(|t| t.id == tx_id)
            .ok_or_else(|| "Transaction not found".to_string())?;
        if tx.status != EscrowStatus::Held {
            return Err(format!("Cannot release: status is {:?}", tx.status));
        }
        tx.status = EscrowStatus::Released;
        tx.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(tx.clone())
    }

    pub fn refund(&mut self, tx_id: &str) -> Result<EscrowTransaction, String> {
        let tx = self.transactions.iter_mut().find(|t| t.id == tx_id)
            .ok_or_else(|| "Transaction not found".to_string())?;
        if tx.status != EscrowStatus::Held && tx.status != EscrowStatus::Disputed {
            return Err(format!("Cannot refund: status is {:?}", tx.status));
        }
        tx.status = EscrowStatus::Refunded;
        tx.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(tx.clone())
    }

    pub fn dispute(&mut self, tx_id: &str, reason: String) -> Result<EscrowTransaction, String> {
        let tx = self.transactions.iter_mut().find(|t| t.id == tx_id)
            .ok_or_else(|| "Transaction not found".to_string())?;
        if tx.status != EscrowStatus::Held {
            return Err(format!("Cannot dispute: status is {:?}", tx.status));
        }
        tx.status = EscrowStatus::Disputed;
        tx.dispute_reason = Some(reason);
        tx.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(tx.clone())
    }

    pub fn get_transaction(&self, tx_id: &str) -> Option<&EscrowTransaction> {
        self.transactions.iter().find(|t| t.id == tx_id)
    }

    pub fn list_transactions(&self, user_id: Option<&str>) -> Vec<&EscrowTransaction> {
        self.transactions.iter().filter(|t| {
            user_id.map_or(true, |uid| t.payer == uid || t.payee == uid)
        }).collect()
    }
}
