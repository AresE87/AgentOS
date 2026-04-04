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

        // S2: sandbox mode edits files via docker exec (read → replace → write)
        match &ctx.execution_mode {
            crate::tools::ExecutionMode::Sandbox { container_id } => {
                let safe_path = path.replace('\'', "'\\''");
                // Read the file from container
                let (content, stderr, exit_code) =
                    crate::sandbox::SandboxManager::exec_command(
                        container_id,
                        &format!("cat '{}'", safe_path),
                    )
                    .await
                    .map_err(|e| ToolError(e))?;

                if exit_code != 0 {
                    return Ok(ToolOutput {
                        content: format!("Error reading file: {}", stderr.trim()),
                        is_error: true,
                    });
                }

                if !content.contains(old_text) {
                    return Ok(ToolOutput {
                        content: format!("old_text not found in {}", path),
                        is_error: true,
                    });
                }

                let new_content = content.replacen(old_text, new_text, 1);
                // Write back via base64 to avoid shell escaping issues
                let encoded = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    new_content.as_bytes(),
                );
                let write_cmd = format!(
                    "echo '{}' | base64 -d > '{}'",
                    encoded, safe_path
                );
                let (_, stderr2, exit_code2) =
                    crate::sandbox::SandboxManager::exec_command(container_id, &write_cmd)
                        .await
                        .map_err(|e| ToolError(e))?;

                if exit_code2 != 0 {
                    return Ok(ToolOutput {
                        content: format!("Error writing file: {}", stderr2.trim()),
                        is_error: true,
                    });
                }

                Ok(ToolOutput {
                    content: format!(
                        "Edited {} (sandbox) — replaced {} bytes with {} bytes",
                        path,
                        old_text.len(),
                        new_text.len()
                    ),
                    is_error: false,
                })
            }
            crate::tools::ExecutionMode::Host => {
                // Original host execution — workspace boundary enforcement
                let enforcement = crate::tools::enforcer::check_file_write(
                    path,
                    &ctx.app_data_dir.to_string_lossy(),
                );
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
    }
}
