use crate::brain::Gateway;
use crate::config::Settings;
use crate::memory::Database;
use crate::agents;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use tauri::Emitter;
use tracing::{info, warn};

/// Execute a chain of subtasks for a complex task.
/// Each subtask is executed sequentially, passing accumulated context from
/// previous subtasks to the next one. Results are stored in the DB and
/// events are emitted so the Board UI updates in real-time.
pub async fn execute_chain(
    chain_id: &str,
    original_task: &str,
    subtask_descriptions: Vec<String>,
    settings: &Settings,
    kill_switch: &Arc<AtomicBool>,
    db_path: &Path,
    app_handle: &tauri::AppHandle,
) -> Result<String, String> {
    let gateway = Gateway::new(settings);
    let mut accumulated_context = String::new();
    let mut total_cost = 0.0;

    // Create subtasks in DB
    for (i, desc) in subtask_descriptions.iter().enumerate() {
        let subtask_id = format!("{}-{}", chain_id, i + 1);
        if let Ok(db) = Database::new(db_path) {
            let _ = db.insert_chain_subtask(&subtask_id, chain_id, i as i32, desc);
        }
    }

    // Emit chain started
    let _ = app_handle.emit("chain:started", serde_json::json!({
        "chain_id": chain_id,
        "original_task": original_task,
        "subtask_count": subtask_descriptions.len(),
    }));

    // Log chain start
    if let Ok(db) = Database::new(db_path) {
        let _ = db.insert_chain_event(
            chain_id, "Orchestrator", "orchestrator", "chain_started",
            &format!("Chain started: {} subtasks", subtask_descriptions.len()),
            None,
        );
    }

    // Execute each subtask sequentially
    for (i, desc) in subtask_descriptions.iter().enumerate() {
        if kill_switch.load(Ordering::Relaxed) {
            warn!(chain_id, "Kill switch activated, stopping chain");
            break;
        }

        let subtask_id = format!("{}-{}", chain_id, i + 1);
        let start = Instant::now();

        // Find best agent for this subtask
        let registry = agents::AgentRegistry::new();
        let agent = registry.find_best(desc);
        let agent_name = format!("{} ({:?})", agent.name, agent.level);
        let agent_system_prompt = agent.system_prompt.clone();

        info!(chain_id, subtask_id = %subtask_id, agent = %agent_name, "Executing subtask");

        // Update status to running
        if let Ok(db) = Database::new(db_path) {
            let _ = db.update_subtask_status(
                &subtask_id, "running", "Working...", "", 0.0, 0, &agent_name, "",
            );
        }

        // Emit subtask started
        let _ = app_handle.emit("chain:update", serde_json::json!({
            "chain_id": chain_id,
            "subtask_id": subtask_id,
            "status": "running",
            "description": desc,
            "agent_name": agent_name,
        }));

        // Build prompt with accumulated context from previous subtasks
        let prompt = if accumulated_context.is_empty() {
            format!(
                "Original task: \"{}\"\n\nYour specific subtask: {}\n\nComplete this subtask thoroughly. Provide your output as clear, detailed text.",
                original_task, desc
            )
        } else {
            format!(
                "Original task: \"{}\"\n\nResults from previous subtasks:\n{}\n\nYour specific subtask: {}\n\nUse the information from previous subtasks to complete yours. Provide your output as clear, detailed text.",
                original_task, accumulated_context, desc
            )
        };

        // Execute via LLM
        let system_prompt = format!(
            "You are {}. {}\nComplete the assigned subtask thoroughly and provide detailed output.",
            agent_name, agent_system_prompt
        );
        let result = gateway
            .complete_as_agent(&prompt, &system_prompt, settings)
            .await;

        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(response) => {
                let cost = response.cost;
                total_cost += cost;

                accumulated_context.push_str(&format!(
                    "\n--- {} (by {}) ---\n{}\n",
                    desc, agent_name, response.content
                ));

                // Update subtask as done
                if let Ok(db) = Database::new(db_path) {
                    let _ = db.update_subtask_status(
                        &subtask_id, "done", "Completed",
                        &response.content, cost, duration_ms,
                        &agent_name, &response.model,
                    );
                    let _ = db.insert_chain_event(
                        chain_id, &agent_name, "agent_level", "subtask_completed",
                        &format!("Completed: {}", desc), None,
                    );
                }

                // Emit subtask completed
                let _ = app_handle.emit("chain:update", serde_json::json!({
                    "chain_id": chain_id,
                    "subtask_id": subtask_id,
                    "status": "done",
                    "output_preview": &response.content[..response.content.len().min(200)],
                    "cost": cost,
                    "duration_ms": duration_ms,
                }));

                info!(chain_id, subtask_id = %subtask_id, cost, duration_ms, "Subtask completed");
            }
            Err(e) => {
                warn!(chain_id, subtask_id = %subtask_id, error = %e, "Subtask failed");

                if let Ok(db) = Database::new(db_path) {
                    let _ = db.update_subtask_status(
                        &subtask_id, "failed", &format!("Error: {}", e),
                        "", 0.0, duration_ms, &agent_name, "",
                    );
                    let _ = db.insert_chain_event(
                        chain_id, &agent_name, "agent_level", "subtask_failed",
                        &format!("Failed: {} - {}", desc, e), None,
                    );
                }

                let _ = app_handle.emit("chain:update", serde_json::json!({
                    "chain_id": chain_id,
                    "subtask_id": subtask_id,
                    "status": "failed",
                    "error": e.to_string(),
                }));

                // Continue with remaining subtasks (partial success)
                accumulated_context.push_str(&format!(
                    "\n--- {} (FAILED) ---\nError: {}\n",
                    desc, e
                ));
            }
        }
    }

    // Complete chain
    if let Ok(db) = Database::new(db_path) {
        let _ = db.complete_chain(chain_id, total_cost);
        let _ = db.insert_chain_event(
            chain_id, "Orchestrator", "orchestrator", "chain_completed",
            &format!("Chain completed, total cost: ${:.4}", total_cost), None,
        );
    }

    // Emit chain finished
    let _ = app_handle.emit("chain:finished", serde_json::json!({
        "chain_id": chain_id,
        "total_cost": total_cost,
        "success": true,
    }));

    Ok(accumulated_context)
}
