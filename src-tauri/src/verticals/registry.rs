use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustryVertical {
    pub id: String,
    pub name: String,
    pub description: String,
    pub specialists: Vec<String>,
    pub playbooks: Vec<String>,
    pub workflow_command: String,
    pub settings_keys: Vec<String>,
    pub status: String,
    pub system_prompt_additions: String,
}

/// Registry for industry vertical packages.
pub struct VerticalRegistry {
    verticals: Vec<IndustryVertical>,
    active_id: Option<String>,
}

impl VerticalRegistry {
    pub fn new() -> Self {
        let verticals = vec![
            IndustryVertical {
                id: "accounting".into(),
                name: "Accounting Operations".into(),
                description:
                    "Month-close and transaction categorization pack backed by the accounting engine."
                        .into(),
                specialists: vec![
                    "Accounting Engine".into(),
                    "Expense Categorizer".into(),
                    "Month Close Reporter".into(),
                ],
                playbooks: vec!["pack-accounting-month-close".into()],
                workflow_command: "vertical_run_workflow(accounting)".into(),
                settings_keys: vec!["plan_type".into()],
                status: "real".into(),
                system_prompt_additions:
                    "You are operating in an accounting context. Keep auditability and categorization accuracy explicit."
                        .into(),
            },
            IndustryVertical {
                id: "legal".into(),
                name: "Legal Intake".into(),
                description:
                    "Case-intake and document-analysis pack backed by the legal suite.".into(),
                specialists: vec![
                    "Case Intake Agent".into(),
                    "Document Review Agent".into(),
                    "Case Management Agent".into(),
                ],
                playbooks: vec!["pack-legal-case-intake".into()],
                workflow_command: "vertical_run_workflow(legal)".into(),
                settings_keys: vec!["plan_type".into()],
                status: "real".into(),
                system_prompt_additions:
                    "You are operating in a legal context. Treat outputs as support material, not legal advice."
                        .into(),
            },
        ];

        Self {
            verticals,
            active_id: None,
        }
    }

    /// List all available industry verticals.
    pub fn list_verticals(&self) -> Vec<IndustryVertical> {
        self.verticals.clone()
    }

    /// Get a specific vertical by ID.
    pub fn get_vertical(&self, id: &str) -> Option<IndustryVertical> {
        self.verticals.iter().find(|v| v.id == id).cloned()
    }

    /// Activate a vertical by ID.
    pub fn activate_vertical(&mut self, id: &str) -> Result<IndustryVertical, String> {
        if let Some(v) = self.verticals.iter().find(|v| v.id == id) {
            self.active_id = Some(id.to_string());
            Ok(v.clone())
        } else {
            Err(format!("Vertical not found: {}", id))
        }
    }

    /// Get the currently active vertical, if any.
    pub fn get_active(&self) -> Option<IndustryVertical> {
        self.active_id
            .as_ref()
            .and_then(|id| self.verticals.iter().find(|v| v.id == *id).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_only_exposes_real_packs() {
        let registry = VerticalRegistry::new();
        let ids: Vec<String> = registry
            .list_verticals()
            .into_iter()
            .map(|v| v.id)
            .collect();
        assert_eq!(ids, vec!["accounting".to_string(), "legal".to_string()]);
    }
}
