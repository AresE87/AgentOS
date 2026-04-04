use crate::tools::trait_def::*;

pub struct ReadFileTool;

#[async_trait::async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

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

    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::ReadOnly
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<ToolOutput, ToolError> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError("Missing 'path' parameter".into()))?;

        // S2: sandbox mode reads files via docker exec
        match &ctx.execution_mode {
            crate::tools::ExecutionMode::Sandbox { container_id } => {
                let (stdout, stderr, exit_code) =
                    crate::sandbox::SandboxManager::exec_command(
                        container_id,
                        &format!("cat '{}'", path.replace('\'', "'\\''")),
                    )
                    .await
                    .map_err(|e| ToolError(e))?;

                if exit_code != 0 {
                    return Ok(ToolOutput {
                        content: format!("Error reading file: {}", stderr.trim()),
                        is_error: true,
                    });
                }

                let truncated = if stdout.len() > 50_000 {
                    format!("{}...[truncated]", &stdout[..50_000])
                } else {
                    stdout
                };
                Ok(ToolOutput {
                    content: truncated,
                    is_error: false,
                })
            }
            crate::tools::ExecutionMode::Host => {
                // Original host execution
                let content = std::fs::read_to_string(path)
                    .map_err(|e| ToolError(format!("Failed to read '{}': {}", path, e)))?;

                let truncated = if content.len() > 50_000 {
                    format!("{}...[truncated]", &content[..50_000])
                } else {
                    content
                };

                Ok(ToolOutput {
                    content: truncated,
                    is_error: false,
                })
            }
        }
    }
}
