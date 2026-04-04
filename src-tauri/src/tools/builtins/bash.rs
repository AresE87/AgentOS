use crate::tools::trait_def::*;

pub struct BashTool;

#[async_trait::async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Execute a shell command (PowerShell on Windows, bash on Linux/macOS). Returns stdout, stderr, and exit code."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "The shell command to execute" }
            },
            "required": ["command"]
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
        let command = input
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError("Missing 'command' parameter".into()))?;

        // 6-layer bash validation
        let validation = crate::security::bash_validator::validate_command(command, false);
        match validation {
            crate::security::bash_validator::ValidationResult::Block { reason } => {
                return Ok(ToolOutput {
                    content: format!("BLOCKED: {}", reason),
                    is_error: true,
                });
            }
            crate::security::bash_validator::ValidationResult::Warn { message } => {
                tracing::warn!("Bash warning: {}", message);
            }
            crate::security::bash_validator::ValidationResult::Allow => {}
        }

        let output = if cfg!(windows) {
            let mut cmd = tokio::process::Command::new("powershell");
            cmd.args(&["-NoProfile", "-NonInteractive", "-Command", command]);
            // Hide the PowerShell window on Windows (CREATE_NO_WINDOW)
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x08000000);
            }
            cmd.output().await
        } else {
            tokio::process::Command::new("bash")
                .args(&["-c", command])
                .output()
                .await
        };

        let output = output.map_err(|e| ToolError(format!("Failed to execute: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let exit_code = output.status.code().unwrap_or(-1);

        let result = if stderr.is_empty() {
            stdout.trim().to_string()
        } else {
            format!(
                "stdout:\n{}\nstderr:\n{}\nexit_code: {}",
                stdout.trim(),
                stderr.trim(),
                exit_code
            )
        };

        // Truncate to 50KB
        let truncated = if result.len() > 50_000 {
            format!("{}...[truncated]", &result[..50_000])
        } else {
            result
        };

        Ok(ToolOutput {
            content: truncated,
            is_error: exit_code != 0,
        })
    }
}
