use crate::eyes::capture;
use crate::hands;
use crate::pipeline::executor;
use crate::types::*;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use super::recorder::PlaybookFile;

pub struct PlaybookPlayer;

impl PlaybookPlayer {
    /// Replay a playbook by executing each recorded step
    pub async fn play(
        playbook: &PlaybookFile,
        cli_timeout: u64,
        kill_switch: &Arc<AtomicBool>,
        screenshots_dir: &Path,
    ) -> Result<Vec<ExecutionResult>, String> {
        let mut results = Vec::new();

        for step in &playbook.steps {
            // Check kill switch
            if kill_switch.load(std::sync::atomic::Ordering::Relaxed) {
                return Err("Kill switch activated during playbook".to_string());
            }

            // Execute the recorded action
            let result = executor::execute(&step.action, cli_timeout, kill_switch).await?;

            // Capture screenshot after action
            if let Ok(screenshot) = capture::capture_full_screen() {
                let _ = capture::save_screenshot(&screenshot, screenshots_dir);
            }

            results.push(result);

            // Wait for UI to settle
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        Ok(results)
    }

    /// Load a playbook from a JSON file
    pub fn load(path: &Path) -> Result<PlaybookFile, Box<dyn std::error::Error + Send + Sync>> {
        let content = std::fs::read_to_string(path)?;
        let playbook: PlaybookFile = serde_json::from_str(&content)?;
        Ok(playbook)
    }

    /// List all playbooks in a directory
    pub fn list_playbooks(
        dir: &Path,
    ) -> Result<Vec<PlaybookFile>, Box<dyn std::error::Error + Send + Sync>> {
        let mut playbooks = Vec::new();
        if dir.exists() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "json") {
                    if let Ok(pb) = Self::load(&path) {
                        playbooks.push(pb);
                    }
                }
            }
        }
        Ok(playbooks)
    }
}
