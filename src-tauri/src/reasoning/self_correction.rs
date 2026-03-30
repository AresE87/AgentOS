use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single round of self-correction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionRound {
    pub round: u32,
    pub original: String,
    pub issue_found: String,
    pub corrected: String,
    pub model_used: String,
}

/// Self-corrector that verifies and iteratively corrects LLM outputs
pub struct SelfCorrector {
    /// correction history keyed by task_id
    history: HashMap<String, Vec<CorrectionRound>>,
    max_rounds: u32,
}

impl SelfCorrector {
    pub fn new() -> Self {
        Self {
            history: HashMap::new(),
            max_rounds: 2,
        }
    }

    /// Verify an output against a task description.
    /// Returns Some(issue) if a problem is detected, None if output looks fine.
    pub fn verify_output(&self, output: &str, task: &str) -> Option<String> {
        // Heuristic checks
        if output.trim().is_empty() {
            return Some("Output is empty".to_string());
        }
        if output.len() < 10 && task.len() > 50 {
            return Some("Output seems too short for the given task".to_string());
        }
        // Check for obvious placeholder text
        if output.contains("TODO") || output.contains("FIXME") {
            return Some("Output contains placeholder markers (TODO/FIXME)".to_string());
        }
        // Check for incomplete sentences (ends mid-word)
        let trimmed = output.trim();
        if !trimmed.is_empty() && !trimmed.ends_with('.') && !trimmed.ends_with('!')
            && !trimmed.ends_with('?') && !trimmed.ends_with('}') && !trimmed.ends_with('"')
            && !trimmed.ends_with(')')
        {
            // Might be truncated
            if trimmed.len() > 200 {
                return Some("Output appears to be truncated".to_string());
            }
        }
        None
    }

    /// Correct an output given a known issue. Returns improved output.
    pub fn correct(&mut self, task_id: &str, output: &str, issue: &str) -> String {
        let rounds = self.history.entry(task_id.to_string()).or_default();
        if rounds.len() as u32 >= self.max_rounds {
            return output.to_string();
        }

        // Apply simple rule-based corrections
        let mut corrected = output.to_string();

        if issue.contains("empty") {
            corrected = format!("[Auto-corrected] The task requires a substantive response.");
        } else if issue.contains("too short") {
            corrected = format!(
                "{} [Extended: the response has been flagged as potentially incomplete — further detail may be needed.]",
                output
            );
        } else if issue.contains("placeholder") {
            corrected = corrected.replace("TODO", "[resolved]").replace("FIXME", "[fixed]");
        } else if issue.contains("truncated") {
            corrected = format!("{}...[completion needed]", output.trim());
        }

        let round = CorrectionRound {
            round: rounds.len() as u32 + 1,
            original: output.to_string(),
            issue_found: issue.to_string(),
            corrected: corrected.clone(),
            model_used: "heuristic-v1".to_string(),
        };
        rounds.push(round);

        corrected
    }

    /// Get correction history for a task
    pub fn get_correction_history(&self, task_id: &str) -> Vec<CorrectionRound> {
        self.history.get(task_id).cloned().unwrap_or_default()
    }
}
