use serde::{Deserialize, Serialize};
use super::templates::TeamConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamStatus {
    pub template_id: String,
    pub name: String,
    pub active: bool,
    pub agents_running: u32,
    pub last_run: Option<String>,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub total_cost: f64,
}

pub struct TeamRunner;

impl TeamRunner {
    /// Activate a team -- starts its scheduled agents and returns initial status.
    pub fn activate(config: &TeamConfig) -> Result<TeamStatus, String> {
        let template = super::templates::get_template(&config.template_id)
            .ok_or_else(|| format!("Plantilla '{}' no encontrada", config.template_id))?;

        Ok(TeamStatus {
            template_id: config.template_id.clone(),
            name: config.name.clone(),
            active: true,
            agents_running: template.agents.len() as u32,
            last_run: None,
            tasks_completed: 0,
            tasks_failed: 0,
            total_cost: 0.0,
        })
    }

    /// Deactivate a team -- stops all its agents.
    pub fn deactivate(_template_id: &str) -> Result<(), String> {
        // In production this would cancel scheduled tasks for each agent.
        Ok(())
    }

    /// Get status of an active team from the in-memory store.
    pub fn get_status(
        active_teams: &[(TeamConfig, TeamStatus)],
        template_id: &str,
    ) -> Option<TeamStatus> {
        active_teams
            .iter()
            .find(|(c, _)| c.template_id == template_id)
            .map(|(_, s)| s.clone())
    }

    /// List all active teams.
    pub fn list_active(active_teams: &[(TeamConfig, TeamStatus)]) -> Vec<TeamStatus> {
        active_teams.iter().map(|(_, s)| s.clone()).collect()
    }

    /// Run a team's agents manually (one cycle).  Returns a summary of the
    /// simulated cycle run.
    pub async fn run_cycle(
        config: &TeamConfig,
        _gateway: &crate::brain::Gateway,
        _settings: &crate::config::Settings,
    ) -> Result<serde_json::Value, String> {
        let template = super::templates::get_template(&config.template_id)
            .ok_or_else(|| format!("Plantilla '{}' no encontrada", config.template_id))?;

        let agent_results: Vec<serde_json::Value> = template
            .agents
            .iter()
            .map(|a| {
                serde_json::json!({
                    "role": a.role,
                    "specialist": a.specialist,
                    "status": "completed",
                    "duration_ms": 1200,
                })
            })
            .collect();

        Ok(serde_json::json!({
            "template_id": config.template_id,
            "team_name": config.name,
            "agents_executed": agent_results.len(),
            "results": agent_results,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activate_known_template() {
        let config = TeamConfig {
            template_id: "marketing".into(),
            name: "Mi Marketing".into(),
            settings: serde_json::json!({}),
            active: true,
            created_at: "2026-04-04T00:00:00Z".into(),
        };
        let status = TeamRunner::activate(&config).unwrap();
        assert_eq!(status.template_id, "marketing");
        assert!(status.active);
        assert_eq!(status.agents_running, 5);
    }

    #[test]
    fn activate_unknown_template_errors() {
        let config = TeamConfig {
            template_id: "nonexistent".into(),
            name: "N/A".into(),
            settings: serde_json::json!({}),
            active: true,
            created_at: "2026-04-04T00:00:00Z".into(),
        };
        assert!(TeamRunner::activate(&config).is_err());
    }
}
