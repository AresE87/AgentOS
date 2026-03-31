use crate::hands;
use crate::security;
use crate::types::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Execute an action using priority chain: API → Terminal → Screen
pub async fn execute(
    action: &AgentAction,
    cli_timeout: u64,
    kill_switch: &Arc<AtomicBool>,
) -> Result<ExecutionResult, String> {
    if kill_switch.load(Ordering::Relaxed) {
        return Err("Kill switch activated".to_string());
    }

    // Check safety first
    let verdict = hands::safety::check_action(action);
    match verdict {
        SafetyVerdict::Blocked { reason } => {
            return Err(format!("Action blocked: {}", reason));
        }
        SafetyVerdict::RequiresConfirmation { reason } => {
            // For now, log and proceed. Phase 3 will add user confirmation flow.
            tracing::warn!("Action requires confirmation: {}", reason);
        }
        SafetyVerdict::Allowed => {}
    }

    let start = Instant::now();

    match action {
        AgentAction::RunCommand { command, shell } => {
            // R36: Sandbox validation before execution
            let sandbox = security::sandbox::CommandSandbox::new();
            sandbox
                .validate_command(command)
                .map_err(|e| format!("Sandbox blocked: {}", e))?;

            // Try Terminal execution
            let result = match shell {
                ShellType::PowerShell => hands::cli::run_powershell(command, cli_timeout).await,
                ShellType::Cmd => hands::cli::run_cmd(command, cli_timeout).await,
            };

            match result {
                Ok(output) => Ok(ExecutionResult {
                    method: ExecutionMethod::Terminal,
                    success: output.exit_code == 0,
                    output: Some(if output.stdout.is_empty() {
                        output.stderr
                    } else {
                        output.stdout
                    }),
                    screenshot_path: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
                Err(e) => Err(format!("Command execution failed: {}", e)),
            }
        }

        AgentAction::Click { x, y } => execute_screen_action(|| hands::input::click(*x, *y), start),
        AgentAction::DoubleClick { x, y } => {
            execute_screen_action(|| hands::input::double_click(*x, *y), start)
        }
        AgentAction::RightClick { x, y } => {
            execute_screen_action(|| hands::input::right_click(*x, *y), start)
        }
        AgentAction::Type { text } => {
            execute_screen_action(|| hands::input::type_text(text), start)
        }
        AgentAction::KeyCombo { keys } => {
            execute_screen_action(|| hands::input::key_combo(keys), start)
        }
        AgentAction::Scroll { x, y, delta } => {
            execute_screen_action(|| hands::input::scroll(*x, *y, *delta), start)
        }
        AgentAction::Wait { ms } => {
            tokio::time::sleep(std::time::Duration::from_millis(*ms)).await;
            Ok(ExecutionResult {
                method: ExecutionMethod::Api,
                success: true,
                output: Some(format!("Waited {}ms", ms)),
                screenshot_path: None,
                duration_ms: start.elapsed().as_millis() as u64,
            })
        }
        AgentAction::Screenshot => {
            // Just signals to capture — handled by the engine
            Ok(ExecutionResult {
                method: ExecutionMethod::Api,
                success: true,
                output: Some("Screenshot requested".to_string()),
                screenshot_path: None,
                duration_ms: 0,
            })
        }
        AgentAction::TaskComplete { summary } => Ok(ExecutionResult {
            method: ExecutionMethod::Api,
            success: true,
            output: Some(summary.clone()),
            screenshot_path: None,
            duration_ms: 0,
        }),
    }
}

fn execute_screen_action<F>(f: F, start: Instant) -> Result<ExecutionResult, String>
where
    F: FnOnce() -> Result<(), Box<dyn std::error::Error + Send + Sync>>,
{
    match f() {
        Ok(()) => Ok(ExecutionResult {
            method: ExecutionMethod::Screen,
            success: true,
            output: None,
            screenshot_path: None,
            duration_ms: start.elapsed().as_millis() as u64,
        }),
        Err(e) => Err(format!("Screen action failed: {}", e)),
    }
}
