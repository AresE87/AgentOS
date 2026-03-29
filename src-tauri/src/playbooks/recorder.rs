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
    name: String,
    steps: Vec<RecordedStep>,
    playbooks_dir: PathBuf,
    recording: bool,
}

impl PlaybookRecorder {
    pub fn new(playbooks_dir: &Path) -> Self {
        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            name: String::new(),
            steps: Vec::new(),
            playbooks_dir: playbooks_dir.to_path_buf(),
            recording: false,
        }
    }

    pub fn start(&mut self, name: &str) -> String {
        self.recording = true;
        self.steps.clear();
        self.session_id = uuid::Uuid::new_v4().to_string();
        self.name = name.to_string();

        // Create the steps directory: playbooks/{name}/steps/
        let steps_dir = self.steps_dir();
        let _ = std::fs::create_dir_all(&steps_dir);

        self.session_id.clone()
    }

    pub fn is_recording(&self) -> bool {
        self.recording
    }

    /// Directory for this playbook
    fn playbook_dir(&self) -> PathBuf {
        let safe_name = self
            .name
            .to_lowercase()
            .replace(' ', "_")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect::<String>();
        self.playbooks_dir.join(&safe_name)
    }

    /// Directory for step screenshots and metadata
    fn steps_dir(&self) -> PathBuf {
        self.playbook_dir().join("steps")
    }

    pub fn record_step(
        &mut self,
        action: AgentAction,
        description: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.recording {
            return Err("Not recording".into());
        }

        let step_num = self.steps.len() as u32;
        let steps_dir = self.steps_dir();
        std::fs::create_dir_all(&steps_dir)?;

        // 1. Capture full screen screenshot
        let screenshot = capture::capture_full_screen()?;

        // 2. Save as steps/{step_number:02}.jpg
        let screenshot_filename = format!("{:02}.jpg", step_num);
        let screenshot_path = steps_dir.join(&screenshot_filename);
        capture::save_screenshot_to(&screenshot, &screenshot_path)?;

        // 3. Save metadata as steps/{step_number:02}.json
        let metadata = serde_json::json!({
            "description": description,
            "action_type": format!("{:?}", action),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "step_number": step_num,
        });
        let meta_filename = format!("{:02}.json", step_num);
        let meta_path = steps_dir.join(&meta_filename);
        std::fs::write(&meta_path, serde_json::to_string_pretty(&metadata)?)?;

        // 4. Track step in memory
        self.steps.push(RecordedStep {
            step_number: step_num,
            action,
            screenshot_path: screenshot_path.to_string_lossy().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            description: description.to_string(),
        });

        Ok(())
    }

    pub fn stop(&mut self) -> PlaybookFile {
        self.recording = false;
        PlaybookFile {
            name: self.name.clone(),
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
        // Save playbook.json inside the playbook's own directory
        let safe_name = playbook
            .name
            .to_lowercase()
            .replace(' ', "_")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect::<String>();
        let playbook_dir = dir.join(&safe_name);
        std::fs::create_dir_all(&playbook_dir)?;

        let path = playbook_dir.join("playbook.json");
        let json = serde_json::to_string_pretty(playbook)?;
        std::fs::write(&path, json)?;

        // Also save a top-level JSON for backward compat with list_playbooks
        let compat_path = dir.join(format!("{}.json", safe_name));
        std::fs::write(&compat_path, serde_json::to_string_pretty(playbook)?)?;

        Ok(path)
    }
}
