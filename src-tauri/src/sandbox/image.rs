use tokio::process::Command;

use super::worker_container::WorkerContainer;

const DOCKERFILE_CONTENT: &str = include_str!("../../../installer/worker-image/Dockerfile");

/// Apply Windows-specific creation flags to hide console windows.
#[cfg(windows)]
fn hide_window(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x08000000);
}

#[cfg(not(windows))]
fn hide_window(_cmd: &mut Command) {}

pub struct WorkerImage;

impl WorkerImage {
    /// Check if the worker image exists locally
    pub async fn exists() -> bool {
        let mut cmd = Command::new("docker");
        cmd.args(["image", "inspect", "agentos-worker:latest"]);
        hide_window(&mut cmd);
        let output = cmd.output().await;
        output.map(|o| o.status.success()).unwrap_or(false)
    }

    /// Build the worker image from embedded Dockerfile
    pub async fn build() -> Result<(), String> {
        // Write Dockerfile to temp dir
        let tmp_dir = std::env::temp_dir().join("agentos-worker-build");
        std::fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
        std::fs::write(tmp_dir.join("Dockerfile"), DOCKERFILE_CONTENT)
            .map_err(|e| e.to_string())?;

        let mut cmd = Command::new("docker");
        cmd.args(["build", "-t", "agentos-worker:latest", "."]);
        cmd.current_dir(&tmp_dir);
        hide_window(&mut cmd);

        let output = cmd
            .output()
            .await
            .map_err(|e| format!("Docker build failed: {}", e))?;

        if output.status.success() {
            // Cleanup
            std::fs::remove_dir_all(&tmp_dir).ok();
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Docker build failed: {}", stderr))
        }
    }

    /// Ensure the image exists, building if necessary
    pub async fn ensure() -> Result<(), String> {
        if Self::exists().await {
            return Ok(());
        }
        tracing::info!("Building agentos-worker Docker image...");
        Self::build().await
    }

    /// Pull models into a running container
    pub async fn pull_models(container_id: &str) -> Result<(), String> {
        // Pull phi3:mini (small, fast)
        let _ = WorkerContainer::exec_command(container_id, "ollama pull phi3:mini").await;
        // Pull llama3.2:1b (moderate)
        let _ = WorkerContainer::exec_command(container_id, "ollama pull llama3.2:1b").await;
        Ok(())
    }
}
