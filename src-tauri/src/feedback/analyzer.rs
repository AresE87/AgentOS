use super::collector::{FeedbackRecord, FeedbackStats};

pub struct InsightAnalyzer;

impl InsightAnalyzer {
    /// Analyze feedback records and generate insight text.
    /// Returns a multi-line summary string.
    pub fn generate_weekly_insights(records: &[FeedbackRecord], stats: &FeedbackStats) -> String {
        if records.is_empty() {
            return "No feedback recorded yet. Rate your tasks with thumbs up/down to improve AgentOS."
                .to_string();
        }

        let mut insights = vec![];

        // Overall satisfaction rate
        insights.push(format!(
            "Overall satisfaction: {:.0}% positive ({} rated tasks)",
            stats.positive_rate * 100.0,
            stats.total
        ));

        // Highlight poorly-rated tasks
        let negative: Vec<_> = records.iter().filter(|r| r.rating < 0).collect();
        if !negative.is_empty() {
            insights.push(format!(
                "{} tasks rated poorly — check your API connection and playbooks.",
                negative.len()
            ));
        }

        // Positive performance highlight
        if stats.positive_rate > 0.8 {
            insights.push("Great performance! 80%+ tasks are rated positively.".to_string());
        }

        insights.join("\n")
    }

    /// Suggest routing improvements based on feedback.
    /// Returns a list of model routing suggestions.
    pub fn get_routing_suggestions(records: &[FeedbackRecord]) -> Vec<String> {
        let mut suggestions = vec![];

        // Count negatives per model
        let model_negatives: std::collections::HashMap<&str, usize> = records
            .iter()
            .filter(|r| r.rating < 0)
            .fold(std::collections::HashMap::new(), |mut acc, r| {
                *acc.entry(r.model_used.as_str()).or_insert(0) += 1;
                acc
            });

        for (model, count) in &model_negatives {
            if *count > 3 {
                suggestions.push(format!(
                    "Model '{}' has {} negative ratings — consider switching to a better model.",
                    model, count
                ));
            }
        }

        if suggestions.is_empty() {
            suggestions.push(
                "Model routing looks good! Keep using the current configuration.".to_string(),
            );
        }

        suggestions
    }
}
