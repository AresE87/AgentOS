use crate::tools::trait_def::*;

pub struct EditFileTool;

#[async_trait::async_trait]
impl Tool for EditFileTool {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn description(&self) -> &str {
        "Edit a file by replacing a specific text span with new text. Reads the file, performs the replacement, and writes it back."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file to edit" },
                "old_text": { "type": "string", "description": "The exact text to find and replace" },
                "new_text": { "type": "string", "description": "The replacement text" }
            },
            "required": ["path", "old_text", "new_text"]
        })
    }

    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::Write
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
        let old_text = input
            .get("old_text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError("Missing 'old_text' parameter".into()))?;
        let new_text = input
            .get("new_text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError("Missing 'new_text' parameter".into()))?;

        // Workspace boundary enforcement
        let enforcement =
            crate::tools::enforcer::check_file_write(path, &ctx.app_data_dir.to_string_lossy());
        if let crate::tools::enforcer::EnforcementResult::Denied { reason } = enforcement {
            return Ok(ToolOutput {
                content: format!("BLOCKED: {}", reason),
                is_error: true,
            });
        }

        let content = std::fs::read_to_string(path)
            .map_err(|e| ToolError(format!("Failed to read '{}': {}", path, e)))?;

        if !content.contains(old_text) {
            return Ok(ToolOutput {
                content: format!("old_text not found in {}", path),
                is_error: true,
            });
        }

        let new_content = content.replacen(old_text, new_text, 1);
        std::fs::write(path, &new_content)
            .map_err(|e| ToolError(format!("Failed to write '{}': {}", path, e)))?;

        Ok(ToolOutput {
            content: format!(
                "Edited {} — replaced {} bytes with {} bytes",
                path,
                old_text.len(),
                new_text.len()
            ),
            is_error: false,
        })
    }
}
