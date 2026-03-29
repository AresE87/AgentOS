use crate::brain::Gateway;
use crate::config::Settings;
use crate::eyes::{capture, vision};
use crate::hands::safety;
use crate::pipeline::executor;
use crate::types::*;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tracing::{info, warn};

use super::recorder::PlaybookFile;

const MAX_ATTEMPTS_PER_STEP: u32 = 5;

/// Scale coordinates from LLM image space to real screen space.
fn scale_action_coords(
    action: AgentAction,
    img_w: u32,
    img_h: u32,
    capture_w: u32,
    capture_h: u32,
) -> AgentAction {
    if capture_w == 0 || capture_h == 0 || img_w == 0 || img_h == 0 {
        return action;
    }

    let scale = |x: i32, y: i32| -> (i32, i32) {
        let real_x = (x as f64 * capture_w as f64 / img_w as f64) as i32;
        let real_y = (y as f64 * capture_h as f64 / img_h as f64) as i32;
        (real_x, real_y)
    };

    match action {
        AgentAction::Click { x, y } => {
            let (rx, ry) = scale(x, y);
            AgentAction::Click { x: rx, y: ry }
        }
        AgentAction::DoubleClick { x, y } => {
            let (rx, ry) = scale(x, y);
            AgentAction::DoubleClick { x: rx, y: ry }
        }
        AgentAction::RightClick { x, y } => {
            let (rx, ry) = scale(x, y);
            AgentAction::RightClick { x: rx, y: ry }
        }
        AgentAction::Scroll { x, y, delta } => {
            let (rx, ry) = scale(x, y);
            AgentAction::Scroll {
                x: rx,
                y: ry,
                delta,
            }
        }
        other => other,
    }
}

pub struct PlaybookPlayer;

