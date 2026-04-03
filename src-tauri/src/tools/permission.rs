use super::trait_def::{Tool, ToolContext, PermissionLevel};

#[derive(Debug, Clone)]
pub enum PermissionDecision {
    Allowed,
    Denied(String),
    NeedsApproval(String),
}

pub fn check_tool_permission(tool: &dyn Tool, _input: &serde_json::Value, _ctx: &ToolContext) -> PermissionDecision {
    match tool.permission_level() {
        PermissionLevel::ReadOnly => PermissionDecision::Allowed,
        PermissionLevel::Write => PermissionDecision::Allowed,
        PermissionLevel::Execute => PermissionDecision::Allowed,
        PermissionLevel::Dangerous => PermissionDecision::NeedsApproval(
            format!("Tool '{}' requires approval (dangerous operation)", tool.name())
        ),
    }
}
