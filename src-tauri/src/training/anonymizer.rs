use super::collector::TrainingRecord;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct Anonymizer;

impl Anonymizer {
    /// Anonymize a record for potential telemetry (opt-in only)
    pub fn anonymize(record: &TrainingRecord) -> serde_json::Value {
        // Hash the ID so it can't be traced back
        let mut hasher = DefaultHasher::new();
        record.id.hash(&mut hasher);
        let anon_id = format!("anon_{:x}", hasher.finish());

        serde_json::json!({
            "id": anon_id,
            "task_type": record.task_type,
            "complexity": record.complexity,
            "model": record.model_used,
            "success": record.success,
            "feedback": record.feedback_rating,
            "duration_ms": record.duration_ms,
            "tokens": record.token_count,
            // No timestamp — just the metrics
        })
    }

    /// Generate anonymized batch for export preview
    pub fn anonymize_batch(records: &[TrainingRecord]) -> Vec<serde_json::Value> {
        records.iter().map(|r| Self::anonymize(r)).collect()
    }

    /// Check if any PII exists in a string (basic check)
    pub fn contains_pii(text: &str) -> bool {
        // Check for email patterns
        let has_email = text.contains('@') && text.contains('.');
        // Check for phone-like patterns
        let digit_count = text.chars().filter(|c| c.is_ascii_digit()).count();
        let has_phone = digit_count >= 7;

        has_email || has_phone
    }
}
