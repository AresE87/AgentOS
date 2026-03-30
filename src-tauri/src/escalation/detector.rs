use serde::{Deserialize, Serialize};

/// R93: Reason for escalation to a human
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EscalationReason {
    LowConfidence,
    RepeatedRetries,
    FinancialAction,
    MissingCredentials,
    SystemUnavailable,
    UserRequest,
}

/// R93: Package of information for human handoff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffPackage {
    pub id: String,
    pub reason: EscalationReason,
    pub task_description: String,
    pub attempts: Vec<String>,
    pub analysis: String,
    pub created_at: String,
    pub status: String, // "pending", "handled", "resolved"
}

/// R93: Detects when escalation is needed
pub struct EscalationDetector;

impl EscalationDetector {
    /// Determine if escalation is needed based on confidence, retries, and task type
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
            _ => None,
        }
    }

    /// Create a handoff package from an escalation reason
    pub fn create_handoff(
        reason: EscalationReason,
        task: &str,
        attempts: Vec<String>,
    ) -> HandoffPackage {
        let analysis = match &reason {
            EscalationReason::LowConfidence => "Agent confidence is too low to proceed safely.".into(),
            EscalationReason::RepeatedRetries => "Task has been retried multiple times without success.".into(),
            EscalationReason::FinancialAction => "Financial action requires human approval.".into(),
            EscalationReason::MissingCredentials => "Required credentials are not available.".into(),
            EscalationReason::SystemUnavailable => "A required system is currently unavailable.".into(),
            EscalationReason::UserRequest => "User explicitly requested human assistance.".into(),
        };

        HandoffPackage {
            id: uuid::Uuid::new_v4().to_string(),
            reason,
            task_description: task.to_string(),
            attempts,
            analysis,
            created_at: chrono::Utc::now().to_rfc3339(),
            status: "pending".into(),
        }
    }
}

/// R93: Manages escalation queue
pub struct EscalationManager {
    escalations: Vec<HandoffPackage>,
}

impl EscalationManager {
    pub fn new() -> Self {
        Self {
            escalations: Vec::new(),
        }
    }

    pub fn add(&mut self, pkg: HandoffPackage) {
        self.escalations.push(pkg);
    }

    pub fn list_pending(&self) -> Vec<&HandoffPackage> {
        self.escalations.iter().filter(|e| e.status == "pending").collect()
    }

    pub fn resolve(&mut self, id: &str) -> Result<(), String> {
        let esc = self.escalations.iter_mut()
            .find(|e| e.id == id)
            .ok_or_else(|| format!("Escalation not found: {}", id))?;
        esc.status = "resolved".into();
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&HandoffPackage> {
        self.escalations.iter().find(|e| e.id == id)
    }

    pub fn get_all(&self) -> &[HandoffPackage] {
        &self.escalations
    }
}
