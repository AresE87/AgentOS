use crate::enterprise::audit::AuditLog;
use crate::observability::health::HealthDashboard;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryDemo {
    pub id: String,
    pub category: String,
    pub title: String,
    pub readiness: String,
    pub evidence: String,
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessCheck {
    pub name: String,
    pub status: String,
    pub evidence: String,
    pub open_gaps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinitiveReadinessReport {
    pub generated_at: String,
    pub overall_status: String,
    pub ready_checks: u64,
    pub warning_checks: u64,
    pub blocked_checks: u64,
    pub checks: Vec<ReadinessCheck>,
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

    pub fn get_category_demos(&self, conn: &Connection) -> Vec<CategoryDemo> {
        let billing_events = audit_count(conn, "plan_changed");
        let handoffs = scalar_u64(conn, "SELECT COUNT(*) FROM human_handoffs");
        let traces = scalar_u64(conn, "SELECT COUNT(*) FROM execution_traces");
        let creator_projects = scalar_u64(conn, "SELECT COUNT(*) FROM creator_projects");
        let microtasks = scalar_u64(conn, "SELECT COUNT(*) FROM microtasks");
        let escrows = scalar_u64(conn, "SELECT COUNT(*) FROM escrows");
        let org_catalog = scalar_u64(conn, "SELECT COUNT(*) FROM org_marketplace_listings WHERE approved = 1");

        vec![
            CategoryDemo {
                id: "billing-hardening".to_string(),
                category: "operations".to_string(),
                title: "Billing and upgrade enforcement".to_string(),
                readiness: if billing_events > 0 { "ready" } else { "partial" }.to_string(),
                evidence: format!("{} completed plan changes captured in audit log", billing_events),
                steps: vec![
                    "Trigger a plan-limited action from a free workspace.".to_string(),
                    "Open the billing checkout flow.".to_string(),
                    "Validate plan change and updated usage state.".to_string(),
                ],
            },
            CategoryDemo {
                id: "handoff-debugger".to_string(),
                category: "operations".to_string(),
                title: "Human handoff with debugger trace".to_string(),
                readiness: if handoffs > 0 && traces > 0 { "ready" } else { "partial" }.to_string(),
                evidence: format!("{} handoffs and {} traces available for replay", handoffs, traces),
                steps: vec![
                    "Create an escalation from a low-confidence task.".to_string(),
                    "Assign it to a human and attach notes.".to_string(),
                    "Review the matching execution trace.".to_string(),
                ],
            },
            CategoryDemo {
                id: "creator-economy".to_string(),
                category: "creator".to_string(),
                title: "Creator project to microtask escrow".to_string(),
                readiness: if creator_projects > 0 && microtasks > 0 && escrows > 0 {
                    "ready"
                } else {
                    "partial"
                }
                .to_string(),
                evidence: format!(
                    "{} creator projects, {} microtasks, {} escrows recorded",
                    creator_projects, microtasks, escrows
                ),
                steps: vec![
                    "Run creator test and package a project.".to_string(),
                    "Publish a microtask linked to the project.".to_string(),
                    "Create and release escrow after completion.".to_string(),
                ],
            },
            CategoryDemo {
                id: "partner-distribution".to_string(),
                category: "partner".to_string(),
                title: "Partner-branded distribution".to_string(),
                readiness: if org_catalog > 0 { "ready" } else { "partial" }.to_string(),
                evidence: format!("{} approved org marketplace listings available for bundle generation", org_catalog),
                steps: vec![
                    "Select the partner org and branding variant.".to_string(),
                    "Prepare the OEM distribution bundle.".to_string(),
                    "Verify catalog, branding and updater metadata in the manifest.".to_string(),
                ],
            },
        ]
    }

    pub fn definitive_readiness(&self, conn: &Connection) -> DefinitiveReadinessReport {
        let reliability = HealthDashboard::reliability_report(conn, 30);
        let creator_projects = scalar_u64(conn, "SELECT COUNT(*) FROM creator_projects");
        let tests = scalar_u64(conn, "SELECT COUNT(*) FROM test_run_history");
        let partner_bundles = scalar_u64(
            conn,
            "SELECT COUNT(*) FROM hardware_partners WHERE distribution_bundle_path IS NOT NULL",
        );
        let swarms = scalar_u64(conn, "SELECT COUNT(*) FROM swarm_tasks");
        let docs_ready = u64::from(
            std::path::Path::new("docs/deployment-runbooks.md").exists()
                && std::path::Path::new("docs/platform_standardization.md").exists(),
        );

        let checks = vec![
            readiness_check(
                "reliability",
                reliability.overall_status != "breached",
                format!(
                    "SLO status={} objectives_breached={}",
                    reliability.overall_status,
                    reliability.breached_objectives.len()
                ),
                if reliability.breached_objectives.is_empty() {
                    vec![]
                } else {
                    reliability.breached_objectives.clone()
                },
            ),
            readiness_check(
                "creator_economy",
                creator_projects > 0 && tests > 0,
                format!("{} creator projects and {} test runs tracked", creator_projects, tests),
                missing_gaps(
                    creator_projects == 0,
                    "No creator projects have been persisted yet",
                    tests == 0,
                    "No creator tests have been recorded yet",
                ),
            ),
            readiness_check(
                "partner_distribution",
                partner_bundles > 0,
                format!("{} partner distribution bundles prepared", partner_bundles),
                if partner_bundles == 0 {
                    vec!["No partner distribution bundle has been generated yet".to_string()]
                } else {
                    vec![]
                },
            ),
            readiness_check(
                "swarm_runtime",
                swarms > 0,
                format!("{} swarm task records present", swarms),
                if swarms == 0 {
                    vec!["Swarm runtime has not recorded a task yet".to_string()]
                } else {
                    vec![]
                },
            ),
            readiness_check(
                "operator_docs",
                docs_ready > 0,
                "Deployment and platform runbooks exist".to_string(),
                if docs_ready == 0 {
                    vec!["Operational docs are missing".to_string()]
                } else {
                    vec![]
                },
            ),
        ];

        let ready_checks = checks.iter().filter(|check| check.status == "ready").count() as u64;
        let warning_checks = checks.iter().filter(|check| check.status == "warning").count() as u64;
        let blocked_checks = checks.iter().filter(|check| check.status == "blocked").count() as u64;
        let overall_status = if blocked_checks > 0 {
            "blocked"
        } else if warning_checks > 0 {
            "warning"
        } else {
            "ready"
        };

        DefinitiveReadinessReport {
            generated_at: chrono::Utc::now().to_rfc3339(),
            overall_status: overall_status.to_string(),
            ready_checks,
            warning_checks,
            blocked_checks,
            checks,
        }
    }
}

fn readiness_check(name: &str, condition: bool, evidence: String, open_gaps: Vec<String>) -> ReadinessCheck {
    let status = if condition {
        "ready"
    } else if open_gaps.is_empty() {
        "warning"
    } else {
        "blocked"
    };
    ReadinessCheck {
        name: name.to_string(),
        status: status.to_string(),
        evidence,
        open_gaps,
    }
}

fn missing_gaps(
    first_missing: bool,
    first_message: &str,
    second_missing: bool,
    second_message: &str,
) -> Vec<String> {
    let mut gaps = Vec::new();
    if first_missing {
        gaps.push(first_message.to_string());
    }
    if second_missing {
        gaps.push(second_message.to_string());
    }
    gaps
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

    #[test]
    fn category_demos_and_readiness_use_real_tables() {
        let conn = setup_conn();
        conn.execute(
            "CREATE TABLE creator_projects (id TEXT PRIMARY KEY)",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE test_run_history (run_id TEXT PRIMARY KEY)",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE hardware_partners (
                id TEXT PRIMARY KEY,
                distribution_bundle_path TEXT
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE swarm_tasks (id TEXT PRIMARY KEY)",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE microtasks (id TEXT PRIMARY KEY)",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE escrows (id TEXT PRIMARY KEY)",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE org_marketplace_listings (
                id TEXT PRIMARY KEY,
                approved INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )
        .unwrap();

        conn.execute("INSERT INTO creator_projects (id) VALUES ('p1')", []).unwrap();
        conn.execute("INSERT INTO test_run_history (run_id) VALUES ('r1')", []).unwrap();
        conn.execute(
            "INSERT INTO hardware_partners (id, distribution_bundle_path) VALUES ('hp1', 'bundle.json')",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO swarm_tasks (id) VALUES ('s1')", []).unwrap();
        conn.execute("INSERT INTO microtasks (id) VALUES ('m1')", []).unwrap();
        conn.execute("INSERT INTO escrows (id) VALUES ('e1')", []).unwrap();
        conn.execute(
            "INSERT INTO org_marketplace_listings (id, approved) VALUES ('l1', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tasks (id, status) VALUES ('t3', 'completed')",
            [],
        )
        .unwrap();

        let dashboard = IPODashboard::new();
        let demos = dashboard.get_category_demos(&conn);
        let readiness = dashboard.definitive_readiness(&conn);

        assert_eq!(demos.len(), 4);
        assert!(demos.iter().any(|demo| demo.id == "partner-distribution"));
        assert!(readiness.checks.iter().any(|check| check.name == "partner_distribution"));
    }
}
