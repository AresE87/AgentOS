use crate::brain::Gateway;
use crate::config::Settings;
use crate::eyes::{capture, vision};
use crate::hands;
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
    let mut step_history: Vec<StepRecord> = Vec::new();

    info!(task_id, description, "Starting PC task execution");

    // STRATEGY 1: Try direct command execution first (fast path)
    if let Some(result) = try_direct_execution(task_id, description, settings, kill_switch, screenshots_dir, db_path, app_handle).await {
        return result;
    }

    // STRATEGY 2: Vision-guided autonomous loop (slow path)
    let gateway = Gateway::new(settings);

    for step_number in 0..MAX_STEPS {
        if kill_switch.load(Ordering::Relaxed) {
            update_task_status(db_path, task_id, "killed");
            return Err("Kill switch activated".to_string());
        }

        // 1. Capture screen
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

        let _ = app_handle.emit("agent:step_started", serde_json::json!({
            "task_id": task_id, "step_number": step_number,
        }));

        // 2. Ask vision LLM
        let action = vision::plan_next_action(
            &screenshot_b64, description, &step_history, settings, &gateway,
        ).await;

        let action = match action {
            Ok(a) => a,
            Err(e) => {
                warn!(task_id, step_number, error = %e, "Vision LLM failed");
                if step_number < 2 {
                    AgentAction::Wait { ms: 1000 }
                } else {
                    update_task_status(db_path, task_id, "failed");
                    return Err(format!("Vision LLM failed: {}", e));
                }
            }
        };

        if matches!(action, AgentAction::TaskComplete { .. }) {
            let result = ExecutionResult {
                method: ExecutionMethod::Api, success: true,
                output: if let AgentAction::TaskComplete { ref summary } = action { Some(summary.clone()) } else { None },
                screenshot_path: Some(screenshot_path.to_string_lossy().to_string()),
                duration_ms: 0,
            };
            step_history.push(StepRecord {
                step_number, action, result,
                screenshot_path: Some(screenshot_path.to_string_lossy().to_string()),
            });
            break;
        }

        let exec_result = executor::execute(&action, settings.cli_timeout, kill_switch).await;
        let result = match exec_result {
            Ok(r) => r,
            Err(e) => ExecutionResult {
                method: ExecutionMethod::Screen, success: false,
                output: Some(e), screenshot_path: None, duration_ms: 0,
            },
        };

        save_step(db_path, task_id, step_number, &action, &screenshot_path, &result);

        let _ = app_handle.emit("agent:step_completed", serde_json::json!({
            "task_id": task_id, "step_number": step_number,
            "success": result.success,
            "screenshot_path": screenshot_path.to_string_lossy(),
        }));

        step_history.push(StepRecord {
            step_number, action, result,
            screenshot_path: Some(screenshot_path.to_string_lossy().to_string()),
        });

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    let duration_ms = start.elapsed().as_millis() as u64;
    let success = step_history.last()
        .map(|s| matches!(s.action, AgentAction::TaskComplete { .. }) || s.result.success)
        .unwrap_or(false);

    update_task_status(db_path, task_id, if success { "completed" } else { "failed" });

    Ok(TaskExecutionResult {
        task_id: task_id.to_string(), success,
        steps: step_history, total_cost: 0.0, duration_ms,
    })
}

