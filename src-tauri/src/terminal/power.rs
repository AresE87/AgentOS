use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Output from a terminal command execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalOutput {
    /// The command that was executed
    pub command: String,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Process exit code
    pub exit_code: i32,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
}

/// LLM-assisted error explanation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorExplanation {
    /// The original error text
    pub error_text: String,
    /// Human-readable explanation of the error
    pub explanation: String,
    /// Suggested fix or remediation
    pub suggested_fix: String,
    /// Confidence score 0.0 - 1.0
    pub confidence: f64,
}

/// Smart terminal with command execution, error explanation, and NL-to-command translation.
pub struct SmartTerminal {
    /// In-memory history of executed commands
    history: Vec<TerminalOutput>,
    /// Maximum history entries to retain
    max_history: usize,
}

impl SmartTerminal {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            max_history: 500,
        }
    }

    /// Execute a shell command and return structured output.
    pub async fn execute(&mut self, command: &str) -> Result<TerminalOutput, String> {
        let start = Instant::now();

        let mut cmd = tokio::process::Command::new("powershell");
        cmd.args(&["-NoProfile", "-NonInteractive", "-Command", command]);
        // Hide the PowerShell window on Windows (CREATE_NO_WINDOW)
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        let output = cmd.output()
            .await
            .map_err(|e| format!("Failed to execute command: {}", e))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        let result = TerminalOutput {
            command: command.to_string(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            duration_ms,
        };

        // Store in history
        self.history.push(result.clone());
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        Ok(result)
    }

    /// Format a prompt for the LLM to explain an error.
    /// Returns an ErrorExplanation with the prompt as the explanation field
    /// (actual LLM call is handled by the gateway at the IPC layer).
    pub fn explain_error(&self, error_text: &str) -> ErrorExplanation {
        let explanation = format!(
            "Explain this terminal error in plain language and suggest a fix:\n\n```\n{}\n```\n\nProvide:\n1. What happened\n2. Why it happened\n3. How to fix it",
            error_text
        );

        ErrorExplanation {
            error_text: error_text.to_string(),
            explanation,
            suggested_fix: String::new(),
            confidence: 0.0,
        }
    }

    /// Convert natural language to a PowerShell command.
    /// Returns a prompt string that can be sent to the LLM gateway.
    pub fn nl_to_command(&self, natural_language: &str) -> String {
        format!(
            "Convert the following natural language request into a single PowerShell command. \
             Return ONLY the command, no explanation:\n\n\"{}\"\n\nPowerShell command:",
            natural_language
        )
    }

    /// Get recent command history.
    pub fn get_history(&self, limit: usize) -> Vec<TerminalOutput> {
        let start = if self.history.len() > limit {
            self.history.len() - limit
        } else {
            0
        };
        self.history[start..].to_vec()
    }

    /// Clear all history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }
}

impl Default for SmartTerminal {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explain_error() {
        let terminal = SmartTerminal::new();
        let explanation = terminal.explain_error("Permission denied");
        assert!(explanation.explanation.contains("Permission denied"));
        assert_eq!(explanation.error_text, "Permission denied");
    }

    #[test]
    fn test_nl_to_command() {
        let terminal = SmartTerminal::new();
        let prompt = terminal.nl_to_command("list all files in the current directory");
        assert!(prompt.contains("list all files"));
        assert!(prompt.contains("PowerShell command"));
    }

    #[test]
    fn test_history_limit() {
        let terminal = SmartTerminal::new();
        let history = terminal.get_history(10);
        assert!(history.is_empty());
    }
}
