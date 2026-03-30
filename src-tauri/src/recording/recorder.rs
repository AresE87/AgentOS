use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub index: u32,
    pub timestamp_ms: u64,
    pub screenshot_path: String,
    pub action: Option<String>,
    pub agent_thought: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recording {
    pub id: String,
    pub task_id: String,
    pub task_description: String,
    pub frames: Vec<Frame>,
    pub duration_ms: u64,
    pub total_actions: u32,
    pub created_at: String,
    pub status: String, // "recording", "completed", "exported"
}

pub struct ScreenRecorder {
    recordings_dir: PathBuf,
    recordings: Vec<Recording>,
}

impl ScreenRecorder {
    pub fn new(recordings_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&recordings_dir).ok();
        Self {
            recordings_dir,
            recordings: vec![],
        }
    }

    pub fn start_recording(&mut self, task_id: &str, description: &str) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let recording_dir = self.recordings_dir.join(&id);
        std::fs::create_dir_all(&recording_dir).ok();

        let recording = Recording {
            id: id.clone(),
            task_id: task_id.to_string(),
            task_description: description.to_string(),
            frames: vec![],
            duration_ms: 0,
            total_actions: 0,
            created_at: chrono::Utc::now().to_rfc3339(),
            status: "recording".to_string(),
        };

        self.recordings.push(recording);
        id
    }

    pub fn add_frame(
        &mut self,
        recording_id: &str,
        screenshot_data: &[u8],
        action: Option<&str>,
        thought: Option<&str>,
    ) -> Result<(), String> {
        let recording = self
            .recordings
            .iter_mut()
            .find(|r| r.id == recording_id)
            .ok_or("Recording not found")?;

        let frame_idx = recording.frames.len() as u32;
        let frame_path = self
            .recordings_dir
            .join(recording_id)
            .join(format!("frame_{:04}.jpg", frame_idx));

        std::fs::write(&frame_path, screenshot_data).map_err(|e| e.to_string())?;

        let start_time = chrono::DateTime::parse_from_rfc3339(&recording.created_at)
            .map(|dt| dt.timestamp_millis() as u64)
            .unwrap_or(0);
        let now = chrono::Utc::now().timestamp_millis() as u64;

        recording.frames.push(Frame {
            index: frame_idx,
            timestamp_ms: now.saturating_sub(start_time),
            screenshot_path: frame_path.to_string_lossy().to_string(),
            action: action.map(|s| s.to_string()),
            agent_thought: thought.map(|s| s.to_string()),
        });

        if action.is_some() {
            recording.total_actions += 1;
        }
        recording.duration_ms = now.saturating_sub(start_time);

        Ok(())
    }

    pub fn stop_recording(&mut self, recording_id: &str) -> Result<Recording, String> {
        let recording = self
            .recordings
            .iter_mut()
            .find(|r| r.id == recording_id)
            .ok_or("Recording not found")?;
        recording.status = "completed".to_string();
        Ok(recording.clone())
    }

    pub fn get_recording(&self, id: &str) -> Option<&Recording> {
        self.recordings.iter().find(|r| r.id == id)
    }

    pub fn list_recordings(&self) -> Vec<RecordingSummary> {
        self.recordings
            .iter()
            .map(|r| RecordingSummary {
                id: r.id.clone(),
                task_description: r.task_description.clone(),
                frame_count: r.frames.len() as u32,
                duration_ms: r.duration_ms,
                total_actions: r.total_actions,
                status: r.status.clone(),
                created_at: r.created_at.clone(),
            })
            .collect()
    }

    pub fn delete_recording(&mut self, id: &str) -> Result<(), String> {
        let dir = self.recordings_dir.join(id);
        if dir.exists() {
            std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
        }
        self.recordings.retain(|r| r.id != id);
        Ok(())
    }

    /// Cleanup recordings older than N days
    pub fn cleanup_old(&mut self, max_age_days: u32) {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(max_age_days as i64);
        let cutoff_str = cutoff.to_rfc3339();

        let to_remove: Vec<String> = self
            .recordings
            .iter()
            .filter(|r| r.created_at < cutoff_str)
            .map(|r| r.id.clone())
            .collect();

        for id in &to_remove {
            self.delete_recording(id).ok();
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordingSummary {
    pub id: String,
    pub task_description: String,
    pub frame_count: u32,
    pub duration_ms: u64,
    pub total_actions: u32,
    pub status: String,
    pub created_at: String,
}
