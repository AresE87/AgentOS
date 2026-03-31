use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A purchase request submitted for procurement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseRequest {
    pub id: String,
    pub item: String,
    pub vendor: String,
    pub amount: f64,
    pub currency: String,
    pub justification: String,
    pub status: String,
    pub requester: String,
    pub created_at: String,
}

/// Summary of procurement spending
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendSummary {
    pub total_spend: f64,
    pub by_category: HashMap<String, f64>,
    pub by_vendor: HashMap<String, f64>,
    pub pending_amount: f64,
}

/// Autonomous procurement engine (R117)
pub struct AutoProcurement {
    requests: Vec<PurchaseRequest>,
    /// Auto-approval threshold in the default currency
    approval_threshold: f64,
}

impl AutoProcurement {
    pub fn new() -> Self {
        Self {
            requests: Vec::new(),
            approval_threshold: 500.0,
        }
    }

    /// Submit a new purchase request
    pub fn submit_request(&mut self, mut req: PurchaseRequest) -> PurchaseRequest {
        if req.id.is_empty() {
            req.id = uuid::Uuid::new_v4().to_string();
        }
        if req.status.is_empty() {
            req.status = "pending".to_string();
        }
        if req.created_at.is_empty() {
            req.created_at = chrono::Utc::now().to_rfc3339();
        }
        self.requests.push(req.clone());
        tracing::info!(
            "Procurement request submitted: {} (${:.2})",
            req.item,
            req.amount
        );
        req
    }

    /// Auto-approve a request if it is under the threshold
    pub fn auto_approve(&mut self, id: &str) -> Result<bool, String> {
        let req = self
            .requests
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or_else(|| format!("Request not found: {}", id))?;

        if req.status != "pending" {
            return Err(format!(
                "Request {} is not pending (status: {})",
                id, req.status
            ));
        }

        if req.amount <= self.approval_threshold {
            req.status = "approved".to_string();
            tracing::info!(
                "Auto-approved procurement: {} (${:.2} <= threshold ${:.2})",
                req.item,
                req.amount,
                self.approval_threshold
            );
            Ok(true)
        } else {
            tracing::info!(
                "Procurement requires manual approval: {} (${:.2} > threshold ${:.2})",
                req.item,
                req.amount,
                self.approval_threshold
            );
            Ok(false)
        }
    }

    /// List all purchase requests
    pub fn list_requests(&self) -> Vec<PurchaseRequest> {
        self.requests.clone()
    }

    /// Get a spending summary
    pub fn get_spend_summary(&self) -> SpendSummary {
        let mut by_vendor: HashMap<String, f64> = HashMap::new();
        let mut by_category: HashMap<String, f64> = HashMap::new();
        let mut total_spend = 0.0;
        let mut pending_amount = 0.0;

        for req in &self.requests {
            match req.status.as_str() {
                "approved" | "ordered" => {
                    total_spend += req.amount;
                    *by_vendor.entry(req.vendor.clone()).or_insert(0.0) += req.amount;
                    *by_category.entry(req.item.clone()).or_insert(0.0) += req.amount;
                }
                "pending" => {
                    pending_amount += req.amount;
                }
                _ => {}
            }
        }

        SpendSummary {
            total_spend,
            by_category,
            by_vendor,
            pending_amount,
        }
    }
}
