use serde::{Deserialize, Serialize};
use tokio::process::Command;

/// Apply Windows-specific creation flags to hide console windows.
#[cfg(windows)]
fn hide_window(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x08000000);
}

#[cfg(not(windows))]
fn hide_window(_cmd: &mut Command) {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerContainer {
    pub id: String,
    pub container_id: String,
    pub status: WorkerContainerStatus,
    pub ollama_port: u16,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkerContainerStatus {
    Starting,
    Running,
    Stopped,
    Failed,
}

impl WorkerContainer {
    /// Start a persistent worker container
    pub async fn start(
        worker_id: &str,
        workspace_path: Option<&str>,
        ollama_port: u16,
    ) -> Result<Self, String> {
        let id_prefix_len = 8.min(worker_id.len());
        let container_name = format!("agentos-worker-{}", &worker_id[..id_prefix_len]);

        let mut args = vec![
            "run".to_string(),
            "-d".to_string(),
            "--name".to_string(),
            container_name,
            "--memory=1g".to_string(),
            "--cpus=1.5".to_string(),
            format!("-p{}:11434", ollama_port),
        ];

        // Mount workspace if provided
        if let Some(wp) = workspace_path {
            args.push("-v".to_string());
            args.push(format!("{}:/workspace", wp));
        }

        args.push("agentos-worker:latest".to_string());

        let mut cmd = Command::new("docker");
        cmd.args(&args);
        hide_window(&mut cmd);

        let output = cmd
            .output()
            .await
            .map_err(|e| format!("Failed to start container: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Container start failed: {}", stderr));
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

        Ok(Self {
            id: worker_id.to_string(),
            container_id,
            status: WorkerContainerStatus::Running,
            ollama_port,
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Execute a command inside the container
    pub async fn exec_command(
        container_id: &str,
        command: &str,
    ) -> Result<(String, String, i32), String> {
        let mut cmd = Command::new("docker");
        cmd.args(["exec", container_id, "bash", "-c", command]);
        hide_window(&mut cmd);

        let output = tokio::time::timeout(std::time::Duration::from_secs(60), cmd.output())
            .await
            .map_err(|_| "Command timed out (60s)".to_string())?
            .map_err(|e| format!("Exec failed: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        Ok((stdout, stderr, exit_code))
    }

    /// Stop and remove the container
    pub async fn stop(container_id: &str) -> Result<(), String> {
        let mut cmd = Command::new("docker");
        cmd.args(["rm", "-f", container_id]);
        hide_window(&mut cmd);
        cmd.output().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Get container logs
    pub async fn get_logs(container_id: &str, tail: u32) -> Result<String, String> {
        let mut cmd = Command::new("docker");
        cmd.args(["logs", "--tail", &tail.to_string(), container_id]);
        hide_window(&mut cmd);
        let output = cmd.output().await.map_err(|e| e.to_string())?;
        Ok(String::from_utf8_lossy(&output.stdout).to_string()
            + &String::from_utf8_lossy(&output.stderr))
    }

    /// Check if container is running
    pub async fn is_running(container_id: &str) -> bool {
        let mut cmd = Command::new("docker");
        cmd.args(["inspect", "-f", "{{.State.Running}}", container_id]);
        hide_window(&mut cmd);
        let output = cmd.output().await.ok();
        output
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "true")
            .unwrap_or(false)
    }

    /// List all AgentOS worker containers
    pub async fn list_all() -> Result<Vec<(String, String, String)>, String> {
        let mut cmd = Command::new("docker");
        cmd.args([
            "ps",
            "-a",
            "--filter",
            "name=agentos-worker",
            "--format",
            "{{.ID}}\t{{.Names}}\t{{.Status}}",
        ]);
        hide_window(&mut cmd);
        let output = cmd.output().await.map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&output.stdout);

        Ok(text
            .lines()
            .filter(|l| !l.is_empty())
            .map(|line| {
                let parts: Vec<&str> = line.split('\t').collect();
                (
                    parts.first().unwrap_or(&"").to_string(),
                    parts.get(1).unwrap_or(&"").to_string(),
                    parts.get(2).unwrap_or(&"").to_string(),
                )
            })
            .collect())
    }

    /// Cleanup all stopped AgentOS worker containers
    pub async fn cleanup_all() -> Result<u32, String> {
        let containers = Self::list_all().await?;
        let mut count = 0;
        for (id, _, status) in &containers {
            if status.contains("Exited") {
                Self::stop(id).await.ok();
                count += 1;
            }
        }
        Ok(count)
    }

    /// Find a free port for Ollama (starting from 11435)
    pub fn next_available_port(existing: &[u16]) -> u16 {
        let mut port = 11435u16;
        while existing.contains(&port) {
            port += 1;
        }
        port
    }
}
