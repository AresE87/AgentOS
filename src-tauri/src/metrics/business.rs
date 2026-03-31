use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessMetrics {
    pub total_tasks: u64,
    pub tasks_this_month: u64,
    pub tasks_last_month: u64,
    pub growth_rate: f64,
    pub total_users: u64,
    pub active_users: u64,
    pub total_llm_cost: f64,
    pub avg_cost_per_task: f64,
    pub uptime_hours: f64,
    pub installed_plugins: u64,
    pub installed_playbooks: u64,
    pub marketplace_reviews: u64,
    pub feedback_positive_rate: f64,
    pub mesh_nodes_ever_connected: u64,
}

impl BusinessMetrics {
    pub fn calculate(conn: &Connection) -> Result<Self, String> {
        let total_tasks = Self::count(conn, "tasks", None)?;
        let tasks_this_month = Self::count(
            conn,
            "tasks",
            Some("created_at > datetime('now', '-30 days')"),
        )?;
        let tasks_last_month = Self::count(conn, "tasks", Some("created_at > datetime('now', '-60 days') AND created_at <= datetime('now', '-30 days')"))?;

        let growth_rate = if tasks_last_month > 0 {
            ((tasks_this_month as f64 - tasks_last_month as f64) / tasks_last_month as f64) * 100.0
        } else {
            0.0
        };

        let total_users = Self::count(conn, "api_keys", None)?;
        let active_users = Self::count(
            conn,
            "api_keys",
            Some("last_used > datetime('now', '-30 days')"),
        )?;

        let total_cost: f64 = if Self::table_exists(conn, "tasks")? {
            conn.query_row(
                "SELECT COALESCE(SUM(CAST(cost AS REAL)), 0) FROM tasks",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0.0)
        } else {
            0.0
        };

        let avg_cost = if total_tasks > 0 {
            total_cost / total_tasks as f64
        } else {
            0.0
        };

        let positive_feedback = Self::count(conn, "feedback", Some("rating > 0"))?;
        let total_feedback = Self::count(conn, "feedback", None)?;
        let feedback_rate = if total_feedback > 0 {
            positive_feedback as f64 / total_feedback as f64
        } else {
            0.0
        };

        let installed_plugins = Self::count(conn, "installed_plugins", None)?;
        let installed_playbooks = Self::count(conn, "marketplace_installs", None)?;
        let marketplace_reviews = Self::count(conn, "marketplace_reviews", None)?;

        Ok(Self {
            total_tasks,
            tasks_this_month,
            tasks_last_month,
            growth_rate,
            total_users,
            active_users,
            total_llm_cost: total_cost,
            avg_cost_per_task: avg_cost,
            uptime_hours: 0.0,
            installed_plugins,
            installed_playbooks,
            marketplace_reviews,
            feedback_positive_rate: feedback_rate,
            mesh_nodes_ever_connected: 0,
        })
    }

    fn table_exists(conn: &Connection, table: &str) -> Result<bool, String> {
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
            [table],
            |row| row.get::<_, i64>(0).map(|n| n > 0),
        )
        .map_err(|e| e.to_string())
    }

    fn count(conn: &Connection, table: &str, where_clause: Option<&str>) -> Result<u64, String> {
        if !Self::table_exists(conn, table)? {
            return Ok(0);
        }

        let sql = match where_clause {
            Some(w) => format!("SELECT COUNT(*) FROM {} WHERE {}", table, w),
            None => format!("SELECT COUNT(*) FROM {}", table),
        };

        conn.query_row(&sql, [], |row| row.get::<_, i64>(0))
            .map(|n| n as u64)
            .map_err(|e| e.to_string())
    }
}
