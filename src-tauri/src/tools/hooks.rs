use super::trait_def::{ToolContext, ToolOutput};

pub enum HookResult {
    Continue,
    ModifyInput(serde_json::Value),
    Block(String),
}

pub struct HookRegistry {
    // Will be populated in Pattern 5
}

impl HookRegistry {
    pub fn new() -> Self {
        Self {}
    }

    pub fn run_pre_hooks(&self, _tool_name: &str, _input: &serde_json::Value, _ctx: &ToolContext) -> HookResult {
        // Placeholder - Pattern 5 will add real hooks
        let _ = (_tool_name, _input, _ctx);
        HookResult::Continue
    }

    pub fn run_post_hooks(&self, _tool_name: &str, _input: &serde_json::Value, _output: &ToolOutput, _ctx: &ToolContext) {
        // Placeholder
        let _ = (_tool_name, _input, _output, _ctx);
    }
}
