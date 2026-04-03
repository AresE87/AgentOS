use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Emitter;

use super::types::*;
use crate::tools::{ToolRegistry, ToolContext, check_tool_permission, PermissionDecision};
use crate::brain::Gateway;
use crate::config::Settings;

pub struct AgentRuntime {
    config: AgentLoopConfig,
}

impl AgentRuntime {
    pub fn new(config: AgentLoopConfig) -> Self {
        Self { config }
    }

    /// The core agent loop: calls LLM, executes tool_use blocks, feeds results back,
    /// and repeats until the model stops requesting tools or we hit max iterations.
    pub async fn run_turn(
        &self,
        user_message: &str,
        system_prompt: &str,
        tools_json: &[serde_json::Value],
        tool_registry: &ToolRegistry,
        tool_ctx: &ToolContext,
        gateway: &Gateway,
        settings: &Settings,
        kill_switch: &Arc<AtomicBool>,
        event_emitter: Option<&tauri::AppHandle>,
    ) -> Result<AgentTurnResult, String> {
        let mut messages: Vec<serde_json::Value> = vec![serde_json::json!({
            "role": "user",
            "content": user_message
        })];

        let mut tool_records: Vec<ToolCallRecord> = vec![];
        let mut total_input = 0u32;
        let mut total_output = 0u32;
        let mut final_text = String::new();

        for iteration in 0..self.config.max_iterations {
            // Check kill switch
            if kill_switch.load(Ordering::Relaxed) {
                return Err("Task cancelled".into());
            }

            // Call LLM with tools
            let response = gateway
                .complete_with_tools(&messages, tools_json, system_prompt, settings)
                .await?;

            // Extract usage
            if let Some(usage) = response.get("usage") {
                total_input +=
                    usage.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                total_output +=
                    usage.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            }

            let stop_reason = response
                .get("stop_reason")
                .and_then(|v| v.as_str())
                .unwrap_or("end_turn");
            let content = response
                .get("content")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            // Collect text and tool_use blocks
            let mut text_parts = vec![];
            let mut tool_uses = vec![];

            for block in &content {
                match block.get("type").and_then(|v| v.as_str()) {
                    Some("text") => {
                        if let Some(t) = block.get("text").and_then(|v| v.as_str()) {
                            text_parts.push(t.to_string());
                        }
                    }
                    Some("tool_use") => {
                        tool_uses.push(block.clone());
                    }
                    _ => {}
                }
            }

            // Add assistant message to conversation
            messages.push(serde_json::json!({
                "role": "assistant",
                "content": content
            }));

            // If no tool calls, we're done
            if tool_uses.is_empty() || stop_reason == "end_turn" {
                final_text = text_parts.join("\n");
                return Ok(AgentTurnResult {
                    text: final_text,
                    tool_calls_made: tool_records,
                    iterations: iteration + 1,
                    total_input_tokens: total_input,
                    total_output_tokens: total_output,
                    stop_reason: stop_reason.to_string(),
                });
            }

            // Execute tool calls
            let mut tool_results = vec![];

            for tool_use in &tool_uses {
                let tool_id = tool_use.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let tool_name = tool_use.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let tool_input = tool_use
                    .get("input")
                    .cloned()
                    .unwrap_or(serde_json::json!({}));

                // Emit tool_start event
                if let Some(handle) = event_emitter {
                    let _ = handle.emit(
                        "agent:tool_start",
                        serde_json::json!({
                            "tool_name": tool_name,
                            "iteration": iteration,
                        }),
                    );
                }

                let start = std::time::Instant::now();

                // Execute via registry
                let result = if let Some(tool) = tool_registry.get(tool_name) {
                    let perm = check_tool_permission(tool, &tool_input, tool_ctx);
                    match perm {
                        PermissionDecision::Allowed => {
                            match tool.execute(tool_input.clone(), tool_ctx).await {
                                Ok(output) => output,
                                Err(e) => crate::tools::ToolOutput {
                                    content: format!("Error: {}", e),
                                    is_error: true,
                                },
                            }
                        }
                        PermissionDecision::Denied(reason) => crate::tools::ToolOutput {
                            content: format!("Permission denied: {}", reason),
                            is_error: true,
                        },
                        PermissionDecision::NeedsApproval(_reason) => {
                            // For now, auto-approve. Pattern 5 will add real approval flow.
                            match tool.execute(tool_input.clone(), tool_ctx).await {
                                Ok(output) => output,
                                Err(e) => crate::tools::ToolOutput {
                                    content: format!("Error: {}", e),
                                    is_error: true,
                                },
                            }
                        }
                    }
                } else {
                    crate::tools::ToolOutput {
                        content: format!("Unknown tool: {}", tool_name),
                        is_error: true,
                    }
                };

                let duration = start.elapsed().as_millis() as u64;

                tool_records.push(ToolCallRecord {
                    tool_name: tool_name.to_string(),
                    input_preview: tool_input.to_string().chars().take(200).collect(),
                    output_preview: result.content.chars().take(200).collect(),
                    success: !result.is_error,
                    duration_ms: duration,
                });

                // Emit tool_result event
                if let Some(handle) = event_emitter {
                    let _ = handle.emit(
                        "agent:tool_result",
                        serde_json::json!({
                            "tool_name": tool_name,
                            "success": !result.is_error,
                            "iteration": iteration,
                        }),
                    );
                }

                tool_results.push(serde_json::json!({
                    "type": "tool_result",
                    "tool_use_id": tool_id,
                    "content": result.content,
                    "is_error": result.is_error,
                }));
            }

            // Add tool results as user message
            messages.push(serde_json::json!({
                "role": "user",
                "content": tool_results
            }));

            // Emit iteration event
            if let Some(handle) = event_emitter {
                let _ = handle.emit(
                    "agent:iteration",
                    serde_json::json!({
                        "iteration": iteration + 1,
                        "total_tokens": total_input + total_output,
                    }),
                );
            }
        }

        Ok(AgentTurnResult {
            text: final_text,
            tool_calls_made: tool_records,
            iterations: self.config.max_iterations,
            total_input_tokens: total_input,
            total_output_tokens: total_output,
            stop_reason: "max_iterations".to_string(),
        })
    }
}
