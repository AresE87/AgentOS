use std::path::PathBuf;

pub struct CrashGuard {
    state_file: PathBuf,
}

impl CrashGuard {
    pub fn new(app_dir: &std::path::Path) -> Self {
        Self {
            state_file: app_dir.join("crash_guard.json"),
        }
    }

    /// Save current state so it can be recovered after crash
    pub fn save_state(&self, state: &CrashState) -> Result<(), String> {
        let json = serde_json::to_string(state).map_err(|e| e.to_string())?;
        std::fs::write(&self.state_file, json).map_err(|e| e.to_string())
    }

    /// Check if there was a crash (state file exists from previous run)
    pub fn check_previous_crash(&self) -> Option<CrashState> {
        if self.state_file.exists() {
            let content = std::fs::read_to_string(&self.state_file).ok()?;
            serde_json::from_str(&content).ok()
        } else {
            None
        }
    }

    /// Clear crash state (app started cleanly)
    pub fn clear(&self) {
        std::fs::remove_file(&self.state_file).ok();
    }

    /// Mark app as running (so next startup can detect crash)
    pub fn mark_running(&self) {
        let state = CrashState {
            running: true,
            started_at: chrono::Utc::now().to_rfc3339(),
            active_mission: None,
            active_containers: vec![],
        };
        self.save_state(&state).ok();
    }

    /// Mark app as stopped cleanly
    pub fn mark_stopped(&self) {
        self.clear();
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrashState {
    pub running: bool,
    pub started_at: String,
    pub active_mission: Option<String>,
    pub active_containers: Vec<String>,
}
