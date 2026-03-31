use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdoptionMetrics {
    pub install_date: String,
    pub os: String,
    pub version: String,
    pub setup_completed: bool,
    pub first_task_sent: bool,
    pub tasks_day_1: u32,
    pub tasks_day_7: u32,
    pub tasks_day_30: u32,
    pub features_used: Vec<String>,
    pub providers_configured: u32,
    pub channels_configured: u32,
    pub personas_created: u32,
    pub playbooks_installed: u32,
}

impl AdoptionMetrics {
    pub fn collect(conn: &Connection) -> Self {
        let total_tasks: i64 = conn
            .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
            .unwrap_or(0);
        let providers: i64 = 0; // Count from settings
        let personas: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='agent_personas'",
                [],
                |r| r.get(0),
            )
            .and_then(|exists: i64| {
                if exists > 0 {
                    conn.query_row("SELECT COUNT(*) FROM agent_personas", [], |r| r.get(0))
                } else {
                    Ok(0)
                }
            })
            .unwrap_or(0);

        let mut features = vec!["chat".to_string()];
        if total_tasks > 0 {
            features.push("tasks".into());
        }
        if personas > 0 {
            features.push("personas".into());
        }

        Self {
            install_date: "".into(),
            os: std::env::consts::OS.to_string(),
            version: "1.1.0".into(),
            setup_completed: true,
            first_task_sent: total_tasks > 0,
            tasks_day_1: 0,
            tasks_day_7: 0,
            tasks_day_30: total_tasks as u32,
            features_used: features,
            providers_configured: providers as u32,
            channels_configured: 0,
            personas_created: personas as u32,
            playbooks_installed: 0,
        }
    }
}
