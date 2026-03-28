use crate::types::CommandOutput;
use std::time::Instant;

/// Launch a visible GUI application (notepad, cmd, chrome, etc.)
/// Uses Start-Process to create a fully independent process with its own window.
/// Returns immediately — does NOT wait for the app to close.
pub async fn launch_app(
    program: &str,
) -> Result<CommandOutput, Box<dyn std::error::Error + Send + Sync>> {
    let start = Instant::now();
    let prog = program.to_string();

    let output = tokio::task::spawn_blocking(move || {
        // Sanitize: remove any characters that could break out of the argument
        let safe_prog: String = prog.chars()
            .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_' || *c == '\\' || *c == '/' || *c == ':' || *c == ' ')
            .collect();
        std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command",
                   &format!("Start-Process -FilePath '{}'", safe_prog)])
            .creation_flags(0x08000000) // hide the powershell window itself
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
    })
    .await
    .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.to_string().into() })?
    .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.to_string().into() })?;

    Ok(CommandOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

/// Execute a PowerShell command that produces output (hidden window, captures stdout)
pub async fn run_powershell(
    command: &str,
    timeout_secs: u64,
) -> Result<CommandOutput, Box<dyn std::error::Error + Send + Sync>> {
    run_shell(
        "powershell.exe",
        &["-NoProfile", "-NonInteractive", "-Command", command],
        timeout_secs,
    )
    .await
}

/// Execute a CMD command (hidden window, captures stdout)
pub async fn run_cmd(
    command: &str,
    timeout_secs: u64,
) -> Result<CommandOutput, Box<dyn std::error::Error + Send + Sync>> {
    run_shell("cmd.exe", &["/C", command], timeout_secs).await
}

async fn run_shell(
    program: &str,
    args: &[&str],
    timeout_secs: u64,
) -> Result<CommandOutput, Box<dyn std::error::Error + Send + Sync>> {
    let start = Instant::now();

    let prog = program.to_string();
    let owned_args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let timeout = timeout_secs;

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(timeout),
        tokio::task::spawn_blocking(move || {
            std::process::Command::new(&prog)
                .args(&owned_args)
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output()
        }),
    )
    .await
    .map_err(|_| format!("Command timed out after {}s", timeout_secs))?
    .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.to_string().into() })?
    .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.to_string().into() })?;

    let duration = start.elapsed().as_millis() as u64;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // Truncate to 1MB
    let stdout = truncate(stdout, 1_048_576);
    let stderr = truncate(stderr, 1_048_576);

    Ok(CommandOutput {
        stdout,
        stderr,
        exit_code: output.status.code().unwrap_or(-1),
        duration_ms: duration,
    })
}

fn truncate(s: String, max: usize) -> String {
    if s.len() > max {
        format!("{}...[truncated]", &s[..max])
    } else {
        s
    }
}

#[cfg(windows)]
trait CommandCreationFlags {
    fn creation_flags(&mut self, flags: u32) -> &mut Self;
}

#[cfg(windows)]
impl CommandCreationFlags for std::process::Command {
    fn creation_flags(&mut self, flags: u32) -> &mut Self {
        use std::os::windows::process::CommandExt;
        CommandExt::creation_flags(self, flags);
        self
    }
}
