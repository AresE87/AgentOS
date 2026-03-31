use crate::enterprise::audit::AuditLog;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestorMetrics {
    pub total_tasks: u64,
    pub completed_tasks: u64,
    pub failed_tasks: u64,
    pub success_rate: f64,
    pub paid_plan_days: u64,
    pub blocked_attempts: u64,
    pub upgrade_intents: u64,
    pub completed_upgrades: u64,
    pub open_handoffs: u64,
    pub completed_handoffs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRoomDocument {
    pub name: String,
    pub category: String,
    pub description: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YearProjection {
    pub period: String,
    pub estimated_mrr: f64,
    pub projected_tasks: u64,
    pub blocked_attempts: u64,
    pub note: String,
}

pub struct IPODashboard;

impl IPODashboard {
    pub fn new() -> Self {
        Self
    }

    pub fn calculate_metrics(&self, conn: &Connection) -> InvestorMetrics {
        let total_tasks = scalar_u64(conn, "SELECT COUNT(*) FROM tasks");
        let completed_tasks = scalar_u64(conn, "SELECT COUNT(*) FROM tasks WHERE status = 'completed'");
        let failed_tasks = scalar_u64(conn, "SELECT COUNT(*) FROM tasks WHERE status = 'failed'");
        let paid_plan_days = scalar_u64(
            conn,
            "SELECT COUNT(*) FROM daily_usage WHERE plan_type IN ('pro', 'team')",
        );
        let blocked_attempts = audit_count(conn, "billing_limit_blocked");
        let upgrade_intents = audit_count(conn, "upgrade_checkout_requested");
        let completed_upgrades = AuditLog::get_by_event_type(conn, "plan_changed", 500)
            .unwrap_or_default()
            .into_iter()
            .filter(|entry| {
                serde_json::from_str::<serde_json::Value>(&entry.details)
                    .ok()
                    .and_then(|value| value.get("plan_type").and_then(|p| p.as_str()).map(str::to_string))
                    .map(|plan| plan == "pro" || plan == "team")
                    .unwrap_or(false)
            })
            .count() as u64;
        let open_handoffs = scalar_u64(
            conn,
            "SELECT COUNT(*) FROM human_handoffs WHERE status IN ('pending_handoff', 'assigned_to_human', 'resumed')",
        );
        let completed_handoffs = scalar_u64(
            conn,
            "SELECT COUNT(*) FROM human_handoffs WHERE status = 'completed_by_human'",
        );

        InvestorMetrics {
            total_tasks,
            completed_tasks,
            failed_tasks,
            success_rate: if total_tasks == 0 {
                0.0
            } else {
                completed_tasks as f64 / total_tasks as f64 * 100.0
            },
            paid_plan_days,
            blocked_attempts,
            upgrade_intents,
            completed_upgrades,
            open_handoffs,
            completed_handoffs,
        }
    }

    pub fn generate_data_room_index(&self, conn: &Connection) -> Vec<DataRoomDocument> {
        vec![
            DataRoomDocument {
                name: "Task Execution History".to_string(),
                category: "Operations".to_string(),
                description: format!("{} persisted tasks in the workspace database", scalar_u64(conn, "SELECT COUNT(*) FROM tasks")),
                status: readiness_from_count(scalar_u64(conn, "SELECT COUNT(*) FROM tasks")),
            },
            DataRoomDocument {
                name: "Revenue Funnel Events".to_string(),
                category: "Revenue".to_string(),
                description: format!(
                    "{} checkout requests and {} completed upgrades",
                    audit_count(conn, "upgrade_checkout_requested"),
                    audit_count(conn, "plan_changed")
                ),
                status: readiness_from_count(audit_count(conn, "upgrade_checkout_requested")),
            },
            DataRoomDocument {
                name: "Human Handoff Register".to_string(),
                category: "Operations".to_string(),
                description: format!(
                    "{} total handoffs tracked",
                    scalar_u64(conn, "SELECT COUNT(*) FROM human_handoffs")
                ),
                status: readiness_from_count(scalar_u64(conn, "SELECT COUNT(*) FROM human_handoffs")),
            },
            DataRoomDocument {
                name: "Execution Traces".to_string(),
                category: "Debugging".to_string(),
                description: format!(
                    "{} debugger traces available for audit",
                    scalar_u64(conn, "SELECT COUNT(*) FROM execution_traces")
                ),
                status: readiness_from_count(scalar_u64(conn, "SELECT COUNT(*) FROM execution_traces")),
            },
        ]
    }

    pub fn get_projections(&self, conn: &Connection, periods: u32) -> Vec<YearProjection> {
        let current_plan = conn
            .query_row(
                "SELECT plan_type FROM daily_usage ORDER BY date DESC LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap_or_else(|_| "free".to_string());
        let current_mrr = match current_plan.as_str() {
            "pro" => 29.0,
            "team" => 99.0,
            _ => 0.0,
        };
        let total_tasks = scalar_u64(conn, "SELECT COUNT(*) FROM tasks");
        let blocked_attempts = audit_count(conn, "billing_limit_blocked");
        let upgrade_intents = audit_count(conn, "upgrade_checkout_requested");
        let completed_upgrades = audit_count(conn, "plan_changed");
        let conversion_rate = if upgrade_intents == 0 {
            0.0
        } else {
            completed_upgrades as f64 / upgrade_intents as f64
        };

        (0..periods)
            .map(|idx| YearProjection {
                period: format!("P{}", idx + 1),
                estimated_mrr: current_mrr,
                projected_tasks: total_tasks,
                blocked_attempts,
                note: if current_mrr == 0.0 && upgrade_intents == 0 {
                    "No paid-plan signal yet; projection stays flat until real upgrades happen.".to_string()
                } else {
                    format!(
                        "Flat projection based on current plan {}, upgrade conversion {:.0}% and observed local usage.",
                        current_plan,
                        conversion_rate * 100.0
                    )
                },
            })
            .collect()
    }
}

fn readiness_from_count(count: u64) -> String {
    if count == 0 {
        "missing".to_string()
    } else {
        "ready".to_string()
    }
}

fn scalar_u64(conn: &Connection, sql: &str) -> u64 {
    conn.query_row(sql, [], |row| row.get::<_, i64>(0))
        .unwrap_or(0) as u64
}

fn audit_count(conn: &Connection, event_type: &str) -> u64 {
    AuditLog::get_by_event_type(conn, event_type, 500)
        .map(|rows| rows.len() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE tasks (
                id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE daily_usage (
                date TEXT PRIMARY KEY,
                tasks_count INTEGER NOT NULL DEFAULT 0,
                tokens_used INTEGER NOT NULL DEFAULT 0,
                plan_type TEXT NOT NULL DEFAULT 'free'
            );
            CREATE TABLE human_handoffs (
                id TEXT PRIMARY KEY,
                status TEXT NOT NULL
            );
            CREATE TABLE execution_traces (
                id TEXT PRIMARY KEY
            );",
        )
        .unwrap();
        AuditLog::ensure_table(&conn).unwrap();
        conn
    }

    #[test]
    fn investor_metrics_use_real_system_tables() {
        let conn = setup_conn();
        conn.execute("INSERT INTO tasks (id, status) VALUES ('t1', 'completed')", []).unwrap();
        conn.execute("INSERT INTO tasks (id, status) VALUES ('t2', 'failed')", []).unwrap();
        conn.execute(
            "INSERT INTO daily_usage (date, tasks_count, tokens_used, plan_type) VALUES ('2026-03-31', 12, 5000, 'pro')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO human_handoffs (id, status) VALUES ('h1', 'pending_handoff'), ('h2', 'completed_by_human')",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO execution_traces (id) VALUES ('trace-1')", []).unwrap();
        AuditLog::log(
            &conn,
            "upgrade_checkout_requested",
            serde_json::json!({ "plan": "pro", "variant": "limit-focused" }),
        )
        .unwrap();
        AuditLog::log(
            &conn,
            "plan_changed",
            serde_json::json!({ "plan_type": "pro", "variant": "limit-focused" }),
        )
        .unwrap();

        let dashboard = IPODashboard::new();
        let metrics = dashboard.calculate_metrics(&conn);
        let data_room = dashboard.generate_data_room_index(&conn);

        assert_eq!(metrics.total_tasks, 2);
        assert_eq!(metrics.completed_upgrades, 1);
        assert_eq!(metrics.open_handoffs, 1);
        assert_eq!(data_room[0].status, "ready");
        assert!(data_room.iter().any(|doc| doc.name == "Execution Traces"));
    }
}
