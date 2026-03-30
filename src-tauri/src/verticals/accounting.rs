use serde::{Deserialize, Serialize};

/// R133 — Accounting vertical module.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub date: String,
    pub description: String,
    pub amount: f64,
    pub category: String,
    pub account: String,
    pub tx_type: TransactionType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    Income,
    Expense,
    Transfer,
}

pub struct AccountingEngine {
    transactions: Vec<Transaction>,
    next_id: u64,
}

impl AccountingEngine {
    pub fn new() -> Self {
        Self {
            transactions: Vec::new(),
            next_id: 1,
        }
    }

    /// Add a new transaction.
    pub fn add_transaction(
        &mut self,
        date: String,
        description: String,
        amount: f64,
        category: String,
        account: String,
        tx_type: TransactionType,
    ) -> Transaction {
        let tx = Transaction {
            id: format!("tx_{}", self.next_id),
            date,
            description,
            amount,
            category,
            account,
            tx_type,
        };
        self.next_id += 1;
        self.transactions.push(tx.clone());
        tx
    }

    /// Get the balance for a specific account (or all accounts).
    pub fn get_balance(&self, account: Option<&str>) -> serde_json::Value {
        let filtered: Vec<&Transaction> = self
            .transactions
            .iter()
            .filter(|t| account.map_or(true, |a| t.account == a))
            .collect();

        let income: f64 = filtered
            .iter()
            .filter(|t| t.tx_type == TransactionType::Income)
            .map(|t| t.amount)
            .sum();
        let expenses: f64 = filtered
            .iter()
            .filter(|t| t.tx_type == TransactionType::Expense)
            .map(|t| t.amount)
            .sum();

        serde_json::json!({
            "account": account.unwrap_or("all"),
            "total_income": income,
            "total_expenses": expenses,
            "net_balance": income - expenses,
            "transaction_count": filtered.len(),
        })
    }

    /// Generate a report for a given period (YYYY-MM format).
    pub fn generate_report(&self, period: &str) -> serde_json::Value {
        let filtered: Vec<&Transaction> = self
            .transactions
            .iter()
            .filter(|t| t.date.starts_with(period))
            .collect();

        let mut by_category: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();
        for t in &filtered {
            *by_category.entry(t.category.clone()).or_insert(0.0) += if t.tx_type == TransactionType::Expense {
                -t.amount
            } else {
                t.amount
            };
        }

        let income: f64 = filtered
            .iter()
            .filter(|t| t.tx_type == TransactionType::Income)
            .map(|t| t.amount)
            .sum();
        let expenses: f64 = filtered
            .iter()
            .filter(|t| t.tx_type == TransactionType::Expense)
            .map(|t| t.amount)
            .sum();

        serde_json::json!({
            "period": period,
            "total_income": income,
            "total_expenses": expenses,
            "net": income - expenses,
            "by_category": by_category,
            "transaction_count": filtered.len(),
        })
    }

    /// Auto-categorize a transaction based on description keywords.
    pub fn categorize_transaction(&self, description: &str) -> serde_json::Value {
        let desc = description.to_lowercase();
        let category = if desc.contains("salary") || desc.contains("payroll") {
            "payroll"
        } else if desc.contains("rent") || desc.contains("lease") {
            "rent"
        } else if desc.contains("utility") || desc.contains("electric") || desc.contains("water") {
            "utilities"
        } else if desc.contains("travel") || desc.contains("flight") || desc.contains("hotel") {
            "travel"
        } else if desc.contains("software") || desc.contains("subscription") || desc.contains("saas") {
            "software"
        } else if desc.contains("food") || desc.contains("meal") || desc.contains("restaurant") {
            "meals"
        } else if desc.contains("marketing") || desc.contains("ads") || desc.contains("promotion") {
            "marketing"
        } else {
            "uncategorized"
        };

        serde_json::json!({
            "description": description,
            "suggested_category": category,
            "confidence": if category == "uncategorized" { 0.3 } else { 0.85 },
        })
    }
}
