use rusqlite::Connection;
use serde::{Deserialize, Serialize};

// ── Structs ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunnelStage {
    pub name: String,
    pub count: u64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunnelData {
    pub stages: Vec<FunnelStage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohortRow {
    pub period: String,
    pub users: u64,
    pub retention_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionData {
    pub cohorts: Vec<CohortRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostForecast {
    pub daily_avg: f64,
    pub weekly_estimate: f64,
    pub monthly_estimate: f64,
    pub trend: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelScore {
    pub model: String,
    pub tasks: u64,
    pub success_rate: f64,
    pub avg_duration_ms: f64,
    pub avg_cost: f64,
}

// ── AnalyticsPro ──────────────────────────────────────────────────

pub struct AnalyticsPro;

impl AnalyticsPro {
    /// Build a user-journey funnel from task data.
    pub fn calculate_funnel(conn: &Connection) -> Result<FunnelData, String> {
        // Stage 1: Total tasks created
        let total: u64 = conn
            .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
            .unwrap_or(0);

        // Stage 2: Tasks that got a response (result is not empty)
        let responded: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks WHERE result IS NOT NULL AND result != ''",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        // Stage 3: Tasks marked completed / successful
        let completed: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks WHERE status = 'completed'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let pct = |n: u64| -> f64 {
            if total == 0 {
                0.0
            } else {
                (n as f64 / total as f64) * 100.0
            }
        };

        Ok(FunnelData {
            stages: vec![
                FunnelStage { name: "Tasks Created".into(), count: total, percentage: 100.0 },
                FunnelStage { name: "Got Response".into(), count: responded, percentage: pct(responded) },
                FunnelStage { name: "Completed".into(), count: completed, percentage: pct(completed) },
            ],
        })
    }

    /// Compute weekly retention cohorts.
    pub fn calculate_retention(conn: &Connection) -> Result<RetentionData, String> {
        let mut stmt = conn
            .prepare(
                "SELECT strftime('%Y-W%W', created_at) as period,
                        COUNT(DISTINCT date(created_at)) as active_days
                 FROM tasks
                 GROUP BY period
                 ORDER BY period DESC
                 LIMIT 12",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                let period: String = row.get(0)?;
                let active_days: u64 = row.get(1)?;
                // retention_pct: active days / 7 as a rough proxy
                let retention_pct = (active_days as f64 / 7.0 * 100.0).min(100.0);
                Ok(CohortRow {
                    period,
                    users: active_days,
                    retention_pct,
                })
            })
            .map_err(|e| e.to_string())?;

        let cohorts: Vec<CohortRow> = rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
        Ok(RetentionData { cohorts })
    }

    /// Forecast costs based on recent spending.
    pub fn forecast_costs(conn: &Connection) -> Result<CostForecast, String> {
        // Average daily cost over the last 30 days
        let daily_avg: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(CAST(cost AS REAL)), 0) / MAX(1, COUNT(DISTINCT date(created_at)))
                 FROM tasks WHERE created_at > datetime('now', '-30 days')",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0.0);

        // Cost in last 7 days vs previous 7 days for trend
        let recent_cost: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(CAST(cost AS REAL)), 0) FROM tasks WHERE created_at > datetime('now', '-7 days')",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0.0);

        let prev_cost: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(CAST(cost AS REAL)), 0) FROM tasks WHERE created_at BETWEEN datetime('now', '-14 days') AND datetime('now', '-7 days')",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0.0);

        let trend = if recent_cost > prev_cost * 1.1 {
            "increasing".to_string()
        } else if recent_cost < prev_cost * 0.9 {
            "decreasing".to_string()
        } else {
            "stable".to_string()
        };

        Ok(CostForecast {
            daily_avg,
            weekly_estimate: daily_avg * 7.0,
            monthly_estimate: daily_avg * 30.0,
            trend,
        })
    }

    /// Compare performance across different LLM models.
    pub fn compare_models(conn: &Connection) -> Result<Vec<ModelScore>, String> {
        // Check if model column exists; fall back gracefully
        let has_model = conn
            .prepare("SELECT model FROM tasks LIMIT 0")
            .is_ok();

        if !has_model {
            // No model column — return a single aggregate
            let total: u64 = conn
                .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
                .unwrap_or(0);
            let completed: u64 = conn
                .query_row("SELECT COUNT(*) FROM tasks WHERE status = 'completed'", [], |r| r.get(0))
                .unwrap_or(0);
            let avg_cost: f64 = conn
                .query_row("SELECT COALESCE(AVG(CAST(cost AS REAL)), 0) FROM tasks", [], |r| r.get(0))
                .unwrap_or(0.0);
            let success_rate = if total == 0 { 0.0 } else { completed as f64 / total as f64 * 100.0 };

            return Ok(vec![ModelScore {
                model: "default".to_string(),
                tasks: total,
                success_rate,
                avg_duration_ms: 0.0,
                avg_cost,
            }]);
        }

        let mut stmt = conn
            .prepare(
                "SELECT COALESCE(model, 'unknown') as m,
                        COUNT(*) as cnt,
                        SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as ok_cnt,
                        COALESCE(AVG(CAST(cost AS REAL)), 0) as avg_c
                 FROM tasks
                 GROUP BY m
                 ORDER BY cnt DESC",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                let model: String = row.get(0)?;
                let tasks: u64 = row.get(1)?;
                let ok: u64 = row.get(2)?;
                let avg_cost: f64 = row.get(3)?;
                let success_rate = if tasks == 0 { 0.0 } else { ok as f64 / tasks as f64 * 100.0 };
                Ok(ModelScore {
                    model,
                    tasks,
                    success_rate,
                    avg_duration_ms: 0.0,
                    avg_cost,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }
}
