use crate::approvals::ApprovalRequest;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceCheck {
    pub id: String,
    pub framework: String,
    pub check_name: String,
    pub status: String,
    pub details: String,
    pub checked_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComplianceFilters {
    pub days: Option<i64>,
    pub agent_name: Option<String>,
    pub status: Option<String>,
    pub user: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComplianceSummary {
    pub approvals_count: usize,
    pub handoffs_count: usize,
    pub executions_count: usize,
    pub period_start: String,
    pub period_end: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceEvidence {
    pub id: String,
    pub source: String,
    pub occurred_at: String,
    pub actor: Option<String>,
    pub agent_name: Option<String>,
    pub status: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceArtifact {
    pub format: String,
    pub filename: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub framework: String,
    pub checks: Vec<ComplianceCheck>,
    pub score: f64,
    pub generated_at: String,
    pub filters: ComplianceFilters,
    pub summary: ComplianceSummary,
    pub evidence: Vec<ComplianceEvidence>,
    pub artifacts: Vec<ComplianceArtifact>,
}

pub struct ComplianceReporter {
    db_path: PathBuf,
    reports: Vec<ComplianceReport>,
}

impl ComplianceReporter {
    pub fn new(db_path: PathBuf) -> Self {
        Self {
            db_path,
            reports: Vec::new(),
        }
    }

    pub fn run_framework_report(
        &mut self,
        framework: &str,
        filters: ComplianceFilters,
        approvals: &[ApprovalRequest],
    ) -> Result<ComplianceReport, String> {
        let now = chrono::Utc::now();
        let days = filters.days.unwrap_or(7).max(1);
        let period_start = now - chrono::Duration::days(days);
        let period_start_str = period_start.to_rfc3339();
        let period_end_str = now.to_rfc3339();
        let conn = Connection::open(&self.db_path)
            .map_err(|e| format!("Failed to open compliance DB: {}", e))?;

        let approval_evidence =
            filter_approvals(approvals, &filters, &period_start_str, &period_end_str);
        let handoff_evidence = query_handoffs(&conn, &filters, &period_start_str, &period_end_str)?;
        let execution_evidence =
            query_execution_traces(&conn, &filters, &period_start_str, &period_end_str)?;

        let mut evidence = Vec::new();
        evidence.extend(approval_evidence.clone());
        evidence.extend(handoff_evidence.clone());
        evidence.extend(execution_evidence.clone());
        evidence.sort_by(|a, b| b.occurred_at.cmp(&a.occurred_at));

        let inventory_ok = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
            .map(|_| true)
            .unwrap_or(false);

        let checks =
            vec![
            make_check(
                framework,
                "Approval evidence captured",
                status_from_count(approval_evidence.len()),
                &format!(
                    "{} approval events matched the selected filters between {} and {}.",
                    approval_evidence.len(),
                    period_start_str,
                    period_end_str
                ),
            ),
            make_check(
                framework,
                "Human handoff evidence captured",
                status_from_count(handoff_evidence.len()),
                &format!(
                    "{} handoff records were loaded from the persistent queue.",
                    handoff_evidence.len()
                ),
            ),
            make_check(
                framework,
                "Execution evidence captured",
                status_from_count(execution_evidence.len()),
                &format!(
                    "{} execution traces were loaded from the debugger store.",
                    execution_evidence.len()
                ),
            ),
            make_check(
                framework,
                "Export artifacts generated",
                if !evidence.is_empty() { "pass" } else { "warning" },
                "JSON and CSV artifacts are generated from the report payload and evidence rows.",
            ),
            make_check(
                framework,
                "Audit storage accessible",
                if inventory_ok { "pass" } else { "fail" },
                "The compliance reporter successfully opened the local SQLite store.",
            ),
        ];

        let summary = ComplianceSummary {
            approvals_count: approval_evidence.len(),
            handoffs_count: handoff_evidence.len(),
            executions_count: execution_evidence.len(),
            period_start: period_start_str.clone(),
            period_end: period_end_str.clone(),
        };

        let mut report = ComplianceReport {
            framework: framework.to_uppercase(),
            checks,
            score: 0.0,
            generated_at: now.to_rfc3339(),
            filters,
            summary,
            evidence,
            artifacts: Vec::new(),
        };
        report.score = score(&report.checks);
        report.artifacts = build_artifacts(&report)?;
        self.reports.push(report.clone());
        Ok(report)
    }

    pub fn get_all_reports(&self) -> &[ComplianceReport] {
        &self.reports
    }
}

fn make_check(framework: &str, name: &str, status: &str, details: &str) -> ComplianceCheck {
    ComplianceCheck {
        id: uuid::Uuid::new_v4().to_string(),
        framework: framework.to_uppercase(),
        check_name: name.to_string(),
        status: status.to_string(),
        details: details.to_string(),
        checked_at: chrono::Utc::now().to_rfc3339(),
    }
}

fn score(checks: &[ComplianceCheck]) -> f64 {
    if checks.is_empty() {
        return 0.0;
    }

    let total = checks
        .iter()
        .map(|check| match check.status.as_str() {
            "pass" => 1.0,
            "warning" => 0.5,
            _ => 0.0,
        })
        .sum::<f64>();
    (total / checks.len() as f64) * 100.0
}

fn status_from_count(count: usize) -> &'static str {
    if count > 0 {
        "pass"
    } else {
        "warning"
    }
}

fn filter_approvals(
    approvals: &[ApprovalRequest],
    filters: &ComplianceFilters,
    period_start: &str,
    period_end: &str,
) -> Vec<ComplianceEvidence> {
    approvals
        .iter()
        .filter(|approval| {
            approval.requested_at.as_str() >= period_start
                && approval.requested_at.as_str() <= period_end
        })
        .filter(|approval| {
            filters
                .status
                .as_deref()
                .map(|status| format!("{:?}", approval.status).eq_ignore_ascii_case(status))
                .unwrap_or(true)
        })
        .filter(|approval| {
            filters
                .user
                .as_deref()
                .map(|user| {
                    approval
                        .response_by
                        .as_deref()
                        .map(|value| value.eq_ignore_ascii_case(user))
                        .unwrap_or(false)
                })
                .unwrap_or(true)
        })
        .map(|approval| ComplianceEvidence {
            id: approval.id.clone(),
            source: "approval".to_string(),
            occurred_at: approval.requested_at.clone(),
            actor: approval.response_by.clone(),
            agent_name: None,
            status: format!("{:?}", approval.status).to_lowercase(),
            summary: format!("{} ({})", approval.action_description, approval.risk_level),
        })
        .collect()
}

fn query_handoffs(
    conn: &Connection,
    filters: &ComplianceFilters,
    period_start: &str,
    period_end: &str,
) -> Result<Vec<ComplianceEvidence>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, task_description, status, assigned_to, created_at, reason
             FROM human_handoffs
             WHERE created_at >= ?1 AND created_at <= ?2
             ORDER BY created_at DESC",
        )
        .map_err(|e| format!("Failed to query handoffs for compliance: {}", e))?;

    let rows = stmt
        .query_map(params![period_start, period_end], |row| {
            Ok(ComplianceEvidence {
                id: row.get(0)?,
                source: "handoff".to_string(),
                summary: format!(
                    "{} [{}]",
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(5)?
                ),
                status: row.get(2)?,
                actor: row.get(3)?,
                occurred_at: row.get(4)?,
                agent_name: None,
            })
        })
        .map_err(|e| format!("Failed to map handoffs for compliance: {}", e))?;

    Ok(rows
        .flatten()
        .filter(|row| {
            filters
                .status
                .as_deref()
                .map(|status| row.status.eq_ignore_ascii_case(status))
                .unwrap_or(true)
        })
        .filter(|row| {
            filters
                .user
                .as_deref()
                .map(|user| {
                    row.actor
                        .as_deref()
                        .map(|actor| actor.eq_ignore_ascii_case(user))
                        .unwrap_or(false)
                })
                .unwrap_or(true)
        })
        .collect())
}

fn query_execution_traces(
    conn: &Connection,
    filters: &ComplianceFilters,
    period_start: &str,
    period_end: &str,
) -> Result<Vec<ComplianceEvidence>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, task_id, agent_name, status, created_at
             FROM execution_traces
             WHERE created_at >= ?1 AND created_at <= ?2
             ORDER BY created_at DESC",
        )
        .map_err(|e| format!("Failed to query execution traces for compliance: {}", e))?;

    let rows = stmt
        .query_map(params![period_start, period_end], |row| {
            Ok(ComplianceEvidence {
                id: row.get(0)?,
                source: "execution".to_string(),
                summary: format!("Task {}", row.get::<_, String>(1)?),
                agent_name: Some(row.get(2)?),
                status: row.get(3)?,
                actor: None,
                occurred_at: row.get(4)?,
            })
        })
        .map_err(|e| format!("Failed to map execution traces for compliance: {}", e))?;

    Ok(rows
        .flatten()
        .filter(|row| {
            filters
                .agent_name
                .as_deref()
                .map(|agent| {
                    row.agent_name
                        .as_deref()
                        .map(|value| value.eq_ignore_ascii_case(agent))
                        .unwrap_or(false)
                })
                .unwrap_or(true)
        })
        .filter(|row| {
            filters
                .status
                .as_deref()
                .map(|status| row.status.eq_ignore_ascii_case(status))
                .unwrap_or(true)
        })
        .collect())
}

