use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

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
    /// Activate a team -- starts its scheduled agents and stores into active_teams.
    pub fn activate(
        config: &TeamConfig,
        active_teams: &mut Vec<(TeamConfig, TeamStatus)>,
    ) -> Result<TeamStatus, String> {
        let template = super::templates::get_template(&config.template_id)
            .ok_or_else(|| format!("Plantilla '{}' no encontrada", config.template_id))?;

        // Remove any previous entry for this template
        active_teams.retain(|(c, _)| c.template_id != config.template_id);

        let status = TeamStatus {
            template_id: config.template_id.clone(),
            name: config.name.clone(),
            active: true,
            agents_running: template.agents.len() as u32,
            last_run: None,
            tasks_completed: 0,
            tasks_failed: 0,
            total_cost: 0.0,
        };

        active_teams.push((config.clone(), status.clone()));
        Ok(status)
    }

    /// Deactivate a team -- removes it from active_teams and marks inactive.
    pub fn deactivate(
        template_id: &str,
        active_teams: &mut Vec<(TeamConfig, TeamStatus)>,
    ) -> Result<TeamStatus, String> {
        let idx = active_teams
            .iter()
            .position(|(c, _)| c.template_id == template_id)
            .ok_or_else(|| format!("Equipo '{}' no esta activo", template_id))?;

        let (_, mut status) = active_teams.remove(idx);
        status.active = false;
        status.agents_running = 0;
        Ok(status)
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

    /// Run a team's agents via the real agent loop (one cycle).
    pub async fn run_cycle(
        config: &TeamConfig,
        gateway: &crate::brain::Gateway,
        settings: &crate::config::Settings,
        tool_registry: &crate::tools::ToolRegistry,
        db_path: &std::path::Path,
        kill_switch: &Arc<AtomicBool>,
        event_emitter: Option<&tauri::AppHandle>,
    ) -> Result<serde_json::Value, String> {
        let template = super::templates::get_template(&config.template_id)
            .ok_or_else(|| format!("Plantilla '{}' no encontrada", config.template_id))?;

        // Build a prompt from the team's agents describing their collective task
        let team_task = format!(
            "Ejecutar un ciclo del equipo '{}'. Cada agente debe realizar su tarea:\n{}",
            template.name,
            template
                .agents
                .iter()
                .map(|a| format!("- {} ({}): {}", a.role, a.specialist, a.description))
                .collect::<Vec<_>>()
                .join("\n")
        );

        // Use the first agent's role + system prompt to drive execution
        let agent = &template.agents[0];
        let system_prompt = format!(
            "Sos el {} del equipo {}. Tu rol: {}. Ejecuta las tareas del ciclo.",
            agent.role, template.name, agent.description
        );

        let tool_defs: Vec<serde_json::Value> = tool_registry
            .definitions()
            .iter()
            .map(|d| {
                serde_json::json!({
                    "name": d.name,
                    "description": d.description,
                    "input_schema": d.input_schema
                })
            })
            .collect();

        let app_data_dir = db_path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf();

        let ctx = crate::tools::ToolContext {
            agent_name: agent.specialist.clone(),
            task_id: format!("team_{}_{}", config.template_id, uuid::Uuid::new_v4()),
            db_path: db_path.to_path_buf(),
            app_data_dir,
            kill_switch: kill_switch.clone(),
            execution_mode: crate::tools::ExecutionMode::default(),
        };

        let runtime =
            crate::agent_loop::AgentRuntime::new(crate::agent_loop::AgentLoopConfig {
                max_iterations: 15,
                max_tokens_per_turn: 4096,
                compact_threshold_tokens: 80_000,
            });

        let result = runtime
            .run_turn(
                &team_task,
                &system_prompt,
                &tool_defs,
                tool_registry,
                &ctx,
                gateway,
                settings,
                kill_switch,
                event_emitter,
                None,
                None,
                None,
            )
            .await?;

        Ok(serde_json::json!({
            "team": template.name,
            "template_id": config.template_id,
            "cycle_completed": true,
            "iterations": result.iterations,
            "tools_used": result.tool_calls_made.len(),
            "output": result.text,
            "total_input_tokens": result.total_input_tokens,
            "total_output_tokens": result.total_output_tokens,
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
        let mut active_teams = Vec::new();
        let status = TeamRunner::activate(&config, &mut active_teams).unwrap();
        assert_eq!(status.template_id, "marketing");
        assert!(status.active);
        assert_eq!(status.agents_running, 5);
        assert_eq!(active_teams.len(), 1);
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
        let mut active_teams = Vec::new();
        assert!(TeamRunner::activate(&config, &mut active_teams).is_err());
    }

    #[test]
    fn deactivate_removes_from_vec() {
        let config = TeamConfig {
            template_id: "marketing".into(),
            name: "Mi Marketing".into(),
            settings: serde_json::json!({}),
            active: true,
            created_at: "2026-04-04T00:00:00Z".into(),
        };
        let mut active_teams = Vec::new();
        TeamRunner::activate(&config, &mut active_teams).unwrap();
        assert_eq!(active_teams.len(), 1);
        let status = TeamRunner::deactivate("marketing", &mut active_teams).unwrap();
        assert!(!status.active);
        assert_eq!(active_teams.len(), 0);
    }

    #[test]
    fn deactivate_nonexistent_errors() {
        let mut active_teams = Vec::new();
        assert!(TeamRunner::deactivate("nonexistent", &mut active_teams).is_err());
    }
}
