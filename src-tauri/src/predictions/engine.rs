use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A predicted next action with confidence score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictedAction {
    pub id: String,
    pub action: String,
    pub confidence: f64,
    pub context: String,
    pub created_at: String,
}

/// Engine that predicts likely next actions based on task history patterns.
pub struct PredictionEngine {
    /// In-memory history of recent tasks for pattern detection
    history: Vec<String>,
    /// Dismissed prediction IDs
    dismissed: Vec<String>,
    /// Pattern frequency tracker: pattern -> count
    patterns: HashMap<String, usize>,
}

impl PredictionEngine {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            dismissed: Vec::new(),
            patterns: HashMap::new(),
        }
    }

    /// Record a task in history and update patterns.
    pub fn record_task(&mut self, task: &str) {
        self.history.push(task.to_string());

        // Track bigram patterns (consecutive task pairs)
        let len = self.history.len();
        if len >= 2 {
            let pattern = format!("{} -> {}", self.history[len - 2], self.history[len - 1]);
            *self.patterns.entry(pattern).or_insert(0) += 1;
        }

        // Keep history bounded
        if self.history.len() > 500 {
            self.history.drain(0..250);
        }
    }

    /// Predict next actions based on recent task history patterns.
    pub fn predict_next_actions(&mut self, recent_tasks: &[String]) -> Vec<PredictedAction> {
        // Record all provided tasks
        for task in recent_tasks {
            self.record_task(task);
        }

        let mut predictions = Vec::new();
        let now = chrono::Utc::now().to_rfc3339();

        if let Some(last_task) = self.history.last().cloned() {
            // Find patterns that start with the last task
            let prefix = format!("{} -> ", last_task);
            let mut candidates: Vec<(String, usize)> = self
                .patterns
                .iter()
                .filter(|(k, _)| k.starts_with(&prefix))
                .map(|(k, v)| {
                    let predicted = k.strip_prefix(&prefix).unwrap_or(k).to_string();
                    (predicted, *v)
                })
                .collect();

            candidates.sort_by(|a, b| b.1.cmp(&a.1));

            let max_count = candidates.first().map(|(_, c)| *c).unwrap_or(1) as f64;

            for (i, (action, count)) in candidates.iter().take(5).enumerate() {
                let id = format!("pred-{}", uuid::Uuid::new_v4());
                if self.dismissed.contains(&id) {
                    continue;
                }
                predictions.push(PredictedAction {
                    id,
                    action: action.clone(),
                    confidence: (*count as f64 / max_count).min(1.0).max(0.1),
                    context: format!("After '{}' (seen {} times)", last_task, count),
                    created_at: now.clone(),
                });
                if i >= 4 {
                    break;
                }
            }
        }

        // If no pattern-based predictions, suggest common tasks
        if predictions.is_empty() {
            let common_suggestions = [
                ("Check email", 0.5, "Morning routine"),
                ("Review calendar", 0.4, "Daily planning"),
                ("Check pending tasks", 0.35, "Task management"),
            ];
            for (action, confidence, context) in &common_suggestions {
                predictions.push(PredictedAction {
                    id: format!("pred-{}", uuid::Uuid::new_v4()),
                    action: action.to_string(),
                    confidence: *confidence,
                    context: context.to_string(),
                    created_at: chrono::Utc::now().to_rfc3339(),
                });
            }
        }

        predictions
    }

    /// Get suggestions based on a context string (e.g., time of day, app open).
    pub fn get_suggestions(&self, context: &str) -> Vec<PredictedAction> {
        let now = chrono::Utc::now().to_rfc3339();
        let ctx_lower = context.to_lowercase();

        let mut suggestions = Vec::new();

        if ctx_lower.contains("morning") || ctx_lower.contains("9am") || ctx_lower.contains("start")
        {
            suggestions.push(PredictedAction {
                id: format!("sug-{}", uuid::Uuid::new_v4()),
                action: "Monday briefing".to_string(),
                confidence: 0.8,
                context: "Morning routine pattern".to_string(),
                created_at: now.clone(),
            });
        }
        if ctx_lower.contains("meeting") || ctx_lower.contains("calendar") {
            suggestions.push(PredictedAction {
                id: format!("sug-{}", uuid::Uuid::new_v4()),
                action: "Prepare meeting briefing".to_string(),
                confidence: 0.75,
                context: "Pre-meeting preparation".to_string(),
                created_at: now.clone(),
            });
        }
        if ctx_lower.contains("code") || ctx_lower.contains("vscode") || ctx_lower.contains("dev") {
            suggestions.push(PredictedAction {
                id: format!("sug-{}", uuid::Uuid::new_v4()),
                action: "Check git status and pending PRs".to_string(),
                confidence: 0.7,
                context: "Development session start".to_string(),
                created_at: now.clone(),
            });
        }
        if ctx_lower.contains("email") || ctx_lower.contains("inbox") {
            suggestions.push(PredictedAction {
                id: format!("sug-{}", uuid::Uuid::new_v4()),
                action: "Summarize unread emails".to_string(),
                confidence: 0.65,
                context: "Email triage".to_string(),
                created_at: now.clone(),
            });
        }

        if suggestions.is_empty() {
            suggestions.push(PredictedAction {
                id: format!("sug-{}", uuid::Uuid::new_v4()),
                action: "No specific suggestions for this context".to_string(),
                confidence: 0.1,
                context: context.to_string(),
                created_at: now,
            });
        }

        suggestions
    }

    /// Dismiss a prediction so it won't appear again.
    pub fn dismiss(&mut self, id: &str) {
        self.dismissed.push(id.to_string());
        // Bound dismissed list
        if self.dismissed.len() > 200 {
            self.dismissed.drain(0..100);
        }
    }
}
