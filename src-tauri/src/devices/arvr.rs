use serde::{Deserialize, Serialize};

/// AR/VR headset configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ARVRConfig {
    /// Headset type: "quest", "vision_pro", "pico", etc.
    pub headset_type: String,
    /// Connection method: "usb", "wifi", "bluetooth"
    pub connection: String,
    /// Display resolution (e.g. "2064x2208")
    pub resolution: String,
    /// Field of view in degrees
    pub fov: f64,
}

/// Status of the AR/VR agent connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ARVRStatus {
    pub connected: bool,
    pub headset_type: String,
    pub battery_pct: Option<u8>,
    pub tracking_active: bool,
    pub overlay_active: bool,
}

/// Manages the AR/VR agent connection and spatial commands
pub struct ARVRAgent {
    config: Option<ARVRConfig>,
    connected: bool,
    overlay_text: Option<String>,
}

impl ARVRAgent {
    pub fn new() -> Self {
        Self {
            config: None,
            connected: false,
            overlay_text: None,
        }
    }

    /// Connect to an AR/VR headset with the given config
    pub fn connect(&mut self, config: ARVRConfig) -> Result<ARVRStatus, String> {
        // Validate config
        if config.headset_type.is_empty() {
            return Err("headset_type is required".into());
        }
        self.config = Some(config.clone());
        self.connected = true;
        Ok(ARVRStatus {
            connected: true,
            headset_type: config.headset_type,
            battery_pct: Some(100),
            tracking_active: true,
            overlay_active: false,
        })
    }

    /// Disconnect from the headset
    pub fn disconnect(&mut self) -> Result<(), String> {
        self.connected = false;
        self.config = None;
        self.overlay_text = None;
        Ok(())
    }

    /// Get the current AR/VR agent status
    pub fn get_status(&self) -> ARVRStatus {
        match &self.config {
            Some(cfg) => ARVRStatus {
                connected: self.connected,
                headset_type: cfg.headset_type.clone(),
                battery_pct: Some(95),
                tracking_active: self.connected,
                overlay_active: self.overlay_text.is_some(),
            },
            None => ARVRStatus {
                connected: false,
                headset_type: String::new(),
                battery_pct: None,
                tracking_active: false,
                overlay_active: false,
            },
        }
    }

    /// Send overlay text to display in the AR/VR view
    pub fn send_overlay(&mut self, text: String) -> Result<(), String> {
        if !self.connected {
            return Err("Not connected to any headset".into());
        }
        self.overlay_text = Some(text);
        Ok(())
    }

    /// Send a spatial command (e.g., "highlight", "point", "place_panel")
    pub fn send_spatial_command(
        &self,
        action: &str,
        _params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        if !self.connected {
            return Err("Not connected to any headset".into());
        }
        Ok(serde_json::json!({
            "ok": true,
            "action": action,
            "status": "executed"
        }))
    }
}
