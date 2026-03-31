use super::traits::PlatformProvider;
use std::path::PathBuf;

pub struct LinuxPlatform;

impl LinuxPlatform {
    pub fn new() -> Self {
        Self
    }
}

impl PlatformProvider for LinuxPlatform {
    fn name(&self) -> &'static str {
        "linux"
    }

    fn os_version(&self) -> String {
        // Try to read /etc/os-release for distro info
        std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|content| {
                content
                    .lines()
                    .find(|l| l.starts_with("PRETTY_NAME="))
                    .map(|l| {
                        l.trim_start_matches("PRETTY_NAME=")
                            .trim_matches('"')
                            .to_string()
                    })
            })
            .unwrap_or_else(|| "Linux".to_string())
    }

    fn default_shell(&self) -> String {
        "bash".to_string()
    }

    fn app_data_dir(&self) -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".config/AgentOS")
    }

    fn screenshots_dir(&self) -> PathBuf {
        self.app_data_dir().join("screenshots")
    }

    fn playbooks_dir(&self) -> PathBuf {
        self.app_data_dir().join("playbooks")
    }

    fn can_capture_screen(&self) -> bool {
        false // TODO: implement with X11/Wayland APIs
    }

    fn can_control_input(&self) -> bool {
        false
    }

    fn open_url(&self, url: &str) -> Result<(), String> {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_platform_reports_honest_capabilities() {
        let platform = LinuxPlatform::new();
        assert_eq!(platform.name(), "linux");
        assert_eq!(platform.default_shell(), "bash");
        assert!(!platform.can_capture_screen());
        assert!(!platform.can_control_input());
        assert!(platform
            .app_data_dir()
            .to_string_lossy()
            .contains(".config"));
    }
}
