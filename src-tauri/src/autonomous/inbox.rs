use serde::{Deserialize, Serialize};
use serde_json::Value;

/// An action the autonomous inbox can take on a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AutoAction {
    Reply(String),
    Forward(String),
    Archive,
    Label(String),
    Escalate,
}

/// A rule that maps a condition to an action for inbox processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxRule {
    pub id: String,
    pub name: String,
    /// Condition expression (e.g. "from:newsletter@*" or "subject:contains:invoice")
    pub condition: String,
    /// Action expression (e.g. "archive" or "label:billing")
    pub action: String,
    pub enabled: bool,
    pub priority: u32,
}

/// An incoming message to be evaluated against inbox rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxMessage {
    pub from: String,
    pub subject: String,
    pub body: String,
    pub labels: Vec<String>,
}

/// Autonomous Inbox manager — evaluates messages against rules
pub struct AutoInbox {
    rules: Vec<InboxRule>,
    next_id: u64,
}

impl AutoInbox {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            next_id: 1,
        }
    }

    /// Add a new inbox rule. Returns the assigned rule id.
    pub fn add_rule(&mut self, mut rule: InboxRule) -> String {
        if rule.id.is_empty() {
            rule.id = format!("inbox-rule-{}", self.next_id);
            self.next_id += 1;
        }
        let id = rule.id.clone();
        self.rules.push(rule);
        // Keep sorted by priority (lower number = higher priority)
        self.rules.sort_by_key(|r| r.priority);
        id
    }

    /// List all inbox rules
    pub fn list_rules(&self) -> Vec<InboxRule> {
        self.rules.clone()
    }

    /// Process a message against all enabled rules.
    /// Returns the first matching action, or None if no rules match.
    pub fn process_message(&self, message: &InboxMessage) -> Option<AutoAction> {
        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }
            if Self::matches_condition(&rule.condition, message) {
                return Some(Self::parse_action(&rule.action));
            }
        }
        None
    }

    /// Remove a rule by id
    pub fn remove_rule(&mut self, id: &str) -> bool {
        let before = self.rules.len();
        self.rules.retain(|r| r.id != id);
        self.rules.len() < before
    }

    /// Toggle a rule's enabled state
    pub fn toggle_rule(&mut self, id: &str) -> Option<bool> {
        for rule in &mut self.rules {
            if rule.id == id {
                rule.enabled = !rule.enabled;
                return Some(rule.enabled);
            }
        }
        None
    }

    // ── helpers ───────────────────────────────────────────────

    fn matches_condition(condition: &str, msg: &InboxMessage) -> bool {
        let cond = condition.to_lowercase();
        let from = msg.from.to_lowercase();
        let subject = msg.subject.to_lowercase();
        let body = msg.body.to_lowercase();

        if cond.starts_with("from:") {
            let pattern = &cond[5..];
            if pattern.ends_with('*') {
                let prefix = &pattern[..pattern.len() - 1];
                return from.contains(prefix);
            }
            return from.contains(pattern);
        }
        if cond.starts_with("subject:contains:") {
            let keyword = &cond[17..];
            return subject.contains(keyword);
        }
        if cond.starts_with("body:contains:") {
            let keyword = &cond[14..];
            return body.contains(keyword);
        }
        if cond.starts_with("label:") {
            let label = &cond[6..];
            return msg.labels.iter().any(|l| l.to_lowercase() == label);
        }
        // Fallback: substring search across all fields
        from.contains(&cond) || subject.contains(&cond) || body.contains(&cond)
    }

    fn parse_action(action_str: &str) -> AutoAction {
        let lower = action_str.to_lowercase();
        if lower == "archive" {
            return AutoAction::Archive;
        }
        if lower == "escalate" {
            return AutoAction::Escalate;
        }
        if lower.starts_with("reply:") {
            return AutoAction::Reply(action_str[6..].to_string());
        }
        if lower.starts_with("forward:") {
            return AutoAction::Forward(action_str[8..].to_string());
        }
        if lower.starts_with("label:") {
            return AutoAction::Label(action_str[6..].to_string());
        }
        // Default to escalate for unrecognised actions
        AutoAction::Escalate
    }
}