fn build_artifacts(report: &ComplianceReport) -> Result<Vec<ComplianceArtifact>, String> {
    let json_content = serde_json::to_string_pretty(report)
        .map_err(|e| format!("Failed to export JSON report: {}", e))?;

    let mut csv = String::from("source,id,occurred_at,status,actor,agent_name,summary\n");
    for item in &report.evidence {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            csv_escape(&item.source),
            csv_escape(&item.id),
            csv_escape(&item.occurred_at),
            csv_escape(&item.status),
            csv_escape(item.actor.as_deref().unwrap_or("")),
            csv_escape(item.agent_name.as_deref().unwrap_or("")),
            csv_escape(&item.summary),
        ));
    }

    Ok(vec![
        ComplianceArtifact {
            format: "json".to_string(),
            filename: format!("compliance-{}-report.json", report.framework.to_lowercase()),
            content: json_content,
        },
        ComplianceArtifact {
            format: "csv".to_string(),
            filename: format!(
                "compliance-{}-evidence.csv",
                report.framework.to_lowercase()
            ),
            content: csv,
        },
    ])
}

fn csv_escape(value: &str) -> String {
    let escaped = value.replace('"', "\"\"");
    format!("\"{}\"", escaped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debugger::trace::AgentDebugger;
    use crate::escalation::EscalationManager;
    use rusqlite::params;
    use tempfile::tempdir;

    #[test]
    fn report_uses_real_handoffs_traces_and_approvals() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("compliance.db");
        let conn = Connection::open(&db_path).unwrap();
        EscalationManager::init_db(&conn).unwrap();
        AgentDebugger::init_db(&conn).unwrap();

        conn.execute(
            "INSERT INTO human_handoffs (
                id, reason, task_description, attempts_json, analysis, task_id, chain_id,
                original_input, task_status, task_output, task_steps_json, chain_subtasks_json,
                evidence_json, status, assigned_to, created_at, updated_at
            ) VALUES (?1, ?2, ?3, '[]', ?4, NULL, NULL, NULL, NULL, NULL, '[]', '[]', '[]', ?5, ?6, ?7, ?7)",
            params![
                "handoff-1",
                "\"low_confidence\"",
                "Review invoice anomaly",
                "Human review required",
                "pending_handoff",
                "ops",
                chrono::Utc::now().to_rfc3339(),
            ],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO execution_traces (
                id, task_id, agent_name, model, status, created_at, updated_at, total_duration_ms, total_cost
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, 1200, 0.14)",
            params![
                "trace-1",
                "task-1",
                "PC Controller",
                "anthropic/sonnet",
                "completed",
                chrono::Utc::now().to_rfc3339(),
            ],
        )
        .unwrap();

        let approvals = vec![ApprovalRequest {
            id: "approval-1".to_string(),
            action_description: "Approve payout batch".to_string(),
            risk_level: crate::approvals::ActionRisk::High,
            status: crate::approvals::ApprovalStatus::Approved,
            requested_at: chrono::Utc::now().to_rfc3339(),
            responded_at: Some(chrono::Utc::now().to_rfc3339()),
            response_by: Some("ops".to_string()),
        }];

        let mut reporter = ComplianceReporter::new(db_path);
        let report = reporter
            .run_framework_report("gdpr", ComplianceFilters::default(), &approvals)
            .unwrap();

        assert_eq!(report.summary.approvals_count, 1);
        assert_eq!(report.summary.handoffs_count, 1);
        assert_eq!(report.summary.executions_count, 1);
        assert_eq!(report.artifacts.len(), 2);
        assert!(report.evidence.iter().any(|item| item.source == "approval"));
        assert!(report.evidence.iter().any(|item| item.source == "handoff"));
        assert!(report
            .evidence
            .iter()
            .any(|item| item.source == "execution"));
    }

    #[test]
    fn report_filters_by_agent_status_and_user() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("compliance-filtered.db");
        let conn = Connection::open(&db_path).unwrap();
        EscalationManager::init_db(&conn).unwrap();
        AgentDebugger::init_db(&conn).unwrap();

        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO human_handoffs (
                id, reason, task_description, attempts_json, analysis, task_id, chain_id,
                original_input, task_status, task_output, task_steps_json, chain_subtasks_json,
                evidence_json, status, assigned_to, created_at, updated_at
            ) VALUES (?1, ?2, ?3, '[]', ?4, NULL, NULL, NULL, NULL, NULL, '[]', '[]', '[]', ?5, ?6, ?7, ?7)",
            params![
                "handoff-ops",
                "\"user_request\"",
                "Manual override",
                "Ops handled case",
                "assigned_to_human",
                "ops",
                now,
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO execution_traces (
                id, task_id, agent_name, model, status, created_at, updated_at, total_duration_ms, total_cost
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, 500, 0.02)",
            params!["trace-a", "task-a", "PC Controller", "model", "completed", now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO execution_traces (
                id, task_id, agent_name, model, status, created_at, updated_at, total_duration_ms, total_cost
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, 500, 0.02)",
            params!["trace-b", "task-b", "Other Agent", "model", "failed", now],
        )
        .unwrap();

        let approvals = vec![
            ApprovalRequest {
                id: "approval-ops".to_string(),
                action_description: "Approve export".to_string(),
                risk_level: crate::approvals::ActionRisk::Medium,
                status: crate::approvals::ApprovalStatus::Approved,
                requested_at: now.clone(),
                responded_at: Some(now.clone()),
                response_by: Some("ops".to_string()),
            },
            ApprovalRequest {
                id: "approval-finance".to_string(),
                action_description: "Approve payout".to_string(),
                risk_level: crate::approvals::ActionRisk::High,
                status: crate::approvals::ApprovalStatus::Rejected,
                requested_at: now.clone(),
                responded_at: Some(now.clone()),
                response_by: Some("finance".to_string()),
            },
        ];

        let mut reporter = ComplianceReporter::new(db_path);
        let report = reporter
            .run_framework_report(
                "sox",
                ComplianceFilters {
                    days: Some(7),
                    agent_name: Some("PC Controller".to_string()),
                    status: Some("completed".to_string()),
                    user: Some("ops".to_string()),
                },
                &approvals,
            )
            .unwrap();

        assert_eq!(report.summary.approvals_count, 0);
        assert_eq!(report.summary.handoffs_count, 0);
        assert_eq!(report.summary.executions_count, 1);
        assert_eq!(report.evidence.len(), 1);
        assert_eq!(
            report.evidence[0].agent_name.as_deref(),
            Some("PC Controller")
        );
    }
}
