use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// B12-3: Business Automations — Natural Language Rules
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessRule {
    pub id: String,
    pub description: String,
    pub trigger_type: String,
    pub trigger_config: serde_json::Value,
    pub action: String,
    pub team: String,
    pub active: bool,
    pub created_at: String,
    pub last_triggered: Option<String>,
    pub times_triggered: u32,
}

pub struct BusinessAutomations {
    rules: Vec<BusinessRule>,
}

impl BusinessAutomations {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn add_rule(&mut self, rule: BusinessRule) {
        self.rules.push(rule);
    }

    pub fn list_rules(&self) -> &[BusinessRule] {
        &self.rules
    }

    pub fn toggle_rule(&mut self, id: &str, active: bool) {
        if let Some(rule) = self.rules.iter_mut().find(|r| r.id == id) {
            rule.active = active;
        }
    }

    pub fn delete_rule(&mut self, id: &str) {
        self.rules.retain(|r| r.id != id);
    }

    /// Parse a natural language rule description into a structured BusinessRule.
    /// Uses the LLM gateway to extract trigger type, config, action and team.
    pub async fn parse_rule(
        description: &str,
        gateway: &crate::brain::Gateway,
        settings: &crate::config::Settings,
    ) -> Result<BusinessRule, String> {
        let prompt = format!(
            "Analiza esta regla de negocio y extrae los campos en JSON:\n\
             Regla: \"{}\"\n\n\
             Responde SOLO con un JSON valido con estos campos:\n\
             {{\"trigger_type\": \"time_based|event_based|threshold_based\",\n\
              \"trigger_config\": {{}},\n\
              \"action\": \"descripcion de la accion\",\n\
              \"team\": \"marketing|sales|support|content|finance\"}}\n\n\
             Si no puedes determinar un campo, usa valores razonables por defecto.",
            description
        );

        let llm_response = gateway
            .complete(&prompt, settings)
            .await
            .map_err(|e| format!("LLM error: {}", e))?;
        let response = llm_response.content;

        // Try to parse the JSON from the response
        let json_str = extract_json_from_response(&response);
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).map_err(|e| format!("JSON parse error: {}", e))?;

        let now = chrono::Utc::now().to_rfc3339();
        Ok(BusinessRule {
            id: format!("rule_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("x")),
            description: description.to_string(),
            trigger_type: parsed["trigger_type"]
                .as_str()
                .unwrap_or("event_based")
                .to_string(),
            trigger_config: parsed["trigger_config"].clone(),
            action: parsed["action"]
                .as_str()
                .unwrap_or(description)
                .to_string(),
            team: parsed["team"].as_str().unwrap_or("sales").to_string(),
            active: true,
            created_at: now,
            last_triggered: None,
            times_triggered: 0,
        })
    }

    /// Check all time-based rules and return those that are due
    pub fn check_rules(&mut self) -> Vec<&BusinessRule> {
        let now = chrono::Utc::now();
        let mut due = Vec::new();

        for rule in &self.rules {
            if !rule.active || rule.trigger_type != "time_based" {
                continue;
            }
            // Check interval from trigger_config
            let interval_minutes = rule.trigger_config["interval_minutes"]
                .as_u64()
                .unwrap_or(60);
            let should_fire = match &rule.last_triggered {
                Some(last) => {
                    if let Ok(last_time) = chrono::DateTime::parse_from_rfc3339(last) {
                        let elapsed = now.signed_duration_since(last_time);
                        elapsed.num_minutes() as u64 >= interval_minutes
                    } else {
                        true
                    }
                }
                None => true,
            };

            if should_fire {
                due.push(rule as &BusinessRule);
            }
        }
        due
    }
}

impl Default for BusinessAutomations {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract JSON object from an LLM response that may contain markdown fences
fn extract_json_from_response(response: &str) -> String {
    // Try to find JSON between ```json ... ``` or { ... }
    if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            return response[start..=end].to_string();
        }
    }
    response.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_list_rules() {
        let mut ba = BusinessAutomations::new();
        ba.add_rule(BusinessRule {
            id: "r1".into(),
            description: "Test rule".into(),
            trigger_type: "event_based".into(),
            trigger_config: serde_json::json!({}),
            action: "do_something".into(),
            team: "sales".into(),
            active: true,
            created_at: "2026-04-04T00:00:00Z".into(),
            last_triggered: None,
            times_triggered: 0,
        });
        assert_eq!(ba.list_rules().len(), 1);
    }

    #[test]
    fn toggle_and_delete() {
        let mut ba = BusinessAutomations::new();
        ba.add_rule(BusinessRule {
            id: "r1".into(),
            description: "Test".into(),
            trigger_type: "event_based".into(),
            trigger_config: serde_json::json!({}),
            action: "act".into(),
            team: "sales".into(),
            active: true,
            created_at: "2026-04-04T00:00:00Z".into(),
            last_triggered: None,
            times_triggered: 0,
        });
        ba.toggle_rule("r1", false);
        assert!(!ba.list_rules()[0].active);
        ba.delete_rule("r1");
        assert!(ba.list_rules().is_empty());
    }
}
