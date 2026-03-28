use crate::eyes::capture;
use crate::types::AgentAction;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedStep {
    pub step_number: u32,
    pub action: AgentAction,
    pub screenshot_path: String,
    pub timestamp: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybookFile {
    pub name: String,
    pub description: String,
    pub version: u32,
    pub author: String,
    pub steps: Vec<RecordedStep>,
    pub created_at: String,
}

pub struct PlaybookRecorder {
    session_id: String,
    steps: Vec<RecordedStep>,
    screenshots_dir: PathBuf,
    recording: bool,
}

impl PlaybookRecorder {
    pub fn new(screenshots_dir: &Path) -> Self {
        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            steps: Vec::new(),
            screenshots_dir: screenshots_dir.to_path_buf(),
            recording: false,
        }
    }

    pub fn start(&mut self) -> String {
        self.recording = true;
        self.steps.clear();
        self.session_id = uuid::Uuid::new_v4().to_string();
        self.session_id.clone()
    }

    pub fn is_recording(&self) -> bool {
        self.recording
    }

    pub fn record_step(
        &mut self,
        action: AgentAction,
        description: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.recording {
            return Err("Not recording".into());
        }

        let screenshot = capture::capture_full_screen()?;
        let path = capture::save_screenshot(&screenshot, &self.screenshots_dir)?;

        self.steps.push(RecordedStep {
            step_number: self.steps.len() as u32,
            action,
            screenshot_path: path.to_string_lossy().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            description: description.to_string(),
        });

        Ok(())
    }

    pub fn stop(&mut self) -> PlaybookFile {
        self.recording = false;
        PlaybookFile {
            name: format!("Recording {}", &self.session_id[..8]),
            description: String::new(),
            version: 1,
            author: String::new(),
            steps: self.steps.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn save_playbook(
        playbook: &PlaybookFile,
        dir: &Path,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        std::fs::create_dir_all(dir)?;
        let filename = format!(
            "{}.json",
            playbook
                .name
                .to_lowercase()
                .replace(' ', "_")
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_')
                .collect::<String>()
        );
        let path = dir.join(filename);
        let json = serde_json::to_string_pretty(playbook)?;
        std::fs::write(&path, json)?;
        Ok(path)
    }
}
