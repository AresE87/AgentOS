use serde::{Deserialize, Serialize};

/// An IoT device managed by the controller
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoTDevice {
    pub id: String,
    pub name: String,
    /// "light", "thermostat", "switch", "sensor", "lock", "camera"
    pub device_type: String,
    /// Current state as a flexible JSON value
    pub state: serde_json::Value,
    /// Communication protocol: "mqtt", "http", "zigbee", "zwave", "ble"
    pub protocol: String,
}

/// Result of controlling an IoT device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlResult {
    pub device_id: String,
    pub action: String,
    pub success: bool,
    pub new_state: serde_json::Value,
}

/// Manages IoT device discovery, registration, and control
pub struct IoTController {
    devices: Vec<IoTDevice>,
}

impl IoTController {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
        }
    }

    /// Discover available IoT devices on the network (simulated)
    pub fn discover_devices(&self) -> Vec<IoTDevice> {
        vec![
            IoTDevice {
                id: "iot-light-001".into(),
                name: "Office Ceiling Light".into(),
                device_type: "light".into(),
                state: serde_json::json!({"on": true, "brightness": 80, "color": "#ffffff"}),
                protocol: "zigbee".into(),
            },
            IoTDevice {
                id: "iot-thermo-001".into(),
                name: "Office Thermostat".into(),
                device_type: "thermostat".into(),
                state: serde_json::json!({"temperature": 22.5, "target": 23.0, "mode": "auto"}),
                protocol: "mqtt".into(),
            },
            IoTDevice {
                id: "iot-switch-001".into(),
                name: "Desk Power Strip".into(),
                device_type: "switch".into(),
                state: serde_json::json!({"on": false}),
                protocol: "http".into(),
            },
            IoTDevice {
                id: "iot-sensor-001".into(),
                name: "Door Sensor".into(),
                device_type: "sensor".into(),
                state: serde_json::json!({"open": false, "battery": 90}),
                protocol: "zigbee".into(),
            },
        ]
    }

    /// Add/register a device to the controller
    pub fn add_device(&mut self, device: IoTDevice) -> Result<(), String> {
        if self.devices.iter().any(|d| d.id == device.id) {
            return Err(format!("Device {} already registered", device.id));
        }
        self.devices.push(device);
        Ok(())
    }

    /// Control a device by sending an action with a value
    pub fn control(
        &mut self,
        id: &str,
        action: &str,
        value: serde_json::Value,
    ) -> Result<ControlResult, String> {
        let device = self
            .devices
            .iter_mut()
            .find(|d| d.id == id)
            .ok_or_else(|| format!("Device {} not found", id))?;

        // Apply action to state
        match action {
            "turn_on" => {
                device.state["on"] = serde_json::json!(true);
            }
            "turn_off" => {
                device.state["on"] = serde_json::json!(false);
            }
            "set_temperature" => {
                device.state["target"] = value.clone();
            }
            "set_brightness" => {
                device.state["brightness"] = value.clone();
            }
            "set_color" => {
                device.state["color"] = value.clone();
            }
            "toggle" => {
                let current = device
                    .state
                    .get("on")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                device.state["on"] = serde_json::json!(!current);
            }
            _ => {
                device.state[action] = value.clone();
            }
        }

        Ok(ControlResult {
            device_id: id.to_string(),
            action: action.to_string(),
            success: true,
            new_state: device.state.clone(),
        })
    }

    /// Get the current state of a device
    pub fn get_state(&self, id: &str) -> Result<serde_json::Value, String> {
        self.devices
            .iter()
            .find(|d| d.id == id)
            .map(|d| d.state.clone())
            .ok_or_else(|| format!("Device {} not found", id))
    }

    /// List all registered devices
    pub fn list_devices(&self) -> Vec<IoTDevice> {
        self.devices.clone()
    }
}
