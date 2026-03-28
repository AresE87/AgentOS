use crate::brain::Gateway;
use crate::config::Settings;
use crate::eyes::{capture, vision};
use crate::memory::Database;
use crate::pipeline::executor;
use crate::types::*;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::Emitter;
use tracing::{info, warn};

const MAX_STEPS: u32 = 20;

/// Run a complete PC control task
pub async fn run_task(
    task_id: &str,
    description: &str,
    settings: &Settings,
    kill_switch: &Arc<AtomicBool>,
    screenshots_dir: &Path,
    db_path: &Path,
    app_handle: &tauri::AppHandle,
) -> Result<TaskExecutionResult, String> {
    let start = Instant::now();
    let gateway = Gateway::new(settings);
    let mut step_history: Vec<StepRecord> = Vec::new();
    let mut total_cost = 0.0;

    let max_steps = MAX_STEPS;

    info!(task_id, description, "Starting PC task execution");

    for step_number in 0..max_steps {
        // Check kill switch
        if kill_switch.load(Ordering::Relaxed) {
            warn!(task_id, "Kill switch activated, aborting task");
            update_task_status(db_path, task_id, "killed");
            return Err("Kill switch activated".to_string());
        }

        // 1. Capture current screen
        let screenshot = tokio::task::spawn_blocking({
            let sd = screenshots_dir.to_path_buf();
            move || {
                let data = capture::capture_full_screen().map_err(|e| e.to_string())?;
                let path = capture::save_screenshot(&data, &sd).map_err(|e| e.to_string())?;
                let b64 = capture::to_base64_jpeg(&data, 80).map_err(|e| e.to_string())?;
                Ok::<_, String>((path, b64))
            }
        })
        .await
        .map_err(|e| e.to_string())??;

        let (screenshot_path, screenshot_b64) = screenshot;

        // Emit progress event
        let _ = app_handle.emit(
            "agent:step_started",
            serde_json::json!({
                "task_id": task_id,
                "step_number": step_number,
            }),
        );

        // 2. Ask vision LLM what to do next
        let action = vision::plan_next_action(
            &screenshot_b64,
            description,
            &step_history,
            settings,
            &gateway,
        )
        .await;

        let action = match action {
            Ok(a) => a,
            Err(e) => {
                warn!(task_id, step_number, error = %e, "Vision LLM failed");
                // Try to continue with a wait if LLM fails
                if step_number < 3 {
                    AgentAction::Wait { ms: 1000 }
                } else {
                    update_task_status(db_path, task_id, "failed");
                    return Err(format!("Vision LLM failed: {}", e));
                }
            }
        };

        // 3. Check for task completion
        if matches!(action, AgentAction::TaskComplete { .. }) {
            info!(task_id, step_number, "Task completed by agent");
            let result = ExecutionResult {
                method: ExecutionMethod::Api,
                success: true,
                output: if let AgentAction::TaskComplete { ref summary } = action {
                    Some(summary.clone())
                } else {
                    None
                },
                screenshot_path: Some(screenshot_path.to_string_lossy().to_string()),
                duration_ms: 0,
            };
            step_history.push(StepRecord {
                step_number,
                action,
                result,
                screenshot_path: Some(screenshot_path.to_string_lossy().to_string()),
            });
            break;
        }

        // 4. Execute action
        let exec_result = executor::execute(&action, settings.cli_timeout, kill_switch).await;

        let result = match exec_result {
            Ok(r) => r,
            Err(e) => {
                warn!(task_id, step_number, error = %e, "Action execution failed");
                ExecutionResult {
                    method: ExecutionMethod::Screen,
                    success: false,
                    output: Some(e),
                    screenshot_path: None,
                    duration_ms: 0,
                }
            }
        };

        // 5. Save step to database
        save_step(
            db_path,
            task_id,
            step_number,
            &action,
            &screenshot_path,
            &result,
        );

        // 6. Emit step completed
        let _ = app_handle.emit(
            "agent:step_completed",
            serde_json::json!({
                "task_id": task_id,
                "step_number": step_number,
                "success": result.success,
                "screenshot_path": screenshot_path.to_string_lossy(),
            }),
        );

        // 7. Record step in history
        step_history.push(StepRecord {
            step_number,
            action,
            result,
            screenshot_path: Some(screenshot_path.to_string_lossy().to_string()),
        });

        // 8. Wait for UI to settle
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    let duration_ms = start.elapsed().as_millis() as u64;
    let success = step_history
        .last()
        .map(|s| matches!(s.action, AgentAction::TaskComplete { .. }))
        .unwrap_or(false);

    update_task_status(db_path, task_id, if success { "completed" } else { "failed" });

    Ok(TaskExecutionResult {
        task_id: task_id.to_string(),
        success,
        steps: step_history,
        total_cost,
        duration_ms,
    })
}

fn update_task_status(db_path: &Path, task_id: &str, status: &str) {
    if let Ok(db) = Database::new(db_path) {
        let _ = db.update_task_status(task_id, status);
    }
}

fn save_step(
    db_path: &Path,
    task_id: &str,
    step_number: u32,
    action: &AgentAction,
    screenshot_path: &Path,
    result: &ExecutionResult,
) {
    if let Ok(db) = Database::new(db_path) {
        let action_type = match action {
            AgentAction::Click { .. } => "click",
            AgentAction::DoubleClick { .. } => "double_click",
            AgentAction::RightClick { .. } => "right_click",
            AgentAction::Type { .. } => "type",
            AgentAction::KeyCombo { .. } => "key_combo",
            AgentAction::Scroll { .. } => "scroll",
            AgentAction::RunCommand { .. } => "run_command",
            AgentAction::Wait { .. } => "wait",
            AgentAction::Screenshot => "screenshot",
            AgentAction::TaskComplete { .. } => "task_complete",
        };
        let description = serde_json::to_string(action).unwrap_or_default();
        let exec_method = match result.method {
            ExecutionMethod::Api => "api",
            ExecutionMethod::Terminal => "terminal",
            ExecutionMethod::Screen => "screen",
        };

        let _ = db.insert_task_step(
            task_id,
            step_number,
            action_type,
            &description,
            &screenshot_path.to_string_lossy(),
            exec_method,
            result.success,
            result.duration_ms,
        );
    }
}
