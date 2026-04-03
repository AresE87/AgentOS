use crate::tools::trait_def::*;

pub struct ReadFileTool;

#[async_trait::async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str { "read_file" }

    fn description(&self) -> &str {
        "Read the contents of a file at the given path. Returns the file text, truncated to 50KB."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Absolute or relative path to the file to read" }
            },
            "required": ["path"]
        })
    }

    fn permission_level(&self) -> PermissionLevel { PermissionLevel::ReadOnly }

    async fn execute(&self, input: serde_json::Value, _ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
        let path = input.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError("Missing 'path' parameter".into()))?;

        let content = std::fs::read_to_string(path)
            .map_err(|e| ToolError(format!("Failed to read '{}': {}", path, e)))?;

        let truncated = if content.len() > 50_000 {
            format!("{}...[truncated]", &content[..50_000])
        } else {
            content
        };

        Ok(ToolOutput { content: truncated, is_error: false })
    }
}
