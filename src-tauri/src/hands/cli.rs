use crate::types::CommandOutput;
use std::time::Instant;
use tokio::process::Command;

/// Execute a PowerShell command
pub async fn run_powershell(
    command: &str,
    timeout_secs: u64,
) -> Result<CommandOutput, Box<dyn std::error::Error + Send + Sync>> {
    run_shell("powershell.exe", &["-NoProfile", "-NonInteractive", "-Command", command], timeout_secs).await
}

/// Execute a CMD command
pub async fn run_cmd(
    command: &str,
    timeout_secs: u64,
) -> Result<CommandOutput, Box<dyn std::error::Error + Send + Sync>> {
    run_shell("cmd.exe", &["/C", command], timeout_secs).await
}

/// Execute an arbitrary process
pub async fn run_process(
    exe: &str,
    args: &[&str],
    timeout_secs: u64,
) -> Result<CommandOutput, Box<dyn std::error::Error + Send + Sync>> {
    run_shell(exe, args, timeout_secs).await
}

async fn run_shell(
    program: &str,
    args: &[&str],
    timeout_secs: u64,
) -> Result<CommandOutput, Box<dyn std::error::Error + Send + Sync>> {
    let start = Instant::now();

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        Command::new(program)
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .creation_flags(0x08000000) // CREATE_NO_WINDOW on Windows
            .output(),
    )
    .await
    .map_err(|_| format!("Command timed out after {}s", timeout_secs))??;

    let duration = start.elapsed().as_millis() as u64;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // Truncate to 1MB
    let stdout = if stdout.len() > 1_048_576 {
        format!("{}...[truncated]", &stdout[..1_048_576])
    } else {
        stdout
    };
    let stderr = if stderr.len() > 1_048_576 {
        format!("{}...[truncated]", &stderr[..1_048_576])
    } else {
        stderr
    };

    Ok(CommandOutput {
        stdout,
        stderr,
        exit_code: output.status.code().unwrap_or(-1),
        duration_ms: duration,
    })
}

/// Trait to add creation flags on Windows
trait CommandExt {
    fn creation_flags(&mut self, flags: u32) -> &mut Self;
}

impl CommandExt for Command {
    fn creation_flags(&mut self, flags: u32) -> &mut Self {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt as WinExt;
            self.as_std_mut().creation_flags(flags);
        }
        self
    }
}
