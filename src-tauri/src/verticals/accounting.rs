use crate::playbooks::smart::{PlaybookVariable, SmartPlaybook, SmartStep, StepType};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountingWorkflowTransaction {
    pub date: String,
    pub description: String,
    pub amount: f64,
    pub category: Option<String>,
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
            *by_category.entry(t.category.clone()).or_insert(0.0) +=
                if t.tx_type == TransactionType::Expense {
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
        } else if desc.contains("software")
            || desc.contains("subscription")
            || desc.contains("saas")
        {
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

    pub fn run_month_close_workflow(
        &mut self,
        period: &str,
        transactions: Vec<AccountingWorkflowTransaction>,
    ) -> serde_json::Value {
        let imported: Vec<Transaction> = transactions
            .into_iter()
            .map(|draft| {
                let category = draft.category.unwrap_or_else(|| {
                    self.categorize_transaction(&draft.description)["suggested_category"]
                        .as_str()
                        .unwrap_or("uncategorized")
                        .to_string()
                });
                self.add_transaction(
                    draft.date,
                    draft.description,
                    draft.amount,
                    category,
                    draft.account,
                    draft.tx_type,
                )
            })
            .collect();

        serde_json::json!({
            "workflow": "accounting_month_close",
            "period": period,
            "imported_transactions": imported,
            "report": self.generate_report(period),
            "balance": self.get_balance(None),
        })
    }

    pub fn month_close_playbook() -> SmartPlaybook {
        SmartPlaybook {
            id: "pack-accounting-month-close".to_string(),
            name: "Accounting Month Close".to_string(),
            description: "Imports categorized transactions and emits a month-close summary."
                .to_string(),
            variables: vec![
                PlaybookVariable {
                    name: "period".to_string(),
                    var_type: "string".to_string(),
                    prompt: "Accounting period (YYYY-MM)".to_string(),
                    options: None,
                    default: None,
                },
                PlaybookVariable {
                    name: "source".to_string(),
                    var_type: "string".to_string(),
                    prompt: "Source file or workbook for transactions".to_string(),
                    options: None,
                    default: Some("transactions.csv".to_string()),
                },
            ],
            steps: vec![
                SmartStep {
                    id: "validate_source".to_string(),
                    description: "Validate the month-close source exists".to_string(),
                    step_type: StepType::Command {
                        command: "Write-Output \"Validating {source} for {period}\"".to_string(),
                    },
                },
                SmartStep {
                    id: "generate_report".to_string(),
                    description: "Generate the month-close report".to_string(),
                    step_type: StepType::Command {
                        command: "Write-Output \"Generating month close report for {period}\""
                            .to_string(),
                    },
                },
                SmartStep {
                    id: "done".to_string(),
                    description: "Finish the pack run".to_string(),
                    step_type: StepType::Done {
                        result: "Accounting month close completed".to_string(),
                    },
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playbooks::smart::{SmartPlaybookExecutionOptions, SmartPlaybookRunner};
    use std::collections::HashMap;

    #[test]
    fn month_close_workflow_imports_and_reports() {
        let mut engine = AccountingEngine::new();
        let workflow = engine.run_month_close_workflow(
            "2026-03",
            vec![
                AccountingWorkflowTransaction {
                    date: "2026-03-03".to_string(),
                    description: "Payroll March".to_string(),
                    amount: 5000.0,
                    category: None,
                    account: "operating".to_string(),
                    tx_type: TransactionType::Income,
                },
                AccountingWorkflowTransaction {
                    date: "2026-03-05".to_string(),
                    description: "Software subscription".to_string(),
                    amount: 120.0,
                    category: None,
                    account: "operating".to_string(),
                    tx_type: TransactionType::Expense,
                },
            ],
        );

        println!(
            "C18 accounting demo net={} txs={}",
            workflow["report"]["net"], workflow["report"]["transaction_count"]
        );

        assert_eq!(workflow["workflow"], "accounting_month_close");
        assert_eq!(workflow["report"]["transaction_count"], 2);
        assert_eq!(
            workflow["imported_transactions"].as_array().unwrap().len(),
            2
        );
    }

    #[tokio::test]
    async fn accounting_pack_playbook_runs_in_dry_run_mode() {
        let playbook = AccountingEngine::month_close_playbook();
        let vars = HashMap::from([
            ("period".to_string(), "2026-03".to_string()),
            ("source".to_string(), "transactions.csv".to_string()),
        ]);
        let mut runner = SmartPlaybookRunner::with_options(
            playbook,
            vars,
            SmartPlaybookExecutionOptions {
                dry_run: true,
                mocked_step_outputs: HashMap::new(),
                mocked_exit_codes: HashMap::new(),
            },
        );
        let results = runner.execute().await.unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(
            results.last().unwrap().output,
            "Accounting month close completed"
        );
    }
}
