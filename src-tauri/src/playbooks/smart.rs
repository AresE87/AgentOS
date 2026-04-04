use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartPlaybook {
    pub id: String,
    pub name: String,
    pub description: String,
    pub variables: Vec<PlaybookVariable>,
    pub steps: Vec<SmartStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybookVariable {
    pub name: String,
    pub var_type: String,
    pub prompt: String,
    pub options: Option<Vec<String>>,
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartStep {
    pub id: String,
    pub step_type: StepType,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StepType {
    #[serde(rename = "command")]
    Command { command: String },

    #[serde(rename = "vision_click")]
    VisionClick { target: String },

    #[serde(rename = "type_text")]
    TypeText { text: String },

    #[serde(rename = "wait")]
    Wait { seconds: u32 },

    #[serde(rename = "condition")]
    Condition {
        check: ConditionCheck,
        if_true: String,
        if_false: String,
    },

    #[serde(rename = "loop")]
    Loop {
        max_iterations: u32,
        steps: Vec<SmartStep>,
        until_condition: String,
    },

    #[serde(rename = "done")]
    Done { result: String },

    #[serde(rename = "browse")]
    Browse { url: String, task: String },

    #[serde(rename = "vision_check")]
    VisionCheck { question: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionCheck {
    ExitCode { step_id: String, expected: i32 },
    Contains { step_id: String, text: String },
    VisionMatch { description: String, threshold: f32 },
}

/// Resolve variables in a string: "Open {filename}" -> "Open report.pdf"
pub fn resolve_variables(text: &str, vars: &HashMap<String, String>) -> String {
    let mut result = text.to_string();
    for (key, value) in vars {
        result = result.replace(&format!("{{{}}}", key), value);
    }
    result
}

/// Smart playbook executor
pub struct SmartPlaybookRunner {
    playbook: SmartPlaybook,
    variables: HashMap<String, String>,
    step_results: HashMap<String, StepResult>,
    options: SmartPlaybookExecutionOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SmartPlaybookExecutionOptions {
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default)]
    pub mocked_step_outputs: HashMap<String, String>,
    #[serde(default)]
    pub mocked_exit_codes: HashMap<String, i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_id: String,
    pub success: bool,
    pub output: String,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
}

impl SmartPlaybookRunner {
    pub fn new(playbook: SmartPlaybook, variables: HashMap<String, String>) -> Self {
        Self::with_options(
            playbook,
            variables,
            SmartPlaybookExecutionOptions::default(),
        )
    }

    pub fn with_options(
        playbook: SmartPlaybook,
        variables: HashMap<String, String>,
        options: SmartPlaybookExecutionOptions,
    ) -> Self {
        Self {
            playbook,
            variables,
            step_results: HashMap::new(),
            options,
        }
    }

    /// Execute the entire playbook
    pub async fn execute(&mut self) -> Result<Vec<StepResult>, String> {
        let steps = self.playbook.steps.clone();
        let mut results = vec![];
        let mut current_idx = 0;

        while current_idx < steps.len() {
            let step = &steps[current_idx];
            let result = self.execute_step(step).await?;

            // Handle conditionals -- may jump to a different step
            let next_idx = match &step.step_type {
                StepType::Condition {
                    check,
                    if_true,
                    if_false,
                    ..
                } => {
                    let passed = self.evaluate_condition(check);
                    let target = if passed { if_true } else { if_false };
                    steps
                        .iter()
                        .position(|s| s.id == *target)
                        .unwrap_or(current_idx + 1)
                }
                StepType::Done { .. } => {
                    self.step_results.insert(step.id.clone(), result.clone());
                    results.push(result);
                    break;
                }
                _ => current_idx + 1,
            };

            self.step_results.insert(step.id.clone(), result.clone());
            results.push(result);
            current_idx = next_idx;
        }

        Ok(results)
    }

    fn execute_step<'a>(
        &'a self,
        step: &'a SmartStep,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<StepResult, String>> + Send + 'a>>
    {
        Box::pin(async move {
            let start = std::time::Instant::now();

            match &step.step_type {
                StepType::Command { command } => {
                    let cmd = resolve_variables(command, &self.variables);
                    if self.options.dry_run {
                        let output = self
                            .options
                            .mocked_step_outputs
                            .get(&step.id)
                            .cloned()
                            .unwrap_or_else(|| format!("[dry-run] {}", cmd));
                        let exit_code = *self.options.mocked_exit_codes.get(&step.id).unwrap_or(&0);
                        return Ok(StepResult {
                            step_id: step.id.clone(),
                            success: exit_code == 0,
                            output,
                            exit_code: Some(exit_code),
                            duration_ms: start.elapsed().as_millis() as u64,
                        });
                    }

                    #[cfg(target_os = "windows")]
                    let output = tokio::process::Command::new("powershell")
                        .args(["-NoProfile", "-Command", &cmd])
                        .output()
                        .await
                        .map_err(|e| e.to_string())?;

                    #[cfg(not(target_os = "windows"))]
                    let output = tokio::process::Command::new("sh")
                        .args(["-c", &cmd])
                        .output()
                        .await
                        .map_err(|e| e.to_string())?;

                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let exit_code = output.status.code().unwrap_or(-1);

                    Ok(StepResult {
                        step_id: step.id.clone(),
                        success: output.status.success(),
                        output: stdout,
                        exit_code: Some(exit_code),
                        duration_ms: start.elapsed().as_millis() as u64,
                    })
                }
                StepType::Wait { seconds } => {
                    if self.options.dry_run {
                        return Ok(StepResult {
                            step_id: step.id.clone(),
                            success: true,
                            output: format!("[dry-run] Wait {} seconds skipped", seconds),
                            exit_code: None,
                            duration_ms: start.elapsed().as_millis() as u64,
                        });
                    }
                    tokio::time::sleep(tokio::time::Duration::from_secs(*seconds as u64)).await;
                    Ok(StepResult {
                        step_id: step.id.clone(),
                        success: true,
                        output: format!("Waited {} seconds", seconds),
                        exit_code: None,
                        duration_ms: start.elapsed().as_millis() as u64,
                    })
                }
                StepType::TypeText { text } => {
                    let resolved = resolve_variables(text, &self.variables);
                    Ok(StepResult {
                        step_id: step.id.clone(),
                        success: true,
                        output: format!("Typed: {}", resolved),
                        exit_code: None,
                        duration_ms: start.elapsed().as_millis() as u64,
                    })
                }
                StepType::Done { result } => Ok(StepResult {
                    step_id: step.id.clone(),
                    success: true,
                    output: resolve_variables(result, &self.variables),
                    exit_code: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
                StepType::Loop {
                    max_iterations,
                    steps: loop_steps,
                    until_condition: _,
                } => {
                    let mut iteration = 0u32;
                    let mut last_output = String::new();

                    while iteration < *max_iterations {
                        iteration += 1;
                        for ls in loop_steps {
                            let r = self.execute_step(ls).await?;
                            last_output = r.output.clone();
                        }
                        // Simple output-based termination check
                        if last_output.to_lowercase().contains("done")
                            || last_output.to_lowercase().contains("complete")
                        {
                            break;
                        }
                    }

                    Ok(StepResult {
                        step_id: step.id.clone(),
                        success: true,
                        output: format!(
                            "Loop completed after {} iterations. Last: {}",
                            iteration, last_output
                        ),
                        exit_code: None,
                        duration_ms: start.elapsed().as_millis() as u64,
                    })
                }
                StepType::VisionClick { target } => {
                    let resolved_target = resolve_variables(target, &self.variables);
                    if self.options.dry_run {
                        return Ok(StepResult {
                            step_id: step.id.clone(),
                            success: true,
                            output: format!("[dry-run] VisionClick on '{}'", resolved_target),
                            exit_code: None,
                            duration_ms: start.elapsed().as_millis() as u64,
                        });
                    }

                    // Capture screenshot and ask LLM where to click
                    let screen = crate::eyes::capture::capture_full_screen()
                        .map_err(|e| format!("Screenshot failed: {}", e))?;
                    let (b64, _w, _h) = crate::eyes::capture::to_base64_jpeg_with_dims(&screen, 80)
                        .map_err(|e| format!("JPEG encode failed: {}", e))?;

                    let prompt = format!(
                        "Find the UI element '{}' in this screenshot. \
                         Return its center coordinates as JSON: {{\"x\": N, \"y\": N}}. \
                         If not found, return {{\"x\": -1, \"y\": -1}}.",
                        resolved_target
                    );

                    let settings = crate::config::Settings::default();
                    let gateway = crate::brain::Gateway::new(&settings);
                    let response = gateway
                        .complete_with_vision(&prompt, &b64, &settings)
                        .await
                        .map_err(|e| format!("Vision LLM failed: {}", e))?;

                    // Parse coordinates from LLM response
                    if let Some(val) = crate::pipeline::engine::extract_json(&response.content) {
                        let x = val["x"].as_i64().unwrap_or(-1) as i32;
                        let y = val["y"].as_i64().unwrap_or(-1) as i32;
                        if x >= 0 && y >= 0 {
                            crate::hands::input::click(x, y)
                                .map_err(|e| format!("Click failed: {}", e))?;
                            Ok(StepResult {
                                step_id: step.id.clone(),
                                success: true,
                                output: format!("Clicked '{}' at ({}, {})", resolved_target, x, y),
                                exit_code: None,
                                duration_ms: start.elapsed().as_millis() as u64,
                            })
                        } else {
                            Ok(StepResult {
                                step_id: step.id.clone(),
                                success: false,
                                output: format!("Could not find '{}' on screen", resolved_target),
                                exit_code: None,
                                duration_ms: start.elapsed().as_millis() as u64,
                            })
                        }
                    } else {
                        Ok(StepResult {
                            step_id: step.id.clone(),
                            success: false,
                            output: format!(
                                "Failed to parse vision response for '{}'",
                                resolved_target
                            ),
                            exit_code: None,
                            duration_ms: start.elapsed().as_millis() as u64,
                        })
                    }
                }
                StepType::Browse { url, task } => {
                    let resolved_url = resolve_variables(url, &self.variables);
                    let resolved_task = resolve_variables(task, &self.variables);
                    if self.options.dry_run {
                        return Ok(StepResult {
                            step_id: step.id.clone(),
                            success: true,
                            output: format!(
                                "[dry-run] Browse '{}' for '{}'",
                                resolved_url, resolved_task
                            ),
                            exit_code: None,
                            duration_ms: start.elapsed().as_millis() as u64,
                        });
                    }

                    // Fetch page content
                    match crate::web::browser::fetch_page(&resolved_url).await {
                        Ok(page) => {
                            let page_text = &page.text[..page.text.len().min(4000)];
                            let settings = crate::config::Settings::default();
                            let gateway = crate::brain::Gateway::new(&settings);
                            let prompt = format!(
                                "Page: {} ({})\nContent:\n{}\n\nTask: {}",
                                page.title, resolved_url, page_text, resolved_task
                            );
                            match gateway.complete(&prompt, &settings).await {
                                Ok(resp) => Ok(StepResult {
                                    step_id: step.id.clone(),
                                    success: true,
                                    output: resp.content,
                                    exit_code: None,
                                    duration_ms: start.elapsed().as_millis() as u64,
                                }),
                                Err(e) => Ok(StepResult {
                                    step_id: step.id.clone(),
                                    success: true,
                                    output: format!(
                                        "Page fetched but LLM analysis failed: {}. Raw: {}",
                                        e, page_text
                                    ),
                                    exit_code: None,
                                    duration_ms: start.elapsed().as_millis() as u64,
                                }),
                            }
                        }
                        Err(e) => Ok(StepResult {
                            step_id: step.id.clone(),
                            success: false,
                            output: format!("Failed to fetch {}: {}", resolved_url, e),
                            exit_code: None,
                            duration_ms: start.elapsed().as_millis() as u64,
                        }),
                    }
                }
                StepType::VisionCheck { question } => {
                    let resolved_q = resolve_variables(question, &self.variables);
                    if self.options.dry_run {
                        return Ok(StepResult {
                            step_id: step.id.clone(),
                            success: true,
                            output: format!("[dry-run] VisionCheck: '{}'", resolved_q),
                            exit_code: None,
                            duration_ms: start.elapsed().as_millis() as u64,
                        });
                    }

                    let screen = crate::eyes::capture::capture_full_screen()
                        .map_err(|e| format!("Screenshot failed: {}", e))?;
                    let (b64, _w, _h) = crate::eyes::capture::to_base64_jpeg_with_dims(&screen, 80)
                        .map_err(|e| format!("JPEG encode failed: {}", e))?;

                    let prompt = format!(
                        "Look at this screenshot and answer: {}. \
                         Respond ONLY with JSON: {{\"result\": true, \"explanation\": \"...\"}} or {{\"result\": false, \"explanation\": \"...\"}}",
                        resolved_q
                    );

                    let settings = crate::config::Settings::default();
                    let gateway = crate::brain::Gateway::new(&settings);
                    let response = gateway
                        .complete_with_vision(&prompt, &b64, &settings)
                        .await
                        .map_err(|e| format!("Vision check failed: {}", e))?;

                    let passed = response.content.contains("\"result\": true")
                        || response.content.contains("\"result\":true");
                    Ok(StepResult {
                        step_id: step.id.clone(),
                        success: passed,
                        output: response.content,
                        exit_code: Some(if passed { 0 } else { 1 }),
                        duration_ms: start.elapsed().as_millis() as u64,
                    })
                }
                StepType::Condition { .. } => {
                    // Conditions are handled in execute() for flow control;
                    // if reached here directly, just mark as executed
                    Ok(StepResult {
                        step_id: step.id.clone(),
                        success: true,
                        output: "Condition evaluated".to_string(),
                        exit_code: None,
                        duration_ms: start.elapsed().as_millis() as u64,
                    })
                }
            }
        }) // end Box::pin
    }

    fn evaluate_condition(&self, check: &ConditionCheck) -> bool {
        match check {
            ConditionCheck::ExitCode { step_id, expected } => self
                .step_results
                .get(step_id)
                .and_then(|r| r.exit_code)
                .map(|code| code == *expected)
                .unwrap_or(false),
            ConditionCheck::Contains { step_id, text } => self
                .step_results
                .get(step_id)
                .map(|r| r.output.contains(text))
                .unwrap_or(false),
            ConditionCheck::VisionMatch {
                description,
                threshold: _,
            } => {
                // Synchronous fallback: always true (async vision check not available in sync context)
                // Real vision checks should use the VisionCheck step type instead
                tracing::warn!("VisionMatch condition evaluated as true (use VisionCheck step for async vision)");
                let _ = description; // suppress unused warning
                true
            }
        }
    }
}

/// Validate a smart playbook structure
pub fn validate_playbook(playbook: &SmartPlaybook) -> Result<Vec<String>, Vec<String>> {
    let mut warnings = vec![];
    let mut errors = vec![];

    if playbook.id.is_empty() {
        errors.push("Playbook id is empty".to_string());
    }
    if playbook.name.is_empty() {
        errors.push("Playbook name is empty".to_string());
    }
    if playbook.steps.is_empty() {
        errors.push("Playbook has no steps".to_string());
    }

    // Collect all step IDs
    let step_ids: Vec<&str> = playbook.steps.iter().map(|s| s.id.as_str()).collect();

    // Check for duplicate step IDs
    let mut seen = std::collections::HashSet::new();
    for id in &step_ids {
        if !seen.insert(id) {
            errors.push(format!("Duplicate step id: {}", id));
        }
    }

    // Check condition targets reference valid step IDs
    for step in &playbook.steps {
        if let StepType::Condition {
            if_true, if_false, ..
        } = &step.step_type
        {
            if !step_ids.contains(&if_true.as_str()) {
                errors.push(format!(
                    "Condition step '{}' references unknown if_true target '{}'",
                    step.id, if_true
                ));
            }
            if !step_ids.contains(&if_false.as_str()) {
                errors.push(format!(
                    "Condition step '{}' references unknown if_false target '{}'",
                    step.id, if_false
                ));
            }
        }
    }

    // Check that referenced variables in commands actually exist
    let var_names: Vec<&str> = playbook.variables.iter().map(|v| v.name.as_str()).collect();
    for step in &playbook.steps {
        let texts_to_check: Vec<&str> = match &step.step_type {
            StepType::Command { command } => vec![command.as_str()],
            StepType::TypeText { text } => vec![text.as_str()],
            StepType::Browse { url, task } => vec![url.as_str(), task.as_str()],
            _ => vec![],
        };
        for text in texts_to_check {
            // Find {varname} patterns
            let mut chars = text.chars().peekable();
            while let Some(c) = chars.next() {
                if c == '{' {
                    let var: String = chars.by_ref().take_while(|&ch| ch != '}').collect();
                    if !var.is_empty() && !var_names.contains(&var.as_str()) {
                        warnings.push(format!(
                            "Step '{}' references variable '{{{}}}' not declared in playbook variables",
                            step.id, var
                        ));
                    }
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(warnings)
    } else {
        Err(errors)
    }
}
