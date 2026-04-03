use crate::tools::trait_def::*;

pub struct SpawnAgentTool;

#[async_trait::async_trait]
impl Tool for SpawnAgentTool {
    fn name(&self) -> &str { "spawn_agent" }

    fn description(&self) -> &str {
        "Spawn a sub-agent to work on a delegated task. (Placeholder - will be fully implemented in Pattern 3.)"
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "task": { "type": "string", "description": "The task description for the sub-agent" },
                "agent_type": { "type": "string", "description": "Type of agent to spawn (e.g. 'researcher', 'coder')" }
            },
            "required": ["task"]
        })
    }

    fn permission_level(&self) -> PermissionLevel { PermissionLevel::Execute }

    async fn execute(&self, _input: serde_json::Value, _ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
        Ok(ToolOutput {
            content: "Sub-agent spawning not yet implemented. This will be available in Pattern 3.".to_string(),
            is_error: false,
        })
    }
}
