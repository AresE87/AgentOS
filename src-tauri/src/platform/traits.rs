use std::path::PathBuf;

pub trait PlatformProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn os_version(&self) -> String;
    fn default_shell(&self) -> String;
    fn app_data_dir(&self) -> PathBuf;
    fn screenshots_dir(&self) -> PathBuf;
    fn playbooks_dir(&self) -> PathBuf;
    fn can_capture_screen(&self) -> bool;
    fn can_control_input(&self) -> bool;
    fn open_url(&self, url: &str) -> Result<(), String>;
}