/// Try to execute the task directly without vision (for simple commands)
async fn try_direct_execution(
    task_id: &str,
    description: &str,
    settings: &Settings,
    kill_switch: &Arc<AtomicBool>,
    screenshots_dir: &Path,
    db_path: &Path,
    app_handle: &tauri::AppHandle,
) -> Option<Result<TaskExecutionResult, String>> {
    let start = Instant::now();
    let lower = description.to_lowercase();

    // Map common natural language commands to actual shell commands
    let command = if lower.contains("abre cmd") || lower.contains("open cmd") || lower.contains("abrir cmd") {
        Some("cmd.exe")
    } else if lower.contains("abre notepad") || lower.contains("open notepad") || lower.contains("bloc de notas") {
        Some("notepad.exe")
    } else if lower.contains("abre calculadora") || lower.contains("open calculator") {
        Some("calc.exe")
    } else if lower.contains("abre explorador") || lower.contains("open explorer") || lower.contains("open file explorer") {
        Some("explorer.exe")
    } else if lower.contains("abre paint") || lower.contains("open paint") {
        Some("mspaint.exe")
    } else if lower.contains("abre chrome") || lower.contains("open chrome") {
        Some("start chrome")
    } else if lower.contains("abre edge") || lower.contains("open edge") {
        Some("start msedge")
    } else if lower.contains("abre firefox") || lower.contains("open firefox") {
        Some("start firefox")
    } else {
        None
    };

    // Check if it's a direct shell command request
    let shell_cmd = if let Some(cmd) = command {
        Some(cmd.to_string())
    } else if lower.starts_with("run ") || lower.starts_with("ejecuta ") {
        // Extract the command after "run " or "ejecuta "
        let cmd = if lower.starts_with("run ") {
            &description[4..]
        } else {
            &description[8..]
        };
        Some(cmd.trim().to_string())
    } else {
        None
    };

    let shell_cmd = shell_cmd?;

    info!(task_id, command = %shell_cmd, "Direct execution: running command");

    let _ = app_handle.emit("agent:step_started", serde_json::json!({
        "task_id": task_id, "step_number": 0,
    }));

    // Execute the command
    let action = AgentAction::RunCommand {
        command: shell_cmd.clone(),
        shell: ShellType::PowerShell,
    };

    let exec_result = executor::execute(&action, settings.cli_timeout, kill_switch).await;

    let result = match exec_result {
        Ok(r) => r,
        Err(e) => {
            // Try with Start-Process as fallback
            let fallback = AgentAction::RunCommand {
                command: format!("Start-Process '{}'", shell_cmd),
                shell: ShellType::PowerShell,
            };
            match executor::execute(&fallback, settings.cli_timeout, kill_switch).await {
                Ok(r) => r,
                Err(_) => {
                    update_task_status(db_path, task_id, "failed");
                    return Some(Err(format!("Command failed: {}", e)));
                }
            }
        }
    };

    // Take a screenshot after execution
    let screenshot_path = tokio::task::spawn_blocking({
        let sd = screenshots_dir.to_path_buf();
        move || {
            if let Ok(data) = capture::capture_full_screen() {
                capture::save_screenshot(&data, &sd).ok()
            } else {
                None
            }
        }
    }).await.ok().flatten();

    let sp = screenshot_path.as_ref().map(|p| p.to_string_lossy().to_string());

    save_step(db_path, task_id, 0, &action, &screenshot_path.unwrap_or_default(), &result);

    let _ = app_handle.emit("agent:step_completed", serde_json::json!({
        "task_id": task_id, "step_number": 0, "success": result.success,
    }));

    let _ = app_handle.emit("agent:task_completed", serde_json::json!({
        "task_id": task_id, "success": result.success, "steps": 1,
        "duration_ms": start.elapsed().as_millis() as u64,
    }));

    update_task_status(db_path, task_id, if result.success { "completed" } else { "failed" });

    let step = StepRecord {
        step_number: 0,
        action: AgentAction::TaskComplete {
            summary: format!("Executed: {}", shell_cmd),
        },
        result,
        screenshot_path: sp,
    };

    Some(Ok(TaskExecutionResult {
        task_id: task_id.to_string(),
        success: true,
        steps: vec![step],
        total_cost: 0.0,
        duration_ms: start.elapsed().as_millis() as u64,
    }))
}

fn update_task_status(db_path: &Path, task_id: &str, status: &str) {
    if let Ok(db) = Database::new(db_path) {
        let _ = db.update_task_status(task_id, status);
    }
}

fn save_step(
    db_path: &Path, task_id: &str, step_number: u32,
    action: &AgentAction, screenshot_path: &Path, result: &ExecutionResult,
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
            task_id, step_number, action_type, &description,
            &screenshot_path.to_string_lossy(), exec_method,
            result.success, result.duration_ms,
        );
    }
}
