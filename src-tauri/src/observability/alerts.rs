use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub condition: AlertCondition,
    pub severity: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AlertCondition {
    #[serde(rename = "error_rate")]
    ErrorRate { threshold_pct: f64 },
    #[serde(rename = "provider_down")]
    ProviderDown { provider: String },
    #[serde(rename = "disk_space")]
    DiskSpace { min_gb: f64 },
    #[serde(rename = "task_failure_streak")]
    TaskFailureStreak { count: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentRunbook {
    pub id: String,
    pub rule_id: String,
    pub title: String,
    pub summary: String,
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub rule_id: String,
    pub rule_name: String,
    pub severity: String,
    pub message: String,
    pub triggered_at: String,
    pub acknowledged: bool,
    pub acknowledged_at: Option<String>,
    pub resolved_at: Option<String>,
    pub status: String,
    pub runbook_id: Option<String>,
    pub resolution_notes: Option<String>,
}

pub struct AlertManager {
    db_path: PathBuf,
}

impl AlertManager {
    pub fn new(db_path: PathBuf) -> Result<Self, String> {
        let manager = Self { db_path };
        let conn = manager.open()?;
        Self::ensure_tables(&conn)?;
        Self::seed_defaults(&conn)?;
        Ok(manager)
    }

    fn open(&self) -> Result<Connection, String> {
        let conn = Connection::open(&self.db_path).map_err(|e| e.to_string())?;
        Self::ensure_tables(&conn)?;
        Self::seed_defaults(&conn)?;
        Ok(conn)
    }

    fn ensure_tables(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS alert_rules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                condition_json TEXT NOT NULL,
                severity TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1
            );
            CREATE TABLE IF NOT EXISTS alerts (
                id TEXT PRIMARY KEY,
                rule_id TEXT NOT NULL,
                rule_name TEXT NOT NULL,
                severity TEXT NOT NULL,
                message TEXT NOT NULL,
                triggered_at TEXT NOT NULL,
                acknowledged INTEGER NOT NULL DEFAULT 0,
                acknowledged_at TEXT,
                resolved_at TEXT,
                status TEXT NOT NULL DEFAULT 'open',
                runbook_id TEXT,
                resolution_notes TEXT
            );
            CREATE TABLE IF NOT EXISTS incident_runbooks (
                id TEXT PRIMARY KEY,
                rule_id TEXT NOT NULL,
                title TEXT NOT NULL,
                summary TEXT NOT NULL,
                steps_json TEXT NOT NULL
            );",
        )
        .map_err(|e| e.to_string())
    }

    fn seed_defaults(conn: &Connection) -> Result<(), String> {
        let rules = default_rules();
        for rule in &rules {
            conn.execute(
                "INSERT INTO alert_rules (id, name, condition_json, severity, enabled)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    condition_json = excluded.condition_json,
                    severity = excluded.severity,
                    enabled = excluded.enabled",
                params![
                    rule.id,
                    rule.name,
                    serde_json::to_string(&rule.condition).map_err(|e| e.to_string())?,
                    rule.severity,
                    rule.enabled as i64,
                ],
            )
            .map_err(|e| e.to_string())?;
        }

        for runbook in default_runbooks() {
            conn.execute(
                "INSERT INTO incident_runbooks (id, rule_id, title, summary, steps_json)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET
                    rule_id = excluded.rule_id,
                    title = excluded.title,
                    summary = excluded.summary,
                    steps_json = excluded.steps_json",
                params![
                    runbook.id,
                    runbook.rule_id,
                    runbook.title,
                    runbook.summary,
                    serde_json::to_string(&runbook.steps).map_err(|e| e.to_string())?,
                ],
            )
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn open_incident(&self, rule_id: &str, message: &str) -> Result<Alert, String> {
        let conn = self.open()?;
        let rule = self
            .get_rule(rule_id)?
            .ok_or_else(|| format!("Alert rule not found: {}", rule_id))?;
        let runbook_id = self.get_runbook_for_rule(rule_id)?.map(|runbook| runbook.id);
        let alert = Alert {
            id: uuid::Uuid::new_v4().to_string(),
            rule_id: rule.id.clone(),
            rule_name: rule.name.clone(),
            severity: rule.severity.clone(),
            message: message.to_string(),
            triggered_at: chrono::Utc::now().to_rfc3339(),
            acknowledged: false,
            acknowledged_at: None,
            resolved_at: None,
            status: "open".to_string(),
            runbook_id,
            resolution_notes: None,
        };
        conn.execute(
            "INSERT INTO alerts
             (id, rule_id, rule_name, severity, message, triggered_at, acknowledged, acknowledged_at, resolved_at, status, runbook_id, resolution_notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                alert.id,
                alert.rule_id,
                alert.rule_name,
                alert.severity,
                alert.message,
                alert.triggered_at,
                alert.acknowledged as i64,
                alert.acknowledged_at,
                alert.resolved_at,
                alert.status,
                alert.runbook_id,
                alert.resolution_notes,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(alert)
    }

    pub fn acknowledge(&self, alert_id: &str) -> Result<(), String> {
        let conn = self.open()?;
        let changed = conn
            .execute(
                "UPDATE alerts
                 SET acknowledged = 1, acknowledged_at = ?2, status = 'acknowledged'
                 WHERE id = ?1 AND resolved_at IS NULL",
                params![alert_id, chrono::Utc::now().to_rfc3339()],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err(format!("Alert not found or already resolved: {}", alert_id));
        }
        Ok(())
    }

    pub fn resolve(&self, alert_id: &str, notes: Option<&str>) -> Result<(), String> {
        let conn = self.open()?;
        let changed = conn
            .execute(
                "UPDATE alerts
                 SET acknowledged = 1,
                     acknowledged_at = COALESCE(acknowledged_at, ?2),
                     resolved_at = ?2,
                     status = 'resolved',
                     resolution_notes = ?3
                 WHERE id = ?1",
                params![alert_id, chrono::Utc::now().to_rfc3339(), notes],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err(format!("Alert not found: {}", alert_id));
        }
        Ok(())
    }

    pub fn get_active(&self) -> Result<Vec<Alert>, String> {
        let conn = self.open()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, rule_id, rule_name, severity, message, triggered_at, acknowledged, acknowledged_at, resolved_at, status, runbook_id, resolution_notes
                 FROM alerts
                 WHERE resolved_at IS NULL
                 ORDER BY triggered_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let alerts = stmt.query_map([], map_alert)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(alerts)
    }

    pub fn get_all(&self) -> Result<Vec<Alert>, String> {
        let conn = self.open()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, rule_id, rule_name, severity, message, triggered_at, acknowledged, acknowledged_at, resolved_at, status, runbook_id, resolution_notes
                 FROM alerts
                 ORDER BY triggered_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let alerts = stmt.query_map([], map_alert)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(alerts)
    }

    pub fn get_rules(&self) -> Result<Vec<AlertRule>, String> {
        let conn = self.open()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, condition_json, severity, enabled
                 FROM alert_rules
                 ORDER BY name ASC",
            )
            .map_err(|e| e.to_string())?;
        let rules = stmt.query_map([], |row| {
            let condition_json: String = row.get(2)?;
            Ok(AlertRule {
                id: row.get(0)?,
                name: row.get(1)?,
                condition: serde_json::from_str(&condition_json).map_err(|err| {
                    rusqlite::Error::FromSqlConversionFailure(
                        2,
                        rusqlite::types::Type::Text,
                        Box::new(err),
                    )
                })?,
                severity: row.get(3)?,
                enabled: row.get::<_, i64>(4)? != 0,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
        Ok(rules)
    }

    pub fn get_runbooks(&self) -> Result<Vec<IncidentRunbook>, String> {
        let conn = self.open()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, rule_id, title, summary, steps_json
                 FROM incident_runbooks
                 ORDER BY title ASC",
            )
            .map_err(|e| e.to_string())?;
        let runbooks = stmt.query_map([], |row| {
            let steps_json: String = row.get(4)?;
            Ok(IncidentRunbook {
                id: row.get(0)?,
                rule_id: row.get(1)?,
                title: row.get(2)?,
                summary: row.get(3)?,
                steps: serde_json::from_str(&steps_json).map_err(|err| {
                    rusqlite::Error::FromSqlConversionFailure(
                        4,
                        rusqlite::types::Type::Text,
                        Box::new(err),
                    )
                })?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
        Ok(runbooks)
    }

    fn get_rule(&self, rule_id: &str) -> Result<Option<AlertRule>, String> {
        let conn = self.open()?;
        conn.query_row(
            "SELECT id, name, condition_json, severity, enabled
             FROM alert_rules
             WHERE id = ?1",
            params![rule_id],
            |row| {
                let condition_json: String = row.get(2)?;
                Ok(AlertRule {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    condition: serde_json::from_str(&condition_json).map_err(|err| {
                        rusqlite::Error::FromSqlConversionFailure(
                            2,
                            rusqlite::types::Type::Text,
                            Box::new(err),
                        )
                    })?,
                    severity: row.get(3)?,
                    enabled: row.get::<_, i64>(4)? != 0,
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())
    }

    fn get_runbook_for_rule(&self, rule_id: &str) -> Result<Option<IncidentRunbook>, String> {
        let conn = self.open()?;
        conn.query_row(
            "SELECT id, rule_id, title, summary, steps_json
             FROM incident_runbooks
             WHERE rule_id = ?1",
            params![rule_id],
            |row| {
                let steps_json: String = row.get(4)?;
                Ok(IncidentRunbook {
                    id: row.get(0)?,
                    rule_id: row.get(1)?,
                    title: row.get(2)?,
                    summary: row.get(3)?,
                    steps: serde_json::from_str(&steps_json).map_err(|err| {
                        rusqlite::Error::FromSqlConversionFailure(
                            4,
                            rusqlite::types::Type::Text,
                            Box::new(err),
                        )
                    })?,
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())
    }
}

fn map_alert(row: &rusqlite::Row<'_>) -> rusqlite::Result<Alert> {
    Ok(Alert {
        id: row.get(0)?,
        rule_id: row.get(1)?,
        rule_name: row.get(2)?,
        severity: row.get(3)?,
        message: row.get(4)?,
        triggered_at: row.get(5)?,
        acknowledged: row.get::<_, i64>(6)? != 0,
        acknowledged_at: row.get(7)?,
        resolved_at: row.get(8)?,
        status: row.get(9)?,
        runbook_id: row.get(10)?,
        resolution_notes: row.get(11)?,
    })
}

fn default_rules() -> Vec<AlertRule> {
    vec![
        AlertRule {
            id: "err-rate".into(),
            name: "High Error Rate".into(),
            condition: AlertCondition::ErrorRate {
                threshold_pct: 20.0,
            },
            severity: "warning".into(),
            enabled: true,
        },
        AlertRule {
            id: "disk-low".into(),
            name: "Low Disk Space".into(),
            condition: AlertCondition::DiskSpace { min_gb: 5.0 },
            severity: "critical".into(),
            enabled: true,
        },
        AlertRule {
            id: "fail-streak".into(),
            name: "Task Failure Streak".into(),
            condition: AlertCondition::TaskFailureStreak { count: 5 },
            severity: "warning".into(),
            enabled: true,
        },
    ]
}

fn default_runbooks() -> Vec<IncidentRunbook> {
    vec![
        IncidentRunbook {
            id: "runbook-err-rate".to_string(),
            rule_id: "err-rate".to_string(),
            title: "High Error Rate".to_string(),
            summary: "Investigate recent failures, provider outages and permission denials.".to_string(),
            steps: vec![
                "Open observability summary and review recent errors.".to_string(),
                "Check reliability report and failure concentration by capability.".to_string(),
                "If needed, create a human handoff for the failing workflow.".to_string(),
            ],
        },
        IncidentRunbook {
            id: "runbook-disk-low".to_string(),
            rule_id: "disk-low".to_string(),
            title: "Low Disk Space".to_string(),
            summary: "Protect local-first state before disk exhaustion causes data loss.".to_string(),
            steps: vec![
                "Export logs and prune non-essential artifacts.".to_string(),
                "Verify SQLite, screenshots and recordings directories.".to_string(),
                "Pause large local captures until free space recovers.".to_string(),
            ],
        },
        IncidentRunbook {
            id: "runbook-fail-streak".to_string(),
            rule_id: "fail-streak".to_string(),
            title: "Task Failure Streak".to_string(),
            summary: "Use retry, rollback and debugger traces to recover the failing path.".to_string(),
            steps: vec![
                "Inspect the last failed execution traces.".to_string(),
                "Retry a single task from the recovery surface.".to_string(),
                "If the issue came from a deployment, roll back the affected plugin or config.".to_string(),
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn incidents_and_runbooks_are_persisted() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("alerts.db");
        let manager = AlertManager::new(db_path).unwrap();

        let incident = manager
            .open_incident("err-rate", "Error rate crossed threshold")
            .unwrap();
        let active = manager.get_active().unwrap();
        let runbooks = manager.get_runbooks().unwrap();

        assert_eq!(active.len(), 1);
        assert!(incident.runbook_id.is_some());
        assert!(runbooks.iter().any(|runbook| runbook.rule_id == "err-rate"));
    }

    #[test]
    fn acknowledge_and_resolve_update_persistent_status() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("alerts-resolve.db");
        let manager = AlertManager::new(db_path).unwrap();
        let incident = manager
            .open_incident("fail-streak", "Five tasks failed in a row")
            .unwrap();

        manager.acknowledge(&incident.id).unwrap();
        manager
            .resolve(&incident.id, Some("Recovered after retry and rollback"))
            .unwrap();

        let all = manager.get_all().unwrap();
        assert_eq!(all[0].status, "resolved");
        assert!(all[0].resolved_at.is_some());
    }
}
