use crate::billing::{Plan, PlanType};
use crate::enterprise::audit::AuditLog;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanUsageSnapshot {
    pub plan_type: String,
    pub active_days: u64,
    pub tasks: u64,
    pub tokens: u64,
    pub limit_hit_days: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingVariantMetrics {
    pub variant: String,
    pub checkout_requests: u64,
    pub completed_upgrades: u64,
    pub conversion_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueFunnel {
    pub blocked_attempts: u64,
    pub checkout_requests: u64,
    pub completed_upgrades: u64,
    pub conversion_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueMetrics {
    pub total_tasks: u64,
    pub completed_tasks: u64,
    pub failed_tasks: u64,
    pub paid_plan_days: u64,
    pub blocked_attempts: u64,
    pub upgrade_intents: u64,
    pub completed_upgrades: u64,
    pub plan_usage: Vec<PlanUsageSnapshot>,
    pub variants: Vec<PricingVariantMetrics>,
    pub funnel: RevenueFunnel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChurnRisk {
    pub user_id: String,
    pub risk_score: f64,
    pub reasons: Vec<String>,
    pub suggested_intervention: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsellCandidate {
    pub user_id: String,
    pub current_plan: String,
    pub suggested_plan: String,
    pub reason: String,
    pub estimated_revenue_increase: f64,
}

pub struct RevenueOptimizer;

impl RevenueOptimizer {
    pub fn new() -> Self {
        Self
    }

    pub fn calculate_metrics(&self, conn: &Connection) -> RevenueMetrics {
        let total_tasks = scalar_u64(conn, "SELECT COUNT(*) FROM tasks");
        let completed_tasks = scalar_u64(conn, "SELECT COUNT(*) FROM tasks WHERE status = 'completed'");
        let failed_tasks = scalar_u64(conn, "SELECT COUNT(*) FROM tasks WHERE status = 'failed'");
        let blocked_attempts = self.audit_count(conn, "billing_limit_blocked");
        let upgrade_intents = self.audit_count(conn, "upgrade_checkout_requested");
        let completed_upgrades = self.completed_upgrades(conn);
        let plan_usage = self.plan_usage(conn);
        let paid_plan_days = plan_usage
            .iter()
            .filter(|row| row.plan_type != "free")
            .map(|row| row.active_days)
            .sum();
        let variants = self.variant_metrics(conn);
        let conversion_rate = percentage(completed_upgrades, upgrade_intents);

        RevenueMetrics {
            total_tasks,
            completed_tasks,
            failed_tasks,
            paid_plan_days,
            blocked_attempts,
            upgrade_intents,
            completed_upgrades,
            plan_usage,
            variants,
            funnel: RevenueFunnel {
                blocked_attempts,
                checkout_requests: upgrade_intents,
                completed_upgrades,
                conversion_rate,
            },
        }
    }

    pub fn predict_churn(&self, conn: &Connection) -> Vec<ChurnRisk> {
        let recent_tasks = scalar_u64(
            conn,
            "SELECT COUNT(*) FROM tasks WHERE created_at >= datetime('now', '-7 days')",
        );
        let previous_tasks = scalar_u64(
            conn,
            "SELECT COUNT(*) FROM tasks WHERE created_at < datetime('now', '-7 days') AND created_at >= datetime('now', '-14 days')",
        );
        let blocked_attempts = self.audit_count_since(conn, "billing_limit_blocked", "-30 days");
        let failed_tasks = scalar_u64(
            conn,
            "SELECT COUNT(*) FROM tasks WHERE status = 'failed' AND created_at >= datetime('now', '-14 days')",
        );

        if recent_tasks == 0 && previous_tasks == 0 && blocked_attempts == 0 && failed_tasks == 0 {
            return Vec::new();
        }

        let activity_drop = if previous_tasks == 0 {
            0.0
        } else {
            1.0 - (recent_tasks as f64 / previous_tasks as f64)
        };

        let mut reasons = Vec::new();
        if activity_drop > 0.35 {
            reasons.push(format!(
                "Task activity fell from {} to {} in the last 7 days",
                previous_tasks, recent_tasks
            ));
        }
        if blocked_attempts > 0 {
            reasons.push(format!(
                "{} blocked billing-limit attempts in the last 30 days",
                blocked_attempts
            ));
        }
        if failed_tasks > 0 {
            reasons.push(format!("{} failed tasks in the last 14 days", failed_tasks));
        }

        if reasons.is_empty() {
            return Vec::new();
        }

        let risk_score = (activity_drop.max(0.0) * 0.6
            + (blocked_attempts.min(5) as f64 / 5.0) * 0.25
            + (failed_tasks.min(5) as f64 / 5.0) * 0.15)
            .min(0.99);

        vec![ChurnRisk {
            user_id: "local-workspace".to_string(),
            risk_score,
            reasons,
            suggested_intervention:
                "Review blocked usage and failed tasks, then surface the matching plan or onboarding path."
                    .to_string(),
        }]
    }

    pub fn get_upsell_candidates(&self, conn: &Connection) -> Vec<UpsellCandidate> {
        let current_plan = latest_plan(conn).unwrap_or_else(|| "free".to_string());
        let blocked_attempts = self.audit_count_since(conn, "billing_limit_blocked", "-30 days");
        let usage = self.plan_usage(conn);
        let Some(current_usage) = usage.iter().find(|row| row.plan_type == current_plan) else {
            return Vec::new();
        };

        let current_plan_type = plan_type_from_str(&current_plan);
        let current_limits = Plan::from_type(&current_plan_type);

        let needs_upgrade = blocked_attempts > 0
            || current_usage.limit_hit_days > 0
            || (current_limits.tasks_per_day != u32::MAX
                && current_usage.tasks >= (current_limits.tasks_per_day as u64 * current_usage.active_days));

        if !needs_upgrade || matches!(current_plan_type, PlanType::Team) {
            return Vec::new();
        }

        let (suggested_plan, revenue_delta) = match current_plan_type {
            PlanType::Free => ("pro", 29.0),
            PlanType::Pro => ("team", 70.0),
            PlanType::Team => ("team", 0.0),
        };

        vec![UpsellCandidate {
            user_id: "local-workspace".to_string(),
            current_plan: current_plan.clone(),
            suggested_plan: suggested_plan.to_string(),
            reason: format!(
                "{} blocked attempts and {} limit-hit days on {}",
                blocked_attempts, current_usage.limit_hit_days, current_plan
            ),
            estimated_revenue_increase: revenue_delta,
        }]
    }

    fn completed_upgrades(&self, conn: &Connection) -> u64 {
        AuditLog::get_by_event_type(conn, "plan_changed", 500)
            .unwrap_or_default()
            .into_iter()
            .filter(|entry| {
                parse_details(&entry.details)
                    .and_then(|details| details.get("plan_type").and_then(Value::as_str).map(str::to_string))
                    .map(|plan| plan == "pro" || plan == "team")
                    .unwrap_or(false)
            })
            .count() as u64
    }

    fn audit_count(&self, conn: &Connection, event_type: &str) -> u64 {
        AuditLog::get_by_event_type(conn, event_type, 500)
            .map(|rows| rows.len() as u64)
            .unwrap_or(0)
    }

    fn audit_count_since(&self, conn: &Connection, event_type: &str, window: &str) -> u64 {
        let sql = format!(
            "SELECT COUNT(*) FROM audit_log WHERE event_type = ?1 AND timestamp >= datetime('now', '{}')",
            window
        );
        conn.query_row(&sql, [event_type], |row| row.get::<_, i64>(0))
            .unwrap_or(0) as u64
    }

    fn variant_metrics(&self, conn: &Connection) -> Vec<PricingVariantMetrics> {
        let mut rows: BTreeMap<String, (u64, u64)> = BTreeMap::new();
        for entry in AuditLog::get_by_event_type(conn, "upgrade_checkout_requested", 500)
            .unwrap_or_default()
        {
            let variant = parse_details(&entry.details)
                .and_then(|details| details.get("variant").and_then(Value::as_str).map(str::to_string))
                .unwrap_or_else(|| "control".to_string());
            rows.entry(variant).or_insert((0, 0)).0 += 1;
        }

        for entry in AuditLog::get_by_event_type(conn, "plan_changed", 500).unwrap_or_default() {
            let Some(details) = parse_details(&entry.details) else {
                continue;
            };
            let Some(plan) = details.get("plan_type").and_then(Value::as_str) else {
                continue;
            };
            if plan != "pro" && plan != "team" {
                continue;
            }
            let variant = details
                .get("variant")
                .and_then(Value::as_str)
                .unwrap_or("control")
                .to_string();
            rows.entry(variant).or_insert((0, 0)).1 += 1;
        }

        rows.into_iter()
            .map(|(variant, (checkout_requests, completed_upgrades))| PricingVariantMetrics {
                variant,
                checkout_requests,
                completed_upgrades,
                conversion_rate: percentage(completed_upgrades, checkout_requests),
            })
            .collect()
    }

    fn plan_usage(&self, conn: &Connection) -> Vec<PlanUsageSnapshot> {
        let mut stmt = match conn.prepare(
            "SELECT plan_type, COUNT(*) as active_days, SUM(tasks_count) as tasks, SUM(tokens_used) as tokens
             FROM daily_usage
             GROUP BY plan_type
             ORDER BY active_days DESC, plan_type ASC",
        ) {
            Ok(stmt) => stmt,
            Err(_) => return Vec::new(),
        };

        let rows = stmt
            .query_map([], |row| {
                let plan_type: String = row.get(0)?;
                let active_days = row.get::<_, i64>(1)? as u64;
                let tasks = row.get::<_, i64>(2)? as u64;
                let tokens = row.get::<_, i64>(3)? as u64;
                let limits = Plan::from_type(&plan_type_from_str(&plan_type));
                let limit_hit_days = count_limit_hit_days(conn, &plan_type, limits.tasks_per_day, limits.tokens_per_day);
                Ok(PlanUsageSnapshot {
                    plan_type,
                    active_days,
                    tasks,
                    tokens,
                    limit_hit_days,
                })
            })
            .ok();

        rows.map(|iter| iter.flatten().collect()).unwrap_or_default()
    }
}

fn scalar_u64(conn: &Connection, sql: &str) -> u64 {
    conn.query_row(sql, [], |row| row.get::<_, i64>(0))
        .unwrap_or(0) as u64
}

fn percentage(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        (numerator as f64 / denominator as f64) * 100.0
    }
}

fn parse_details(details: &str) -> Option<Value> {
    serde_json::from_str(details).ok()
}

fn latest_plan(conn: &Connection) -> Option<String> {
    conn.query_row(
        "SELECT plan_type FROM daily_usage ORDER BY date DESC LIMIT 1",
        [],
        |row| row.get::<_, String>(0),
    )
    .ok()
}

fn plan_type_from_str(value: &str) -> PlanType {
    match value {
        "pro" => PlanType::Pro,
        "team" => PlanType::Team,
        _ => PlanType::Free,
    }
}

fn count_limit_hit_days(
    conn: &Connection,
    plan_type: &str,
    tasks_per_day: u32,
    tokens_per_day: u64,
) -> u64 {
    let mut stmt = match conn.prepare(
        "SELECT tasks_count, tokens_used FROM daily_usage WHERE plan_type = ?1",
    ) {
        Ok(stmt) => stmt,
        Err(_) => return 0,
    };

    let rows = stmt
        .query_map([plan_type], |row| {
            Ok((row.get::<_, i64>(0)? as u64, row.get::<_, i64>(1)? as u64))
        })
        .ok();

    rows.map(|iter| {
        iter.flatten()
            .filter(|(tasks, tokens)| {
                let task_hit = tasks_per_day != u32::MAX && *tasks >= tasks_per_day as u64;
                let token_hit = tokens_per_day != u64::MAX && *tokens >= tokens_per_day;
                task_hit || token_hit
            })
            .count() as u64
    })
    .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enterprise::audit::AuditLog;

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE tasks (
                id TEXT PRIMARY KEY,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                status TEXT NOT NULL DEFAULT 'completed'
            );
            CREATE TABLE daily_usage (
                date TEXT PRIMARY KEY,
                tasks_count INTEGER NOT NULL DEFAULT 0,
                tokens_used INTEGER NOT NULL DEFAULT 0,
                plan_type TEXT NOT NULL DEFAULT 'free'
            );",
        )
        .unwrap();
        AuditLog::ensure_table(&conn).unwrap();
        conn
    }

    #[test]
    fn calculates_real_metrics_from_usage_and_audit_log() {
        let conn = setup_conn();
        conn.execute(
            "INSERT INTO tasks (id, created_at, status) VALUES ('t1', datetime('now'), 'completed')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tasks (id, created_at, status) VALUES ('t2', datetime('now'), 'failed')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO daily_usage (date, tasks_count, tokens_used, plan_type) VALUES ('2026-03-30', 20, 50000, 'free')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO daily_usage (date, tasks_count, tokens_used, plan_type) VALUES ('2026-03-31', 35, 60000, 'free')",
            [],
        )
        .unwrap();
        AuditLog::log(
            &conn,
            "billing_limit_blocked",
            serde_json::json!({ "plan_type": "free", "kind": "task_limit" }),
        )
        .unwrap();
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

        let optimizer = RevenueOptimizer::new();
        let metrics = optimizer.calculate_metrics(&conn);

        assert_eq!(metrics.total_tasks, 2);
        assert_eq!(metrics.failed_tasks, 1);
        assert_eq!(metrics.blocked_attempts, 1);
        assert_eq!(metrics.upgrade_intents, 1);
        assert_eq!(metrics.completed_upgrades, 1);
        assert_eq!(metrics.funnel.conversion_rate, 100.0);
        assert_eq!(metrics.plan_usage[0].limit_hit_days, 2);
        assert_eq!(metrics.variants[0].variant, "limit-focused");
    }

    #[test]
    fn upsell_candidates_require_real_pressure() {
        let conn = setup_conn();
        conn.execute(
            "INSERT INTO daily_usage (date, tasks_count, tokens_used, plan_type) VALUES ('2026-03-31', 21, 100, 'free')",
            [],
        )
        .unwrap();
        AuditLog::log(
            &conn,
            "billing_limit_blocked",
            serde_json::json!({ "plan_type": "free", "kind": "task_limit" }),
        )
        .unwrap();

        let optimizer = RevenueOptimizer::new();
        let candidates = optimizer.get_upsell_candidates(&conn);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].suggested_plan, "pro");
    }
}
