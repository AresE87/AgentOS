use serde::{Deserialize, Serialize};

/// TV display mode configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TVConfig {
    /// "dashboard", "kiosk", "slideshow"
    pub display_mode: String,
    /// Auto-refresh interval in seconds
    pub auto_refresh_secs: u64,
    /// Type of content to show: "board", "metrics", "feed", "swarm", "ambient"
    pub content_type: String,
}

impl Default for TVConfig {
    fn default() -> Self {
        Self {
            display_mode: "dashboard".into(),
            auto_refresh_secs: 30,
            content_type: "board".into(),
        }
    }
}

/// Status of TV display mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TVStatus {
    pub enabled: bool,
    pub config: Option<TVConfig>,
    pub current_screen: String,
    pub uptime_secs: u64,
}

/// Manages TV / large display mode for team dashboards
pub struct TVDisplayMode {
    enabled: bool,
    config: Option<TVConfig>,
    started_at: Option<std::time::Instant>,
}

impl TVDisplayMode {
    pub fn new() -> Self {
        Self {
            enabled: false,
            config: None,
            started_at: None,
        }
    }

    /// Enable TV display mode
    pub fn enable(&mut self, config: TVConfig) -> Result<TVStatus, String> {
        if !["dashboard", "kiosk", "slideshow"].contains(&config.display_mode.as_str()) {
            return Err(format!(
                "Invalid display_mode '{}'. Must be dashboard, kiosk, or slideshow",
                config.display_mode
            ));
        }
        if config.auto_refresh_secs < 1 {
            return Err("auto_refresh_secs must be >= 1".into());
        }
        self.enabled = true;
        self.config = Some(config);
        self.started_at = Some(std::time::Instant::now());
        Ok(self.get_status())
    }

    /// Disable TV display mode
    pub fn disable(&mut self) -> Result<(), String> {
        self.enabled = false;
        self.config = None;
        self.started_at = None;
        Ok(())
    }

    /// Get current TV display status
    pub fn get_status(&self) -> TVStatus {
        let uptime = self.started_at.map(|s| s.elapsed().as_secs()).unwrap_or(0);
        TVStatus {
            enabled: self.enabled,
            config: self.config.clone(),
            current_screen: self
                .config
                .as_ref()
                .map(|c| c.content_type.clone())
                .unwrap_or_else(|| "none".into()),
            uptime_secs: uptime,
        }
    }

    /// Change the content type being displayed
    pub fn set_content(&mut self, content_type: &str) -> Result<TVStatus, String> {
        if !self.enabled {
            return Err("TV display mode is not enabled".into());
        }
        let valid = ["board", "metrics", "feed", "swarm", "ambient"];
        if !valid.contains(&content_type) {
            return Err(format!(
                "Invalid content_type '{}'. Must be one of: {:?}",
                content_type, valid
            ));
        }
        if let Some(ref mut cfg) = self.config {
            cfg.content_type = content_type.to_string();
        }
        Ok(self.get_status())
    }
}
