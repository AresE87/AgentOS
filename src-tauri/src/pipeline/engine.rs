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
    _settings: &Settings,
    _kill_switch: &Arc<AtomicBool>,
    screenshots_dir: &Path,
    db_path: &Path,
    app_handle: &tauri::AppHandle,
) -> Option<Result<TaskExecutionResult, String>> {
    let start = Instant::now();
    let lower = description.to_lowercase();
    // Strip quotes from user input
    let lower = lower.trim_matches('"').trim_matches('"').trim_matches('"').trim();

    // Map natural language to executable programs
    let program = if lower.contains("abre cmd") || lower.contains("open cmd") || lower.contains("abrir cmd") {
        Some("cmd.exe")
    } else if lower.contains("abre notepad") || lower.contains("open notepad") || lower.contains("bloc de notas") {
        Some("notepad.exe")
    } else if lower.contains("abre calculadora") || lower.contains("open calculator") || lower.contains("calc") {
        Some("calc.exe")
    } else if lower.contains("abre explorador") || lower.contains("open explorer") || lower.contains("file explorer") {
        Some("explorer.exe")
    } else if lower.contains("abre paint") || lower.contains("open paint") {
        Some("mspaint.exe")
    } else if lower.contains("abre chrome") || lower.contains("open chrome") {
        Some("chrome.exe")
    } else if lower.contains("abre edge") || lower.contains("open edge") {
        Some("msedge.exe")
    } else if lower.contains("abre firefox") || lower.contains("open firefox") {
        Some("firefox.exe")
    } else if lower.contains("abre powershell") || lower.contains("open powershell") {
        Some("powershell.exe")
    } else if lower.contains("abre terminal") || lower.contains("open terminal") {
        Some("wt.exe") // Windows Terminal
    } else if lower.starts_with("run ") {
        // Use lower (already trimmed) instead of description to avoid byte-offset mismatch
        Some(&lower[4..])
    } else if lower.starts_with("ejecuta ") {
        Some(&lower[8..])
    } else if lower.starts_with("abre ") {
        Some(&lower[5..])
    } else if lower.starts_with("open ") {
        Some(&lower[5..])
    } else {
        None
    };

    let program = program?.trim();
    if program.is_empty() {
        return None;
    }

    info!(task_id, program = %program, "Direct execution: launching app");

    let _ = app_handle.emit("agent:step_started", serde_json::json!({
        "task_id": task_id, "step_number": 0,
    }));

    // Use launch_app which does Start-Process (non-blocking, visible window)
    let exec_result = hands::cli::launch_app(program).await;

    let result = match exec_result {
        Ok(output) => {
            info!(task_id, exit_code = output.exit_code, "App launched successfully");
            ExecutionResult {
                method: ExecutionMethod::Terminal,
                success: output.exit_code == 0,
                output: Some(format!("Launched: {}", program)),
                screenshot_path: None,
                duration_ms: output.duration_ms,
            }
        }
        Err(e) => {
            warn!(task_id, error = %e, "launch_app failed");
            ExecutionResult {
                method: ExecutionMethod::Terminal,
                success: false,
                output: Some(format!("Failed to launch {}: {}", program, e)),
                screenshot_path: None,
                duration_ms: 0,
            }
        }
    };

    // Wait a moment for the app to appear, then take a screenshot
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let screenshot_path = tokio::task::spawn_blocking({
        let sd = screenshots_dir.to_path_buf();
        move || {
            capture::capture_full_screen()
                .ok()
                .and_then(|data| capture::save_screenshot(&data, &sd).ok())
        }
    }).await.ok().flatten();

    let sp = screenshot_path.as_ref().map(|p| p.to_string_lossy().to_string());
    let action = AgentAction::RunCommand {
        command: program.to_string(),
        shell: ShellType::PowerShell,
    };

    save_step(db_path, task_id, 0, &action, &screenshot_path.unwrap_or_default(), &result);

    let _ = app_handle.emit("agent:step_completed", serde_json::json!({
        "task_id": task_id, "step_number": 0, "success": result.success,
    }));

    update_task_status(db_path, task_id, if result.success { "completed" } else { "failed" });

    let step = StepRecord {
        step_number: 0,
        action: AgentAction::TaskComplete {
            summary: format!("Launched: {}", program),
        },
        result,
        screenshot_path: sp,
    };

    let was_success = step.result.success;
    Some(Ok(TaskExecutionResult {
        task_id: task_id.to_string(),
        success: was_success,
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
