use super::runtime::AgentRuntime;
use super::types::{AgentLoopConfig, AgentTurnResult};
use crate::brain::Gateway;
use crate::config::Settings;
use crate::tools::{ToolContext, ToolRegistry};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::Emitter;
use tracing::{info, warn};

/// Maximum nesting depth for sub-agents to prevent runaway recursion.
const MAX_DEPTH: u32 = 3;

pub struct SubAgentManager;

impl SubAgentManager {
    /// Execute a sub-agent with its own tool loop.
    ///
    /// The sub-agent gets a fresh conversation starting from `instructions`,
    /// its own AgentRuntime with a lower iteration budget, and shares the
    /// parent's kill switch so that cancellation propagates.
    pub async fn execute_sub_agent(
        agent_name: &str,
        instructions: &str,
        max_iterations: u32,
        tool_registry: &ToolRegistry,
        gateway: &Gateway,
        settings: &Settings,
        parent_ctx: &ToolContext,
        depth: u32,
        event_emitter: Option<&tauri::AppHandle>,
    ) -> Result<AgentTurnResult, String> {
        if depth >= MAX_DEPTH {
            return Err(format!(
                "Max sub-agent depth ({}) reached — refusing to spawn '{}'",
                MAX_DEPTH, agent_name
            ));
        }

        info!(
            agent_name = agent_name,
            depth = depth,
            max_iterations = max_iterations,
            "spawning sub-agent"
        );

        let config = AgentLoopConfig {
            max_iterations,
            max_tokens_per_turn: 4096,
            compact_threshold_tokens: 80_000,
        };

        let runtime = AgentRuntime::new(config);

        let sub_ctx = ToolContext {
            agent_name: agent_name.to_string(),
            task_id: format!("{}_sub_{}", parent_ctx.task_id, uuid::Uuid::new_v4()),
            db_path: parent_ctx.db_path.clone(),
            app_data_dir: parent_ctx.app_data_dir.clone(),
            kill_switch: parent_ctx.kill_switch.clone(),
        };

        let system_prompt = format!(
            "You are {}, a specialized sub-agent of AgentOS. Complete this task:\n\n{}\n\n\
             Use tools as needed. Be thorough but concise.",
            agent_name, instructions
        );

        // Build tool definitions for the sub-agent (all tools available)
        let tool_defs: Vec<serde_json::Value> = tool_registry
            .definitions()
            .iter()
            .map(|d| {
                serde_json::json!({
                    "name": d.name,
                    "description": d.description,
                    "input_schema": d.input_schema,
                })
            })
            .collect();

        // Emit sub-agent started event
        if let Some(handle) = event_emitter {
            let _ = handle.emit(
                "agent:subagent_started",
                serde_json::json!({
                    "parent_task_id": parent_ctx.task_id,
                    "sub_task_id": sub_ctx.task_id,
                    "agent_name": agent_name,
                    "depth": depth,
                }),
            );
        }

        // Box::pin the recursive future to avoid infinitely sized types
        // Sub-agents don't persist sessions separately
        let result = Box::pin(runtime.run_turn(
            instructions,
            &system_prompt,
            &tool_defs,
            tool_registry,
            &sub_ctx,
            gateway,
            settings,
            &parent_ctx.kill_switch,
            event_emitter,
            None,
            None,
            None,
        ))
        .await;

        // Emit sub-agent completed event
        if let Some(handle) = event_emitter {
            let _ = handle.emit(
                "agent:subagent_completed",
                serde_json::json!({
                    "parent_task_id": parent_ctx.task_id,
                    "sub_task_id": sub_ctx.task_id,
                    "agent_name": agent_name,
                    "success": result.is_ok(),
                }),
            );
        }

        result
    }
}
