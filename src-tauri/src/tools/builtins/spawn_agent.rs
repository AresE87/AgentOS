use crate::tools::trait_def::*;

pub struct SpawnAgentTool;

impl SpawnAgentTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Tool for SpawnAgentTool {
    fn name(&self) -> &str {
        "spawn_agent"
    }

    fn description(&self) -> &str {
        "Spawn a sub-agent to handle a specific subtask. The sub-agent runs independently \
         with its own tool set and returns the result. Use this for complex tasks that \
         benefit from decomposition."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "agent_name": {
                    "type": "string",
                    "description": "Name/role for the sub-agent (e.g., 'Researcher', 'Code Reviewer')"
                },
                "instructions": {
                    "type": "string",
                    "description": "Detailed instructions for what the sub-agent should do"
                },
                "tools": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "List of tool names the sub-agent can use. If empty, all tools are available."
                },
                "max_iterations": {
                    "type": "integer",
                    "description": "Maximum iterations for the sub-agent (default: 10)"
                }
            },
            "required": ["agent_name", "instructions"]
        })
    }

    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::Execute
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _ctx: &ToolContext,
    ) -> Result<ToolOutput, ToolError> {
        let agent_name = input
            .get("agent_name")
            .and_then(|v| v.as_str())
            .unwrap_or("Sub-Agent");
        let instructions = input
            .get("instructions")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError("Missing 'instructions'".into()))?;
        let max_iter = input
            .get("max_iterations")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as u32;

        // Return a sentinel string that the AgentRuntime detects and replaces
        // with actual sub-agent execution (which needs access to Gateway, etc.).
        Ok(ToolOutput {
            content: format!(
                "__SPAWN_AGENT__:{}:{}:{}",
                agent_name, max_iter, instructions
            ),
            is_error: false,
        })
    }
}
