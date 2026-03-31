use serde::{Deserialize, Serialize};

/// A connected wearable device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WearableDevice {
    pub id: String,
    pub name: String,
    /// "watch", "ring", "glasses"
    pub device_type: String,
    pub connected: bool,
    pub battery_pct: u8,
}

/// Health data from a wearable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthData {
    pub device_id: String,
    pub heart_rate: Option<u32>,
    pub steps: Option<u64>,
    pub calories: Option<f64>,
    pub sleep_hours: Option<f64>,
    pub stress_level: Option<u8>,
    pub timestamp: String,
}

/// Manages wearable device connections and interactions
pub struct WearableManager {
    devices: Vec<WearableDevice>,
}

impl WearableManager {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
        }
    }

    /// Scan for nearby wearable devices (simulated discovery)
    pub fn scan_devices(&self) -> Vec<WearableDevice> {
        // In production, this would use BLE scanning
        vec![
            WearableDevice {
                id: "wearable-001".into(),
                name: "Apple Watch Ultra".into(),
                device_type: "watch".into(),
                connected: false,
                battery_pct: 85,
            },
            WearableDevice {
                id: "wearable-002".into(),
                name: "Oura Ring Gen 3".into(),
                device_type: "ring".into(),
                connected: false,
                battery_pct: 72,
            },
            WearableDevice {
                id: "wearable-003".into(),
                name: "Ray-Ban Meta".into(),
                device_type: "glasses".into(),
                connected: false,
                battery_pct: 60,
            },
        ]
    }

    /// Connect to a wearable device by ID
    pub fn connect(&mut self, id: &str) -> Result<WearableDevice, String> {
        // Check if already connected
        if self.devices.iter().any(|d| d.id == id) {
            return Err(format!("Device {} already connected", id));
        }
        let scanned = self.scan_devices();
        if let Some(mut device) = scanned.into_iter().find(|d| d.id == id) {
            device.connected = true;
            self.devices.push(device.clone());
            Ok(device)
        } else {
            Err(format!("Device {} not found", id))
        }
    }

    /// Disconnect a wearable device by ID
    pub fn disconnect(&mut self, id: &str) -> Result<(), String> {
        let idx = self
            .devices
            .iter()
            .position(|d| d.id == id)
            .ok_or_else(|| format!("Device {} not connected", id))?;
        self.devices.remove(idx);
        Ok(())
    }

    /// List all currently connected wearable devices
    pub fn list_connected(&self) -> Vec<WearableDevice> {
        self.devices.clone()
    }

    /// Send a notification to a specific wearable
    pub fn send_notification(&self, id: &str, title: &str, body: &str) -> Result<(), String> {
        if !self.devices.iter().any(|d| d.id == id) {
            return Err(format!("Device {} not connected", id));
        }
        // In production, would push via BLE or companion app
        tracing::info!("Notification to {}: {} - {}", id, title, body);
        Ok(())
    }

    /// Get health data from a wearable device
    pub fn get_health_data(&self, id: &str) -> Result<HealthData, String> {
        if !self.devices.iter().any(|d| d.id == id) {
            return Err(format!("Device {} not connected", id));
        }
        Ok(HealthData {
            device_id: id.to_string(),
            heart_rate: Some(72),
            steps: Some(8542),
            calories: Some(420.5),
            sleep_hours: Some(7.2),
            stress_level: Some(35),
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }
}