impl PlaybookPlayer {
    /// Replay a playbook using vision-guided execution.
    /// For each step, captures the current screen, sends it to the vision LLM
    /// with the step description as guidance, then executes the returned action.
    pub async fn play(
        playbook: &PlaybookFile,
        settings: &Settings,
        kill_switch: &Arc<AtomicBool>,
        app_handle: &tauri::AppHandle,
    ) -> Result<Vec<ExecutionResult>, String> {
        let mut results = Vec::new();
        let gateway = Gateway::new(settings);

        for (step_idx, step) in playbook.steps.iter().enumerate() {
            // Check kill switch
            if kill_switch.load(Ordering::Relaxed) {
                return Err("Kill switch activated during playbook".to_string());
            }

            // Emit step_started event
            let _ = app_handle.emit(
                "playbook:step_started",
                serde_json::json!({
                    "step_number": step.step_number,
                    "description": &step.description,
                    "total_steps": playbook.steps.len(),
                }),
            );

            info!(
                step = step.step_number,
                description = %step.description,
                "Playbook: starting step {}/{}",
                step_idx + 1,
                playbook.steps.len()
            );

            let mut step_result = None;
            let mut step_history: Vec<StepRecord> = Vec::new();
            let mut recent_actions: Vec<String> = Vec::new();

            for attempt in 0..MAX_ATTEMPTS_PER_STEP {
                if kill_switch.load(Ordering::Relaxed) {
                    return Err("Kill switch activated during playbook".to_string());
                }

                // Minimize AgentOS before screen capture
                if let Some(win) = app_handle.get_webview_window("main") {
                    let _ = win.minimize();
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }

                // Capture current screen
                let screenshot = match capture::capture_full_screen() {
                    Ok(s) => s,
                    Err(e) => {
                        warn!("Failed to capture screen: {}", e);
                        continue;
                    }
                };

                let cap_w = screenshot.width;
                let cap_h = screenshot.height;

                let (b64, img_w, img_h) = match capture::to_base64_jpeg_with_dims(&screenshot, 75)
                {
                    Ok(r) => r,
                    Err(e) => {
                        warn!("Failed to encode screenshot: {}", e);
                        continue;
                    }
                };

                // Check for dedup (repeated identical actions)
                let dedup_warning = if recent_actions.len() >= 3 {
                    let last3: Vec<&String> =
                        recent_actions.iter().rev().take(3).collect();
                    last3.windows(2).all(|w| w[0] == w[1])
                } else {
                    false
                };

                // Build task description for the vision LLM
                // Include the step description and playbook context
                let task_desc = format!(
                    "You are replaying step {} of a recorded playbook.\n\
                     PLAYBOOK: {}\n\
                     STEP INSTRUCTION: {}\n\
                     Look at the current screen and perform the action described. \
                     When this step is complete, respond with TaskComplete.",
                    step.step_number + 1,
                    playbook.name,
                    step.description,
                );

                // Ask the vision LLM for the next action
                let action = match vision::plan_next_action(
                    &b64,
                    &task_desc,
                    &step_history,
                    settings,
                    &gateway,
                    Some((img_w, img_h)),
                    dedup_warning,
                )
                .await
                {
                    Ok(a) => a,
                    Err(e) => {
                        warn!(
                            step = step.step_number,
                            attempt, error = %e,
                            "Vision LLM failed for playbook step"
                        );
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        continue;
                    }
                };

                // Track for dedup
                recent_actions.push(format!("{:?}", action));

                // If vision says step is complete, move on
                if let AgentAction::TaskComplete { ref summary } = action {
                    info!(
                        step = step.step_number,
                        summary = %summary,
                        "Playbook step completed"
                    );
                    step_result = Some(ExecutionResult {
                        method: ExecutionMethod::Screen,
                        success: true,
                        output: Some(summary.clone()),
                        screenshot_path: None,
                        duration_ms: 0,
                    });
                    break;
                }

                // Scale coordinates from image space to screen space
                let scaled_action = scale_action_coords(action.clone(), img_w, img_h, cap_w, cap_h);
                info!(
                    step = step.step_number,
                    attempt,
                    action = ?scaled_action,
                    "Playbook vision action (scaled)"
                );

                // Safety check before execution
                let verdict = safety::check_action(&scaled_action);
                if let SafetyVerdict::Blocked { reason } = verdict {
                    warn!(
                        step = step.step_number,
                        reason = %reason,
                        "Action blocked by safety check"
                    );
                    step_history.push(StepRecord {
                        step_number: attempt,
                        action: scaled_action,
                        result: ExecutionResult {
                            method: ExecutionMethod::Screen,
                            success: false,
                            output: Some(format!("Blocked: {}", reason)),
                            screenshot_path: None,
                            duration_ms: 0,
                        },
                        screenshot_path: None,
                    });
                    continue;
                }

                // Execute the action
                let result =
                    match executor::execute(&scaled_action, settings.cli_timeout, kill_switch).await
                    {
                        Ok(r) => r,
                        Err(e) => ExecutionResult {
                            method: ExecutionMethod::Screen,
                            success: false,
                            output: Some(e),
                            screenshot_path: None,
                            duration_ms: 0,
                        },
                    };

                step_history.push(StepRecord {
                    step_number: attempt,
                    action: scaled_action,
                    result: result.clone(),
                    screenshot_path: None,
                });

                if result.success {
                    step_result = Some(result);
                    // Wait for UI to settle before next step
                    tokio::time::sleep(std::time::Duration::from_millis(800)).await;
                    // Don't break — let the loop try again to check if the step is truly done
                    // unless this is the last attempt
                    if attempt >= MAX_ATTEMPTS_PER_STEP - 1 {
                        break;
                    }
                } else {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }

            // Restore AgentOS window
            if let Some(win) = app_handle.get_webview_window("main") {
                if let Err(e) = win.unminimize() {
                    warn!("Failed to restore window: {}", e);
                }
            }

            // Use the result (or a fallback if all attempts failed)
            let final_result = step_result.unwrap_or(ExecutionResult {
                method: ExecutionMethod::Screen,
                success: false,
                output: Some(format!(
                    "Step {} failed after {} attempts",
                    step.step_number, MAX_ATTEMPTS_PER_STEP
                )),
                screenshot_path: None,
                duration_ms: 0,
            });

            // Emit step_completed event
            let _ = app_handle.emit(
                "playbook:step_completed",
                serde_json::json!({
                    "step_number": step.step_number,
                    "success": final_result.success,
                    "output": final_result.output,
                }),
            );

            results.push(final_result);
        }

        Ok(results)
    }

    /// Load a playbook from a JSON file
    pub fn load(path: &Path) -> Result<PlaybookFile, Box<dyn std::error::Error + Send + Sync>> {
        let content = std::fs::read_to_string(path)?;
        let playbook: PlaybookFile = serde_json::from_str(&content)?;
        Ok(playbook)
    }

    /// List all playbooks in a directory
    pub fn list_playbooks(
        dir: &Path,
    ) -> Result<Vec<PlaybookFile>, Box<dyn std::error::Error + Send + Sync>> {
        let mut playbooks = Vec::new();
        if dir.exists() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                // Check top-level .json files (backward compat)
                if path.extension().map_or(false, |e| e == "json") {
                    if let Ok(pb) = Self::load(&path) {
                        playbooks.push(pb);
                    }
                }
                // Also check {name}/playbook.json directories
                if path.is_dir() {
                    let pb_file = path.join("playbook.json");
                    if pb_file.exists() {
                        if let Ok(pb) = Self::load(&pb_file) {
                            // Avoid duplicates if the top-level json was already loaded
                            if !playbooks.iter().any(|existing: &PlaybookFile| existing.name == pb.name) {
                                playbooks.push(pb);
                            }
                        }
                    }
                }
            }
        }
        Ok(playbooks)
    }
}
