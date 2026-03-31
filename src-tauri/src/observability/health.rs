use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub overall: String, // "healthy", "degraded", "critical"
    pub components: Vec<ComponentHealth>,
    pub observed_at: String,
    pub total_tasks: u64,
    pub failed_tasks: u64,
    pub avg_task_latency_ms: f64,
    pub recent_error_logs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub name: String,
    pub status: String, // "ok", "warning", "error"
    pub details: String,
    pub latency_ms: Option<u64>,
}

pub struct HealthDashboard;

impl HealthDashboard {
    pub async fn check_all(
        db_path: &Path,
        api_enabled: bool,
        vault_unlocked: bool,
        providers_configured: usize,
        recent_error_logs: usize,
    ) -> HealthStatus {
        let mut components = vec![];
        let observed_at = chrono::Utc::now().to_rfc3339();
        let mut total_tasks = 0u64;
        let mut failed_tasks = 0u64;
        let mut avg_task_latency_ms = 0.0f64;

        match Connection::open(db_path) {
            Ok(conn) => {
                let db_probe_start = std::time::Instant::now();
                let count_result: Result<i64, _> =
                    conn.query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0));
                let failed_result: Result<i64, _> = conn.query_row(
                    "SELECT COUNT(*) FROM tasks WHERE status = 'failed'",
                    [],
                    |row| row.get(0),
                );
                let latency_result: Result<f64, _> = conn.query_row(
                    "SELECT COALESCE(AVG(duration_ms), 0) FROM tasks",
                    [],
                    |row| row.get(0),
                );
                match (count_result, failed_result, latency_result) {
                    (Ok(total), Ok(failed), Ok(avg_latency)) => {
                        total_tasks = total.max(0) as u64;
                        failed_tasks = failed.max(0) as u64;
                        avg_task_latency_ms = avg_latency.max(0.0);
                        components.push(ComponentHealth {
                            name: "Database".into(),
                            status: "ok".into(),
                            details: format!("SQLite operational, {} tasks tracked", total_tasks),
                            latency_ms: Some(db_probe_start.elapsed().as_millis() as u64),
                        });
                    }
                    _ => components.push(ComponentHealth {
                        name: "Database".into(),
                        status: "error".into(),
                        details: "SQLite reachable but task metrics query failed".into(),
                        latency_ms: Some(db_probe_start.elapsed().as_millis() as u64),
                    }),
                }
            }
            Err(error) => components.push(ComponentHealth {
                name: "Database".into(),
                status: "error".into(),
                details: format!("SQLite open failed: {}", error),
                latency_ms: None,
            }),
        }

        components.push(ComponentHealth {
            name: "LLM Provider".into(),
            status: if providers_configured > 0 {
                "ok".into()
            } else {
                "warning".into()
            },
            details: if providers_configured > 0 {
                format!("{} provider(s) configured", providers_configured)
            } else {
                "No provider credentials configured".into()
            },
            latency_ms: None,
        });

        components.push(ComponentHealth {
            name: "API Server".into(),
            status: if api_enabled { "ok".into() } else { "warning".into() },
            details: if api_enabled {
                "Public API enabled".into()
            } else {
                "Public API disabled".into()
            },
            latency_ms: None,
        });

        components.push(ComponentHealth {
            name: "Vault".into(),
            status: if vault_unlocked { "ok".into() } else { "warning".into() },
            details: if vault_unlocked {
                "Secrets loaded from secure vault".into()
            } else {
                "Vault locked or unavailable".into()
            },
            latency_ms: None,
        });

        let failure_rate = if total_tasks > 0 {
            (failed_tasks as f64 / total_tasks as f64) * 100.0
        } else {
            0.0
        };
        components.push(ComponentHealth {
            name: "Task Execution".into(),
            status: if failure_rate >= 25.0 {
                "error".into()
            } else if failure_rate >= 10.0 || recent_error_logs > 0 {
                "warning".into()
            } else {
                "ok".into()
            },
            details: format!(
                "failure_rate={:.1}% avg_latency_ms={:.1} recent_errors={}",
                failure_rate, avg_task_latency_ms, recent_error_logs
            ),
            latency_ms: Some(avg_task_latency_ms.round() as u64),
        });

        let overall = if components.iter().any(|c| c.status == "error") {
            "critical"
        } else if components.iter().any(|c| c.status == "warning") {
            "degraded"
        } else {
            "healthy"
        };

        HealthStatus {
            overall: overall.to_string(),
            components,
            observed_at,
            total_tasks,
            failed_tasks,
            avg_task_latency_ms,
            recent_error_logs: recent_error_logs as u64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn health_dashboard_reports_real_components() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("health.db");
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE tasks (
                id TEXT PRIMARY KEY,
                status TEXT,
                duration_ms INTEGER
            );
            INSERT INTO tasks (id, status, duration_ms) VALUES
                ('a', 'completed', 150),
                ('b', 'failed', 300);",
        )
        .unwrap();

        let status = HealthDashboard::check_all(&db_path, true, true, 1, 2).await;
        assert_eq!(status.total_tasks, 2);
        assert_eq!(status.failed_tasks, 1);
        assert!(status.avg_task_latency_ms >= 150.0);
        assert!(status.components.iter().any(|c| c.name == "Database"));
        assert!(status.components.iter().any(|c| c.name == "Task Execution"));
    }
}
