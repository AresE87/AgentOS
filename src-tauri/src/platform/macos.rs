use super::traits::PlatformProvider;
use std::path::PathBuf;

pub struct MacosPlatform;

impl MacosPlatform {
    pub fn new() -> Self {
        Self
    }
}

impl PlatformProvider for MacosPlatform {
    fn name(&self) -> &'static str {
        "macos"
    }

    fn os_version(&self) -> String {
        std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| format!("macOS {}", s.trim()))
            .unwrap_or_else(|| "macOS".to_string())
    }

    fn default_shell(&self) -> String {
        "zsh".to_string()
    }

    fn app_data_dir(&self) -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join("Library/Application Support/AgentOS")
    }

    fn screenshots_dir(&self) -> PathBuf {
        self.app_data_dir().join("screenshots")
    }

    fn playbooks_dir(&self) -> PathBuf {
        self.app_data_dir().join("playbooks")
    }

    fn can_capture_screen(&self) -> bool {
        false // TODO: implement with macOS APIs
    }

    fn can_control_input(&self) -> bool {
        false
    }

    fn open_url(&self, url: &str) -> Result<(), String> {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
