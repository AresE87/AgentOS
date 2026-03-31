use super::traits::PlatformProvider;
use std::path::PathBuf;

pub struct WindowsPlatform;

impl WindowsPlatform {
    pub fn new() -> Self {
        Self
    }
}

impl PlatformProvider for WindowsPlatform {
    fn name(&self) -> &'static str {
        "windows"
    }

    fn os_version(&self) -> String {
        std::process::Command::new("cmd")
            .args(["/C", "ver"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "Windows 10+".to_string())
    }

    fn default_shell(&self) -> String {
        "powershell".to_string()
    }

    fn app_data_dir(&self) -> PathBuf {
        let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(appdata).join("AgentOS")
    }

    fn screenshots_dir(&self) -> PathBuf {
        self.app_data_dir().join("screenshots")
    }

    fn playbooks_dir(&self) -> PathBuf {
        self.app_data_dir().join("playbooks")
    }

    fn can_capture_screen(&self) -> bool {
        true
    }

    fn can_control_input(&self) -> bool {
        true
    }

    fn open_url(&self, url: &str) -> Result<(), String> {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_platform_reports_honest_capabilities() {
        let platform = WindowsPlatform::new();
        assert_eq!(platform.name(), "windows");
        assert_eq!(platform.default_shell(), "powershell");
        assert!(platform.can_capture_screen());
        assert!(platform.can_control_input());
    }
}
