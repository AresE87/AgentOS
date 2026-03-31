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

pub struct ROICalculator;

impl ROICalculator {
    pub fn calculate(
        conn: &Connection,
        period: &str,
        hourly_rate: f64,
        avg_minutes_per_task: f64,
    ) -> Result<ROIReport, String> {
        let (date_filter, period_label) = match period {
            "week" => ("datetime('now', '-7 days')", "this_week"),
            "month" => ("datetime('now', '-30 days')", "this_month"),
            _ => ("datetime('now', '-365 days')", "all_time"),
        };

        let count_sql = format!(
            "SELECT COUNT(*) FROM tasks WHERE created_at > {}",
            date_filter
        );
        let tasks_completed: u32 = conn
            .query_row(&count_sql, [], |row| row.get(0))
            .unwrap_or(0);

        let cost_sql = format!(
            "SELECT COALESCE(SUM(CAST(cost AS REAL)), 0) FROM tasks WHERE created_at > {}",
            date_filter
        );
        let total_llm_cost: f64 = conn
            .query_row(&cost_sql, [], |row| row.get(0))
            .unwrap_or(0.0);

        let time_saved = tasks_completed as f64 * avg_minutes_per_task;
        let manual_cost = (time_saved / 60.0) * hourly_rate;
        let net_savings = manual_cost - total_llm_cost;
        let roi_pct = if total_llm_cost > 0.0 {
            (net_savings / total_llm_cost) * 100.0
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
