use super::collector::{FeedbackRecord, FeedbackStats};
use rusqlite::Connection;

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

    /// Generate a weekly insight report and persist it to the database.
    /// Returns the generated insights text.
    pub fn generate_and_save(conn: &Connection) -> Result<String, String> {
        super::collector::FeedbackCollector::ensure_table(conn)?;
        let records = super::collector::FeedbackCollector::get_recent(conn, 100)?;
        let stats = super::collector::FeedbackCollector::get_stats(conn)?;
        let insights = Self::generate_weekly_insights(&records, &stats);

        // Ensure weekly_reports table exists
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS weekly_reports (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL
            )"
        ).map_err(|e| e.to_string())?;

        conn.execute(
            "INSERT INTO weekly_reports (id, content, created_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![
                uuid::Uuid::new_v4().to_string(),
                insights,
                chrono::Utc::now().to_rfc3339()
            ],
        ).map_err(|e| e.to_string())?;

        Ok(insights)
    }

    /// List previously generated weekly reports.
    pub fn list_reports(conn: &Connection, limit: usize) -> Result<Vec<serde_json::Value>, String> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS weekly_reports (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL
            )"
        ).map_err(|e| e.to_string())?;

        let mut stmt = conn.prepare(
            "SELECT id, content, created_at FROM weekly_reports ORDER BY created_at DESC LIMIT ?1"
        ).map_err(|e| e.to_string())?;

        let rows = stmt.query_map(rusqlite::params![limit as i64], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "content": row.get::<_, String>(1)?,
                "created_at": row.get::<_, String>(2)?,
            }))
        }).map_err(|e| e.to_string())?;

        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}
