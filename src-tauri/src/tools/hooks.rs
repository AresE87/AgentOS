use super::trait_def::{ToolContext, ToolOutput};
use std::collections::HashMap;
use std::sync::Mutex;
use tracing::{info, warn};

// ── Hook result types ──────────────────────────────────────────────

#[derive(Debug)]
pub enum HookResult {
    Continue,
    ModifyInput(serde_json::Value),
    Block(String),
}

// ── Hook traits ────────────────────────────────────────────────────

pub trait PreToolHook: Send + Sync {
    fn before_tool(&self, tool_name: &str, input: &serde_json::Value, ctx: &ToolContext) -> HookResult;
}

pub trait PostToolHook: Send + Sync {
    fn after_tool(&self, tool_name: &str, input: &serde_json::Value, output: &ToolOutput, ctx: &ToolContext);
}

// ── Hook registry ──────────────────────────────────────────────────

pub struct HookRegistry {
    pre_hooks: Vec<Box<dyn PreToolHook>>,
    post_hooks: Vec<Box<dyn PostToolHook>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self {
            pre_hooks: vec![],
            post_hooks: vec![],
        }
    }

    /// Create a registry pre-loaded with the default hooks (audit, safety, cost).
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.add_pre_hook(Box::new(SafetyHook));
        registry.add_post_hook(Box::new(AuditHook));
        registry.add_post_hook(Box::new(CostHook::new()));
        registry
    }

    pub fn add_pre_hook(&mut self, hook: Box<dyn PreToolHook>) {
        self.pre_hooks.push(hook);
    }

    pub fn add_post_hook(&mut self, hook: Box<dyn PostToolHook>) {
        self.post_hooks.push(hook);
    }

    pub fn run_pre_hooks(&self, tool_name: &str, input: &serde_json::Value, ctx: &ToolContext) -> HookResult {
        for hook in &self.pre_hooks {
            match hook.before_tool(tool_name, input, ctx) {
                HookResult::Continue => continue,
                other => return other,
            }
        }
        HookResult::Continue
    }

    pub fn run_post_hooks(&self, tool_name: &str, input: &serde_json::Value, output: &ToolOutput, ctx: &ToolContext) {
        for hook in &self.post_hooks {
            hook.after_tool(tool_name, input, output, ctx);
        }
    }
}

// ── AuditHook — logs every tool invocation ─────────────────────────

pub struct AuditHook;

impl PostToolHook for AuditHook {
    fn after_tool(&self, tool_name: &str, _input: &serde_json::Value, output: &ToolOutput, ctx: &ToolContext) {
        let success = !output.is_error;
        info!(
            tool = tool_name,
            task_id = %ctx.task_id,
            agent = %ctx.agent_name,
            success = success,
            "audit: tool invocation recorded"
        );

        // Best-effort write to audit log DB
        if let Ok(conn) = rusqlite::Connection::open(&ctx.db_path) {
            let _ = crate::enterprise::AuditLog::log(
                &conn,
                "tool_invocation",
                serde_json::json!({
                    "tool_name": tool_name,
                    "task_id": ctx.task_id,
                    "agent_name": ctx.agent_name,
                    "success": success,
                }),
            );
        }
    }
}

// ── SafetyHook — blocks dangerous bash commands ────────────────────

pub struct SafetyHook;

impl PreToolHook for SafetyHook {
    fn before_tool(&self, tool_name: &str, input: &serde_json::Value, _ctx: &ToolContext) -> HookResult {
        if tool_name == "bash" || tool_name == "execute_command" {
            if let Some(cmd) = input.get("command").and_then(|v| v.as_str()) {
                // 6-layer bash validator (supersedes simple sandbox pattern check)
                match crate::security::bash_validator::validate_command(cmd, false) {
                    crate::security::bash_validator::ValidationResult::Block { reason } => {
                        warn!(
                            tool = tool_name,
                            command_preview = &cmd[..cmd.len().min(80)],
                            "safety hook blocked command: {}",
                            reason
                        );
                        return HookResult::Block(reason);
                    },
                    crate::security::bash_validator::ValidationResult::Warn { message } => {
                        warn!(
                            tool = tool_name,
                            "bash validator warning: {}",
                            message
                        );
                    },
                    crate::security::bash_validator::ValidationResult::Allow => {},
                }

                // Also run legacy sandbox patterns for defense-in-depth
                let sandbox = crate::security::sandbox::CommandSandbox::new();
                if let Err(reason) = sandbox.validate_command(cmd) {
                    warn!(
                        tool = tool_name,
                        command_preview = &cmd[..cmd.len().min(80)],
                        "sandbox blocked command: {}",
                        reason
                    );
                    return HookResult::Block(reason);
                }
            }
        }
        HookResult::Continue
    }
}

// ── CostHook — tracks cumulative tool count per task ───────────────

const COST_HOOK_WARN_THRESHOLD: u64 = 100;

pub struct CostHook {
    counters: Mutex<HashMap<String, u64>>,
}

impl CostHook {
    pub fn new() -> Self {
        Self {
            counters: Mutex::new(HashMap::new()),
        }
    }
}

impl PostToolHook for CostHook {
    fn after_tool(&self, tool_name: &str, _input: &serde_json::Value, _output: &ToolOutput, ctx: &ToolContext) {
        if let Ok(mut map) = self.counters.lock() {
            let count = map.entry(ctx.task_id.clone()).or_insert(0);
            *count += 1;

            if *count == COST_HOOK_WARN_THRESHOLD {
                warn!(
                    task_id = %ctx.task_id,
                    tool_count = *count,
                    last_tool = tool_name,
                    "cost hook: task exceeded {} tool invocations",
                    COST_HOOK_WARN_THRESHOLD
                );
            } else if *count > COST_HOOK_WARN_THRESHOLD && *count % 50 == 0 {
                warn!(
                    task_id = %ctx.task_id,
                    tool_count = *count,
                    "cost hook: task tool count still growing"
                );
            }
        }
    }
}
