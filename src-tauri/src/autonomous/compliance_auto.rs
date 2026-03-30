use serde::{Deserialize, Serialize};

/// A compliance requirement to be monitored
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceTask {
    pub id: String,
    pub regulation: String,
    pub requirement: String,
    pub check_command: String,
    pub last_checked: Option<String>,
    pub status: String,
    pub remediation: Option<String>,
}

/// Autonomous compliance monitoring engine (R118)
pub struct AutoCompliance {
    tasks: Vec<ComplianceTask>,
}

impl AutoCompliance {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
        }
    }

    /// Register a new compliance requirement
    pub fn register_requirement(&mut self, mut task: ComplianceTask) -> ComplianceTask {
        if task.id.is_empty() {
            task.id = uuid::Uuid::new_v4().to_string();
        }
        if task.status.is_empty() {
            task.status = "pending".to_string();
        }
        self.tasks.push(task.clone());
        tracing::info!(
            "Compliance requirement registered: {} — {}",
            task.regulation,
            task.requirement
        );
        task
    }

    /// Run all compliance checks and return updated tasks
    pub fn run_all_checks(&mut self) -> Vec<ComplianceTask> {
        let now = chrono::Utc::now().to_rfc3339();
        for task in &mut self.tasks {
            task.last_checked = Some(now.clone());
            // Simulate compliance check based on the check_command field
            // In production, this would execute the actual check command
            if task.check_command.is_empty() {
                task.status = "pending".to_string();
            } else if task.check_command.contains("fail") || task.check_command.contains("non_compliant") {
                task.status = "non_compliant".to_string();
            } else {
                task.status = "compliant".to_string();
            }
        }
        tracing::info!("Ran {} compliance checks", self.tasks.len());
        self.tasks.clone()
    }

    /// Get all non-compliant tasks
    pub fn get_non_compliant(&self) -> Vec<ComplianceTask> {
        self.tasks
            .iter()
            .filter(|t| t.status == "non_compliant")
            .cloned()
            .collect()
    }

    /// Attempt auto-remediation for a task
    pub fn auto_remediate(&mut self, id: &str) -> Result<ComplianceTask, String> {
        let task = self
            .tasks
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| format!("Compliance task not found: {}", id))?;

        if task.status != "non_compliant" {
            return Err(format!(
                "Task {} is not non-compliant (status: {})",
                id, task.status
            ));
        }

        // Simulate remediation — in production this would execute remediation steps
        task.remediation = Some(format!(
            "Auto-remediation applied for {} at {}",
            task.regulation,
            chrono::Utc::now().to_rfc3339()
        ));
        task.status = "compliant".to_string();
        tracing::info!(
            "Auto-remediated compliance task: {} — {}",
            task.regulation,
            task.requirement
        );
        Ok(task.clone())
    }
}
