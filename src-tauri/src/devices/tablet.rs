use serde::{Deserialize, Serialize};

/// Tablet mode configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabletConfig {
    pub touch_enabled: bool,
    pub gesture_support: bool,
    pub font_scale: f64,
    /// "compact", "regular", "expanded"
    pub layout: String,
}

impl Default for TabletConfig {
    fn default() -> Self {
        Self {
            touch_enabled: true,
            gesture_support: true,
            font_scale: 1.0,
            layout: "regular".into(),
        }
    }
}

/// Status of tablet mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabletStatus {
    pub enabled: bool,
    pub config: Option<TabletConfig>,
}

/// Manages tablet mode UI adaptations
pub struct TabletMode {
    enabled: bool,
    config: Option<TabletConfig>,
}

impl TabletMode {
    pub fn new() -> Self {
        Self {
            enabled: false,
            config: None,
        }
    }

    /// Enable tablet mode with the given configuration
    pub fn enable(&mut self, config: TabletConfig) -> Result<TabletStatus, String> {
        if !["compact", "regular", "expanded"].contains(&config.layout.as_str()) {
            return Err(format!(
                "Invalid layout '{}'. Must be compact, regular, or expanded",
                config.layout
            ));
        }
        if config.font_scale < 0.5 || config.font_scale > 3.0 {
            return Err("font_scale must be between 0.5 and 3.0".into());
        }
        self.enabled = true;
        self.config = Some(config);
        Ok(self.get_status())
    }

    /// Disable tablet mode
    pub fn disable(&mut self) -> Result<(), String> {
        self.enabled = false;
        self.config = None;
        Ok(())
    }

    /// Get current tablet mode status
    pub fn get_status(&self) -> TabletStatus {
        TabletStatus {
            enabled: self.enabled,
            config: self.config.clone(),
        }
    }

    /// Adjust the layout mode without changing other settings
    pub fn adjust_layout(&mut self, layout: &str) -> Result<TabletStatus, String> {
        if !self.enabled {
            return Err("Tablet mode is not enabled".into());
        }
        if !["compact", "regular", "expanded"].contains(&layout) {
            return Err(format!(
                "Invalid layout '{}'. Must be compact, regular, or expanded",
                layout
            ));
        }
        if let Some(ref mut cfg) = self.config {
            cfg.layout = layout.to_string();
        }
        Ok(self.get_status())
    }
}
