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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReliabilityObjective {
    pub name: String,
    pub sli: f64,
    pub target: f64,
    pub remaining_error_budget: f64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReliabilityReport {
    pub window_days: u32,
    pub evaluated_at: String,
    pub total_tasks: u64,
    pub successful_tasks: u64,
    pub failed_tasks: u64,
    pub p95_latency_ms: f64,
    pub open_handoffs: u64,
    pub unresolved_alerts: u64,
    pub objectives: Vec<ReliabilityObjective>,
    pub overall_status: String,
    pub breached_objectives: Vec<String>,
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
            status: if api_enabled {
                "ok".into()
            } else {
                "warning".into()
            },
            details: if api_enabled {
                "Public API enabled".into()
            } else {
                "Public API disabled".into()
            },
            latency_ms: None,
        });

        components.push(ComponentHealth {
            name: "Vault".into(),
            status: if vault_unlocked {
                "ok".into()
            } else {
                "warning".into()
            },
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

    pub fn reliability_report(conn: &Connection, window_days: u32) -> ReliabilityReport {
        let total_tasks = scalar_u64(
            conn,
            &format!(
                "SELECT COUNT(*) FROM tasks WHERE created_at >= datetime('now', '-{} days')",
                window_days
            ),
        );
        let failed_tasks = scalar_u64(
            conn,
            &format!(
                "SELECT COUNT(*) FROM tasks WHERE status = 'failed' AND created_at >= datetime('now', '-{} days')",
                window_days
            ),
        );
        let successful_tasks = total_tasks.saturating_sub(failed_tasks);
        let latency_values = load_latency_values(conn, window_days);
        let p95_latency_ms = percentile(&latency_values, 0.95);
        let open_handoffs = scalar_u64(
            conn,
            "SELECT COUNT(*) FROM human_handoffs WHERE status IN ('pending_handoff', 'assigned_to_human', 'resumed')",
        );
        let unresolved_alerts = scalar_u64(
            conn,
            "SELECT COUNT(*) FROM audit_log WHERE event_type IN ('incident_opened', 'reliability_alert')",
        );

        let success_sli = percentage(successful_tasks, total_tasks);
        let latency_sli = if p95_latency_ms <= 0.0 {
            100.0
        } else if p95_latency_ms <= 2000.0 {
            100.0
        } else {
            (2000.0 / p95_latency_ms * 100.0).min(100.0)
        };
        let handoff_sli = if open_handoffs == 0 { 100.0 } else { 0.0 };

        let objectives = vec![
            build_objective("task_success_rate", success_sli, 99.0),
            build_objective("p95_latency_under_2s", latency_sli, 95.0),
            build_objective("handoff_backlog", handoff_sli, 100.0),
        ];
        let breached_objectives = objectives
            .iter()
            .filter(|objective| objective.status == "breached")
            .map(|objective| objective.name.clone())
            .collect::<Vec<_>>();
        let overall_status = if breached_objectives.is_empty() {
            if unresolved_alerts > 0 {
                "warning"
            } else {
                "healthy"
            }
        } else {
            "breached"
        };

        ReliabilityReport {
            window_days,
            evaluated_at: chrono::Utc::now().to_rfc3339(),
            total_tasks,
            successful_tasks,
            failed_tasks,
            p95_latency_ms,
            open_handoffs,
            unresolved_alerts,
            objectives,
            overall_status: overall_status.to_string(),
            breached_objectives,
        }
    }
}

fn scalar_u64(conn: &Connection, sql: &str) -> u64 {
    conn.query_row(sql, [], |row| row.get::<_, i64>(0))
        .unwrap_or(0) as u64
}

fn percentage(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        100.0
    } else {
        (numerator as f64 / denominator as f64) * 100.0
    }
}

fn load_latency_values(conn: &Connection, window_days: u32) -> Vec<f64> {
    let sql = format!(
        "SELECT COALESCE(duration_ms, 0) FROM tasks WHERE created_at >= datetime('now', '-{} days') ORDER BY duration_ms ASC",
        window_days
    );
    let mut stmt = match conn.prepare(&sql) {
        Ok(stmt) => stmt,
        Err(_) => return Vec::new(),
    };
    stmt.query_map([], |row| row.get::<_, f64>(0))
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
}

fn percentile(values: &[f64], percentile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let index = ((values.len() - 1) as f64 * percentile).round() as usize;
    values[index]
}

fn build_objective(name: &str, sli: f64, target: f64) -> ReliabilityObjective {
    ReliabilityObjective {
        name: name.to_string(),
        sli,
        target,
        remaining_error_budget: (100.0 - (target - sli).max(0.0)).clamp(0.0, 100.0),
        status: if sli >= target {
            "met".to_string()
        } else {
            "breached".to_string()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enterprise::audit::AuditLog;

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

    #[test]
    fn reliability_report_uses_real_tasks_handoffs_and_alerts() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE tasks (
                id TEXT PRIMARY KEY,
                status TEXT,
                duration_ms INTEGER,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE human_handoffs (
                id TEXT PRIMARY KEY,
                status TEXT NOT NULL
            );",
        )
        .unwrap();
        AuditLog::ensure_table(&conn).unwrap();
        conn.execute(
            "INSERT INTO tasks (id, status, duration_ms, created_at) VALUES
                ('a', 'completed', 120, datetime('now')),
                ('b', 'completed', 240, datetime('now')),
                ('c', 'failed', 4000, datetime('now'))",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO human_handoffs (id, status) VALUES ('h1', 'assigned_to_human')",
            [],
        )
        .unwrap();
        AuditLog::log(
            &conn,
            "reliability_alert",
            serde_json::json!({ "component": "tasks" }),
        )
        .unwrap();

        let report = HealthDashboard::reliability_report(&conn, 30);

        assert_eq!(report.total_tasks, 3);
        assert_eq!(report.failed_tasks, 1);
        assert_eq!(report.open_handoffs, 1);
        assert_eq!(report.unresolved_alerts, 1);
        assert!(report
            .breached_objectives
            .iter()
            .any(|name| name == "task_success_rate"));
    }
}
