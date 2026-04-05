use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ROIReport {
    pub period: String,
    pub tasks_completed: u32,
    pub total_time_saved_minutes: f64,
    pub total_llm_cost: f64,
    pub estimated_manual_cost: f64,
    pub net_savings: f64,
    pub roi_percentage: f64,
    pub hourly_rate: f64,
    pub avg_minutes_per_task: f64,
}

impl ROIReport {
    /// Return a zeroed report when no data is available.
    pub fn empty(period: &str, hourly_rate: f64, avg_minutes_per_task: f64) -> Self {
        Self {
            period: period.to_string(),
            tasks_completed: 0,
            total_time_saved_minutes: 0.0,
            total_llm_cost: 0.0,
            estimated_manual_cost: 0.0,
            net_savings: 0.0,
            roi_percentage: 0.0,
            hourly_rate,
            avg_minutes_per_task,
        }
    }
}

pub struct ROICalculator;

impl ROICalculator {
    pub fn calculate(
        conn: &Connection,
        period: &str,
        hourly_rate: f64,
        avg_minutes_per_task: f64,
    ) -> Result<ROIReport, String> {
        let period_label = match period {
            "week" => "this_week",
            "month" => "this_month",
            _ => "all_time",
        };

        // Check if the tasks table actually exists
        let tasks_exist: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='tasks'",
                [],
                |r| r.get::<_, i64>(0).map(|n| n > 0),
            )
            .unwrap_or(false);

        if !tasks_exist {
            return Ok(ROIReport::empty(period_label, hourly_rate, avg_minutes_per_task));
        }

        // Detect whether created_at column exists (handle varying schemas)
        let has_created_at: bool = conn
            .prepare("SELECT created_at FROM tasks LIMIT 0")
            .is_ok();

        // Detect whether cost column exists
        let has_cost: bool = conn.prepare("SELECT cost FROM tasks LIMIT 0").is_ok();

        // Build date filter only if the column is present
        let date_filter = if has_created_at {
            let cutoff = match period {
                "week" => "datetime('now', '-7 days')",
                "month" => "datetime('now', '-30 days')",
                _ => "datetime('now', '-365 days')",
            };
            format!(" WHERE created_at > {}", cutoff)
        } else {
            String::new()
        };

        // Count tasks
        let count_sql = format!("SELECT COUNT(*) FROM tasks{}", date_filter);
        let tasks_completed: u32 = conn
            .query_row(&count_sql, [], |row| row.get::<_, i64>(0))
            .unwrap_or(0) as u32;

        // Sum cost if column exists, otherwise 0
        let total_llm_cost: f64 = if has_cost {
            let cost_sql = format!(
                "SELECT COALESCE(SUM(CAST(cost AS REAL)), 0) FROM tasks{}",
                if date_filter.is_empty() {
                    String::new()
                } else {
                    format!("{} AND cost IS NOT NULL", date_filter)
                }
            );
            conn.query_row(&cost_sql, [], |row| row.get(0))
                .unwrap_or(0.0)
        } else {
            0.0
        };

        let time_saved = tasks_completed as f64 * avg_minutes_per_task;
        let manual_cost = (time_saved / 60.0) * hourly_rate;
        let net_savings = manual_cost - total_llm_cost;
        let roi_pct = if total_llm_cost > 0.0 {
            (net_savings / total_llm_cost) * 100.0
        } else if tasks_completed > 0 {
            100.0 // All savings, no cost
        } else {
            0.0
        };

        Ok(ROIReport {
            period: period_label.to_string(),
            tasks_completed,
            total_time_saved_minutes: time_saved,
            total_llm_cost,
            estimated_manual_cost: manual_cost,
            net_savings,
            roi_percentage: roi_pct,
            hourly_rate,
            avg_minutes_per_task,
        })
    }
}
