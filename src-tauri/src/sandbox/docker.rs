use serde::{Deserialize, Serialize};
use std::time::Instant;
use tokio::process::Command;

// ── Data types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub image: String,
    pub memory_limit_mb: u32,
    pub cpu_limit: f64,
    pub timeout_secs: u32,
    pub network_enabled: bool,
    pub working_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub sandbox_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningContainer {
    pub id: String,
    pub image: String,
    pub status: String,
    pub name: String,
}

// ── SandboxManager ──────────────────────────────────────────────────────

pub struct SandboxManager;

impl SandboxManager {
    /// Check if Docker CLI is available and the daemon is running.
    pub async fn is_docker_available() -> bool {
        match Command::new("docker").arg("version").output().await {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    /// Run a command inside a new Docker container with resource limits.
    /// The container is automatically removed after execution (`--rm`).
    pub async fn create_sandbox(
        config: &SandboxConfig,
        command: &str,
    ) -> Result<SandboxResult, String> {
        let sandbox_id = format!("agentos-sandbox-{}", uuid::Uuid::new_v4());

        let mut args: Vec<String> = vec![
            "run".into(),
            "--rm".into(),
            "--name".into(),
            sandbox_id.clone(),
            // Memory limit
            format!("--memory={}m", config.memory_limit_mb),
            // CPU limit
            format!("--cpus={}", config.cpu_limit),
        ];

        // Network isolation
        if !config.network_enabled {
            args.push("--network=none".into());
        }

        // Working directory inside container
        if let Some(ref wd) = config.working_dir {
            args.push("-w".into());
            args.push(wd.clone());
        }

        // Image
        args.push(config.image.clone());

        // Command — run via shell so pipes / && etc. work
        args.push("sh".into());
        args.push("-c".into());
        args.push(command.to_string());

        let start = Instant::now();

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(config.timeout_secs as u64),
            Command::new("docker").args(&args).output(),
        )
        .await
        .map_err(|_| format!("Sandbox timed out after {}s", config.timeout_secs))?
        .map_err(|e| format!("Failed to start docker: {}", e))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(SandboxResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            duration_ms,
            sandbox_id,
        })
    }

    /// List currently running AgentOS sandbox containers.
    pub async fn list_running() -> Result<Vec<RunningContainer>, String> {
        let output = Command::new("docker")
            .args([
                "ps",
                "--filter",
                "name=agentos-sandbox-",
                "--format",
                "{{.ID}}\t{{.Image}}\t{{.Status}}\t{{.Names}}",
            ])
            .output()
            .await
            .map_err(|e| format!("Failed to run docker ps: {}", e))?;

        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let containers: Vec<RunningContainer> = stdout
            .lines()
            .filter(|line| !line.is_empty())
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(4, '\t').collect();
                if parts.len() >= 4 {
                    Some(RunningContainer {
                        id: parts[0].to_string(),
                        image: parts[1].to_string(),
                        status: parts[2].to_string(),
                        name: parts[3].to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(containers)
    }

    /// S2: Execute a command inside an already-running container.
    /// Returns (stdout, stderr, exit_code).
    pub async fn exec_command(
        container_id: &str,
        command: &str,
    ) -> Result<(String, String, i32), String> {
        let output = Command::new("docker")
            .args(["exec", container_id, "sh", "-c", command])
            .output()
            .await
            .map_err(|e| format!("Docker exec failed: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        Ok((stdout, stderr, exit_code))
    }

    /// Kill (stop + remove) a running sandbox container by ID or name.
    pub async fn kill_sandbox(id: &str) -> Result<(), String> {
        let output = Command::new("docker")
            .args(["rm", "-f", id])
            .output()
            .await
            .map_err(|e| format!("Failed to kill container: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }
}
