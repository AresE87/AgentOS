use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// B12-2: Inter-Team Orchestration -- Cross-team event bus
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossTeamEvent {
    pub id: String,
    pub from_team: String,
    pub to_team: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub created_at: String,
    pub processed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationRule {
    pub id: String,
    pub trigger_team: String,
    pub trigger_event: String,
    pub target_team: String,
    pub target_action: String,
    pub description: String,
    pub active: bool,
}

/// A triggered action produced by process_pending: the original event plus a
/// human-readable task description that can be fed to the agent loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggeredAction {
    pub event: CrossTeamEvent,
    pub rule_id: String,
    pub target_team: String,
    pub target_action: String,
    pub task_description: String,
}

pub struct CrossTeamOrchestrator {
    rules: Vec<OrchestrationRule>,
    events: Vec<CrossTeamEvent>,
}

impl CrossTeamOrchestrator {
    pub fn new() -> Self {
        let mut orchestrator = Self {
            rules: Vec::new(),
            events: Vec::new(),
        };
        orchestrator.add_defaults();
        orchestrator
    }

    /// Add a default set of inter-team rules
    pub fn add_defaults(&mut self) {
        let defaults = vec![
            OrchestrationRule {
                id: "rule_mkt_sales".into(),
                trigger_team: "marketing".into(),
                trigger_event: "lead_generated".into(),
                target_team: "sales".into(),
                target_action: "create_sales_task".into(),
                description: "Cuando Marketing genera un lead, crear tarea en Ventas".into(),
                active: true,
            },
            OrchestrationRule {
                id: "rule_sales_finance".into(),
                trigger_team: "sales".into(),
                trigger_event: "deal_closed".into(),
                target_team: "finance".into(),
                target_action: "generate_invoice".into(),
                description: "Cuando Ventas cierra un deal, generar factura en Finanzas".into(),
                active: true,
            },
            OrchestrationRule {
                id: "rule_support_sales".into(),
                trigger_team: "support".into(),
                trigger_event: "customer_complaint".into(),
                target_team: "sales".into(),
                target_action: "notify_account_manager".into(),
                description: "Cuando Soporte recibe queja, notificar al gerente de cuenta".into(),
                active: true,
            },
            OrchestrationRule {
                id: "rule_content_mkt".into(),
                trigger_team: "content".into(),
                trigger_event: "article_published".into(),
                target_team: "marketing".into(),
                target_action: "create_social_posts".into(),
                description: "Cuando Contenido publica articulo, crear posts en Marketing".into(),
                active: true,
            },
            OrchestrationRule {
                id: "rule_finance_sales".into(),
                trigger_team: "finance".into(),
                trigger_event: "payment_overdue".into(),
                target_team: "sales".into(),
                target_action: "trigger_followup".into(),
                description: "Cuando Finanzas detecta pago vencido, seguimiento en Ventas".into(),
                active: true,
            },
        ];
        for rule in defaults {
            if !self.rules.iter().any(|r| r.id == rule.id) {
                self.rules.push(rule);
            }
        }
    }

    pub fn add_rule(&mut self, rule: OrchestrationRule) {
        self.rules.push(rule);
    }

    pub fn fire_event(&mut self, event: CrossTeamEvent) {
        self.events.push(event);
    }

    /// Process pending events: match against rules and return triggered actions
    /// with task descriptions that can be fed directly into the agent loop.
    pub fn process_pending(&mut self) -> Vec<TriggeredAction> {
        let mut triggered = Vec::new();
        for event in &mut self.events {
            if event.processed {
                continue;
            }
            for rule in &self.rules {
                if !rule.active {
                    continue;
                }
                if rule.trigger_team == event.from_team && rule.trigger_event == event.event_type {
                    let task_description = format!(
                        "[Auto-trigger] {} -> {}: {}\nRegla: {}\nEvento original: {:?}",
                        rule.trigger_team,
                        rule.target_team,
                        rule.target_action,
                        rule.description,
                        event.payload
                    );
                    triggered.push(TriggeredAction {
                        event: event.clone(),
                        rule_id: rule.id.clone(),
                        target_team: rule.target_team.clone(),
                        target_action: rule.target_action.clone(),
                        task_description,
                    });
                    break;
                }
            }
            event.processed = true;
        }
        triggered
    }

    pub fn get_event_log(&self, limit: usize) -> Vec<&CrossTeamEvent> {
        self.events.iter().rev().take(limit).collect()
    }

    pub fn list_rules(&self) -> &[OrchestrationRule] {
        &self.rules
    }
}

impl Default for CrossTeamOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_rules_loaded() {
        let o = CrossTeamOrchestrator::new();
        assert_eq!(o.list_rules().len(), 5);
    }

    #[test]
    fn fire_and_process() {
        let mut o = CrossTeamOrchestrator::new();
        o.fire_event(CrossTeamEvent {
            id: "evt1".into(),
            from_team: "marketing".into(),
            to_team: "sales".into(),
            event_type: "lead_generated".into(),
            payload: serde_json::json!({"lead": "test@example.com"}),
            created_at: "2026-04-04T00:00:00Z".into(),
            processed: false,
        });
        let triggered = o.process_pending();
        assert_eq!(triggered.len(), 1);
        assert!(triggered[0].event.processed);
        assert_eq!(triggered[0].target_team, "sales");
        assert_eq!(triggered[0].target_action, "create_sales_task");
        assert!(triggered[0].task_description.contains("Auto-trigger"));
    }

    #[test]
    fn inactive_rule_does_not_trigger() {
        let mut o = CrossTeamOrchestrator::new();
        // Deactivate all rules
        for rule in o.rules.iter_mut() {
            rule.active = false;
        }
        o.fire_event(CrossTeamEvent {
            id: "evt2".into(),
            from_team: "marketing".into(),
            to_team: "sales".into(),
            event_type: "lead_generated".into(),
            payload: serde_json::json!({}),
            created_at: "2026-04-04T00:00:00Z".into(),
            processed: false,
        });
        let triggered = o.process_pending();
        assert_eq!(triggered.len(), 0);
    }

    #[test]
    fn already_processed_events_skipped() {
        let mut o = CrossTeamOrchestrator::new();
        o.fire_event(CrossTeamEvent {
            id: "evt3".into(),
            from_team: "marketing".into(),
            to_team: "sales".into(),
            event_type: "lead_generated".into(),
            payload: serde_json::json!({}),
            created_at: "2026-04-04T00:00:00Z".into(),
            processed: false,
        });
        let first = o.process_pending();
        assert_eq!(first.len(), 1);
        // Processing again should yield nothing
        let second = o.process_pending();
        assert_eq!(second.len(), 0);
    }
}
