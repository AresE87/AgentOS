use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Action the support system should take for a ticket
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum SupportAction {
    AutoReply(String),
    Escalate,
    RequestInfo(String),
    Close,
}

/// A customer support ticket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportTicket {
    pub id: String,
    pub customer: String,
    pub issue: String,
    pub priority: String,
    pub status: String,
    pub auto_response: Option<String>,
    pub created_at: String,
}

/// Autonomous customer support engine (R116)
pub struct AutoSupport {
    tickets: Vec<SupportTicket>,
    /// Simple keyword-based knowledge base for auto-replies
    knowledge_base: HashMap<String, String>,
}

impl AutoSupport {
    pub fn new() -> Self {
        let mut kb = HashMap::new();
        kb.insert(
            "password".to_string(),
            "To reset your password, go to Settings > Security > Reset Password. You will receive an email with a reset link.".to_string(),
        );
        kb.insert(
            "billing".to_string(),
            "For billing inquiries, please check your invoice under Settings > Billing. If the issue persists, we will escalate to our billing team.".to_string(),
        );
        kb.insert(
            "login".to_string(),
            "If you cannot log in, try clearing your browser cache and cookies, then attempt again. Ensure your account is not locked.".to_string(),
        );
        kb.insert(
            "refund".to_string(),
            "Refund requests are processed within 5-7 business days. Please provide your order number for faster processing.".to_string(),
        );
        kb.insert(
            "install".to_string(),
            "For installation help, please visit our documentation at docs.agentos.ai/install. Ensure you meet the minimum system requirements.".to_string(),
        );

        Self {
            tickets: Vec::new(),
            knowledge_base: kb,
        }
    }

    /// Process a ticket and determine the best action
    pub fn process_ticket(&mut self, mut ticket: SupportTicket) -> SupportAction {
        // Assign id if missing
        if ticket.id.is_empty() {
            ticket.id = uuid::Uuid::new_v4().to_string();
        }
        if ticket.created_at.is_empty() {
            ticket.created_at = chrono::Utc::now().to_rfc3339();
        }

        let action = if self.should_escalate(&ticket) {
            ticket.status = "escalated".to_string();
            SupportAction::Escalate
        } else {
            let response = self.generate_response(&ticket);
            if response.contains("could not find") {
                ticket.status = "in_progress".to_string();
                SupportAction::RequestInfo(
                    "Could you provide more details about your issue?".to_string(),
                )
            } else {
                ticket.status = "resolved".to_string();
                ticket.auto_response = Some(response.clone());
                SupportAction::AutoReply(response)
            }
        };

        self.tickets.push(ticket);
        action
    }

    /// Generate a response by searching the knowledge base
    pub fn generate_response(&self, ticket: &SupportTicket) -> String {
        let issue_lower = ticket.issue.to_lowercase();
        for (keyword, response) in &self.knowledge_base {
            if issue_lower.contains(keyword) {
                return response.clone();
            }
        }
        format!(
            "We could not find an automatic answer for your issue. A support agent will review your ticket shortly. Reference: {}",
            if ticket.id.is_empty() { "pending" } else { &ticket.id }
        )
    }

    /// Determine if a ticket should be escalated to a human
    pub fn should_escalate(&self, ticket: &SupportTicket) -> bool {
        let priority = ticket.priority.to_lowercase();
        if priority == "critical" {
            return true;
        }
        let issue_lower = ticket.issue.to_lowercase();
        let escalation_keywords = [
            "legal",
            "lawsuit",
            "fraud",
            "security breach",
            "data leak",
            "urgent",
            "emergency",
            "ceo",
            "executive",
        ];
        escalation_keywords
            .iter()
            .any(|kw| issue_lower.contains(kw))
    }

    /// List all tickets
    pub fn list_tickets(&self) -> Vec<SupportTicket> {
        self.tickets.clone()
    }

    /// Resolve a ticket by id
    pub fn resolve_ticket(&mut self, id: &str) -> Result<SupportTicket, String> {
        let ticket = self
            .tickets
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| format!("Ticket not found: {}", id))?;
        ticket.status = "resolved".to_string();
        Ok(ticket.clone())
    }

    /// Get support stats
    pub fn stats(&self) -> serde_json::Value {
        let total = self.tickets.len();
        let resolved = self
            .tickets
            .iter()
            .filter(|t| t.status == "resolved")
            .count();
        let escalated = self
            .tickets
            .iter()
            .filter(|t| t.status == "escalated")
            .count();
        let open = self
            .tickets
            .iter()
            .filter(|t| t.status == "open" || t.status == "in_progress")
            .count();
        let auto_resolved = self
            .tickets
            .iter()
            .filter(|t| t.auto_response.is_some() && t.status == "resolved")
            .count();

        serde_json::json!({
            "total": total,
            "resolved": resolved,
            "escalated": escalated,
            "open": open,
            "auto_resolved": auto_resolved,
            "auto_resolution_rate": if total > 0 { (auto_resolved as f64 / total as f64 * 100.0).round() } else { 0.0 },
        })
    }
}
