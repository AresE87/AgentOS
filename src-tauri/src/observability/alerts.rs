use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub condition: AlertCondition,
    pub severity: String, // "info", "warning", "critical"
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
pub struct Alert {
    pub id: String,
    pub rule_id: String,
    pub rule_name: String,
    pub severity: String,
    pub message: String,
    pub triggered_at: String,
    pub acknowledged: bool,
}

pub struct AlertManager {
    rules: Vec<AlertRule>,
    active_alerts: Vec<Alert>,
}

impl AlertManager {
    pub fn new() -> Self {
        Self {
            rules: vec![
                AlertRule {
                    id: "err-rate".into(),
                    name: "High Error Rate".into(),
                    condition: AlertCondition::ErrorRate { threshold_pct: 20.0 },
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
            ],
            active_alerts: vec![],
        }
    }

    pub fn check_disk_space(&mut self) -> Option<Alert> {
        // Check C: drive free space — placeholder for sysinfo or PowerShell
        None
    }

    pub fn add_alert(&mut self, rule_id: &str, message: &str) {
        if let Some(rule) = self.rules.iter().find(|r| r.id == rule_id) {
            self.active_alerts.push(Alert {
                id: uuid::Uuid::new_v4().to_string(),
                rule_id: rule_id.to_string(),
                rule_name: rule.name.clone(),
                severity: rule.severity.clone(),
                message: message.to_string(),
                triggered_at: chrono::Utc::now().to_rfc3339(),
                acknowledged: false,
            });
        }
    }

    pub fn acknowledge(&mut self, alert_id: &str) {
        if let Some(alert) = self.active_alerts.iter_mut().find(|a| a.id == alert_id) {
            alert.acknowledged = true;
        }
    }

    pub fn get_active(&self) -> Vec<&Alert> {
        self.active_alerts.iter().filter(|a| !a.acknowledged).collect()
    }

    pub fn get_all(&self) -> &[Alert] {
        &self.active_alerts
    }

    pub fn get_rules(&self) -> &[AlertRule] {
        &self.rules
    }
}
