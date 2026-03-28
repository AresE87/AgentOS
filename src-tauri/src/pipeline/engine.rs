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

const COMMAND_TRANSLATOR_PROMPT: &str = r#"You are a Windows PC command translator. The user gives you a task in natural language. You must respond with ONLY a PowerShell command that accomplishes the task. No explanation, no markdown, just the raw command.

Rules:
- Use Start-Process to open GUI applications (so they get their own window)
- Use explorer.exe to open folders
- Use $env:USERPROFILE for the user's home directory
- For web URLs, use Start-Process with the URL
- For complex multi-step tasks, chain commands with semicolons
- NEVER use destructive commands (rm -rf, format, del /s)
- If the task is impossible via command line, respond with: IMPOSSIBLE: <reason>

Examples:
User: "abre la carpeta descargas"
Command: Start-Process explorer.exe "$env:USERPROFILE\Downloads"

User: "abre notepad y escribe hola mundo"
Command: Start-Process notepad.exe; Start-Sleep -Seconds 1

User: "busca archivos pdf en mi escritorio"
Command: Get-ChildItem "$env:USERPROFILE\Desktop" -Filter "*.pdf" -Recurse | Select-Object FullName, Length, LastWriteTime

User: "dime cuanto espacio libre tiene mi disco"
Command: Get-PSDrive C | Select-Object @{N='Free(GB)';E={[math]::Round($_.Free/1GB,2)}}, @{N='Used(GB)';E={[math]::Round($_.Used/1GB,2)}}

User: "abre youtube en chrome"
Command: Start-Process chrome.exe "https://www.youtube.com"

User: "crea una carpeta llamada proyectos en el escritorio"
Command: New-Item -Path "$env:USERPROFILE\Desktop\proyectos" -ItemType Directory -Force

User: "muestra los procesos que mas memoria usan"
Command: Get-Process | Sort-Object WorkingSet64 -Descending | Select-Object -First 10 Name, @{N='Memory(MB)';E={[math]::Round($_.WorkingSet64/1MB,1)}}
"#;

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

    info!(task_id, description, "Starting PC task execution");

    // STRATEGY 1: Ask LLM to translate to a PowerShell command
    let _ = app_handle.emit("agent:step_started", serde_json::json!({
        "task_id": task_id, "step_number": 0,
    }));

    let llm_result = gateway
        .complete_with_system(description, Some(COMMAND_TRANSLATOR_PROMPT), settings)
        .await;

    match llm_result {
        Ok(response) => {
            let command = response.content.trim().to_string();
            info!(task_id, command = %command, "LLM translated to command");

            // Check if LLM says it's impossible
            if command.starts_with("IMPOSSIBLE:") {
                // Fall through to vision pipeline
                info!(task_id, "LLM says task needs vision, falling through");
            } else {
                // Execute the PowerShell command
                let exec_result = execute_smart_command(
                    task_id, &command, description, settings, kill_switch,
                    screenshots_dir, db_path, app_handle, &mut step_history,
                ).await;

                if let Some(result) = exec_result {
                    return result;
                }
            }
        }
        Err(e) => {
            warn!(task_id, error = %e, "LLM command translation failed");
        }
    }

    // STRATEGY 2: Vision-guided autonomous loop (for complex UI tasks)
    info!(task_id, "Falling back to vision-guided execution");

    for step_number in (step_history.len() as u32)..MAX_STEPS {
        if kill_switch.load(Ordering::Relaxed) {
            update_task_status(db_path, task_id, "killed");
            return Err("Kill switch activated".to_string());
        }

        // Capture screen
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

        // Ask vision LLM what to do
        let action = vision::plan_next_action(
            &screenshot_b64, description, &step_history, settings, &gateway,
        ).await;

        let action = match action {
            Ok(a) => a,
            Err(e) => {
                warn!(task_id, step_number, error = %e, "Vision LLM failed");
                update_task_status(db_path, task_id, "failed");
                return Err(format!("Vision LLM failed: {}", e));
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

/// Execute a command that the LLM generated, with smart handling
async fn execute_smart_command(
    task_id: &str,
    command: &str,
    description: &str,
    settings: &Settings,
    kill_switch: &Arc<AtomicBool>,
    screenshots_dir: &Path,
    db_path: &Path,
    app_handle: &tauri::AppHandle,
    step_history: &mut Vec<StepRecord>,
) -> Option<Result<TaskExecutionResult, String>> {
    let start = Instant::now();

    // Determine if this is a GUI launch (Start-Process) or a data command
    let is_gui_launch = command.to_lowercase().contains("start-process");
    let needs_output = !is_gui_launch;

    let exec_result = if is_gui_launch {
        // For GUI launches, use launch_app logic (non-blocking)
        hands::cli::run_powershell(command, 30).await
    } else {
        // For data commands, capture output
        hands::cli::run_powershell(command, settings.cli_timeout).await
    };

    let result = match exec_result {
        Ok(output) => {
            info!(task_id, exit = output.exit_code, "Command executed");
            ExecutionResult {
                method: ExecutionMethod::Terminal,
                success: output.exit_code == 0,
                output: Some(if !output.stdout.trim().is_empty() {
                    output.stdout.clone()
                } else if !output.stderr.trim().is_empty() {
                    format!("Error: {}", output.stderr)
                } else {
                    format!("Command executed: {}", command)
                }),
                screenshot_path: None,
                duration_ms: output.duration_ms,
            }
        }
        Err(e) => {
            warn!(task_id, error = %e, "Command failed");
            ExecutionResult {
                method: ExecutionMethod::Terminal,
                success: false,
                output: Some(format!("Failed: {}", e)),
                screenshot_path: None,
                duration_ms: 0,
            }
        }
    };

    // Wait for GUI to appear, then screenshot
    if is_gui_launch {
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
    }

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
        command: command.to_string(),
        shell: ShellType::PowerShell,
    };

    save_step(db_path, task_id, 0, &action, &screenshot_path.unwrap_or_default(), &result);

    let _ = app_handle.emit("agent:step_completed", serde_json::json!({
        "task_id": task_id, "step_number": 0, "success": result.success,
        "output": result.output,
    }));

    let was_success = result.success;
    let output_text = result.output.clone();

    step_history.push(StepRecord {
        step_number: 0,
        action: AgentAction::TaskComplete {
            summary: output_text.clone().unwrap_or_default(),
        },
        result,
        screenshot_path: sp,
    });

    update_task_status(db_path, task_id, if was_success { "completed" } else { "failed" });

    // Emit completion with the actual command output so Chat can show it
    let _ = app_handle.emit("agent:task_completed", serde_json::json!({
        "task_id": task_id,
        "success": was_success,
        "output": output_text,
        "command": command,
        "steps": 1,
        "duration_ms": start.elapsed().as_millis() as u64,
    }));

    Some(Ok(TaskExecutionResult {
        task_id: task_id.to_string(),
        success: was_success,
        steps: step_history.clone(),
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
