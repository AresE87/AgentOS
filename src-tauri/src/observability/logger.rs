use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,  // "info", "warn", "error", "debug"
    pub module: String, // "brain", "pipeline", "mesh", etc.
    pub message: String,
    pub trace_id: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

pub struct StructuredLogger {
    log_dir: PathBuf,
    max_file_size: u64, // bytes, default 10MB
    max_files: usize,   // default 5 rotated files
}

impl StructuredLogger {
    pub fn new(log_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&log_dir).ok();
        Self {
            log_dir,
            max_file_size: 10 * 1024 * 1024,
            max_files: 5,
        }
    }

    pub fn log(
        &self,
        level: &str,
        module: &str,
        message: &str,
        trace_id: Option<&str>,
        metadata: Option<serde_json::Value>,
    ) {
        let entry = LogEntry {
            timestamp: Utc::now().to_rfc3339(),
            level: level.to_string(),
            module: module.to_string(),
            message: message.to_string(),
            trace_id: trace_id.map(|s| s.to_string()),
            metadata,
        };

        if let Ok(json) = serde_json::to_string(&entry) {
            let log_file = self.log_dir.join("agentos.log");
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_file) {
                writeln!(file, "{}", json).ok();

                // Check rotation
                if let Ok(meta) = file.metadata() {
                    if meta.len() > self.max_file_size {
                        self.rotate();
                    }
                }
            }
        }
    }

    fn rotate(&self) {
        let base = self.log_dir.join("agentos");
        // Rotate: .log.4 -> .log.5, .log.3 -> .log.4, etc.
        for i in (1..self.max_files).rev() {
            let from = format!("{}.log.{}", base.display(), i);
            let to = format!("{}.log.{}", base.display(), i + 1);
            std::fs::rename(&from, &to).ok();
        }
        let current = self.log_dir.join("agentos.log");
        let first = format!("{}.log.1", base.display());
        std::fs::rename(current, first).ok();
    }

    pub fn get_recent(
        &self,
        limit: usize,
        level_filter: Option<&str>,
        module_filter: Option<&str>,
    ) -> Vec<LogEntry> {
        let log_file = self.log_dir.join("agentos.log");
        let content = std::fs::read_to_string(&log_file).unwrap_or_default();

        content
            .lines()
            .rev()
            .filter_map(|line| serde_json::from_str::<LogEntry>(line).ok())
            .filter(|e| level_filter.map(|l| e.level == l).unwrap_or(true))
            .filter(|e| module_filter.map(|m| e.module == m).unwrap_or(true))
            .take(limit)
            .collect()
    }

    pub fn export(&self) -> Result<String, String> {
        let log_file = self.log_dir.join("agentos.log");
        std::fs::read_to_string(&log_file).map_err(|e| e.to_string())
    }
}
