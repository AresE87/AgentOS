use crate::tools::trait_def::*;

pub struct WriteFileTool;

#[async_trait::async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str { "write_file" }

    fn description(&self) -> &str {
        "Write content to a file at the given path. Creates the file if it does not exist, overwrites if it does."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file to write" },
                "content": { "type": "string", "description": "Content to write to the file" }
            },
            "required": ["path", "content"]
        })
    }

    fn permission_level(&self) -> PermissionLevel { PermissionLevel::Write }

    async fn execute(&self, input: serde_json::Value, ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
        let path = input.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError("Missing 'path' parameter".into()))?;
        let content = input.get("content").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError("Missing 'content' parameter".into()))?;

        // Workspace boundary enforcement
        let enforcement = crate::tools::enforcer::check_file_write(
            path,
            &ctx.app_data_dir.to_string_lossy(),
        );
        if let crate::tools::enforcer::EnforcementResult::Denied { reason } = enforcement {
            return Ok(ToolOutput { content: format!("BLOCKED: {}", reason), is_error: true });
        }

        // Ensure parent directory exists
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ToolError(format!("Failed to create directory: {}", e)))?;
        }

        std::fs::write(path, content)
            .map_err(|e| ToolError(format!("Failed to write '{}': {}", path, e)))?;

        Ok(ToolOutput {
            content: format!("Wrote {} bytes to {}", content.len(), path),
            is_error: false,
        })
    }
}
