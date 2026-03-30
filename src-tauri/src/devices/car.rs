use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Supported vehicle communication protocols
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CarProtocol {
    Obd2,
    Canbus,
    Api,
}

impl std::fmt::Display for CarProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CarProtocol::Obd2 => write!(f, "obd2"),
            CarProtocol::Canbus => write!(f, "canbus"),
            CarProtocol::Api => write!(f, "api"),
        }
    }
}

/// Represents an active connection to a vehicle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarConnection {
    pub id: String,
    pub vehicle_name: String,
    pub protocol: CarProtocol,
    pub connected: bool,
    pub vehicle_data: Value,
}

/// Configuration used to establish a car connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarConfig {
    pub vehicle_name: String,
    pub protocol: CarProtocol,
    /// OBD2 / CAN-bus port or API base URL depending on protocol
    pub endpoint: Option<String>,
    /// Optional API key for cloud-based vehicle APIs (e.g. Tesla, GM)
    pub api_key: Option<String>,
}

/// Diagnostic snapshot from the vehicle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsReport {
    pub engine_rpm: Option<f64>,
    pub speed_kmh: Option<f64>,
    pub fuel_level_pct: Option<f64>,
    pub battery_voltage: Option<f64>,
    pub coolant_temp_c: Option<f64>,
    pub dtc_codes: Vec<String>,
    pub timestamp: String,
}

/// Location data from the vehicle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub heading: Option<f64>,
    pub speed_kmh: Option<f64>,
    pub timestamp: String,
}

/// CarAgent manages vehicle connections and interactions
pub struct CarAgent {
    connections: Vec<CarConnection>,
}

impl CarAgent {
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
        }
    }

    /// Connect to a vehicle using the given configuration
    pub fn connect(&mut self, config: CarConfig) -> Result<CarConnection, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let connection = CarConnection {
            id: id.clone(),
            vehicle_name: config.vehicle_name.clone(),
            protocol: config.protocol.clone(),
            connected: true,
            vehicle_data: serde_json::json!({
                "endpoint": config.endpoint,
                "protocol": config.protocol.to_string(),
                "connected_at": chrono::Utc::now().to_rfc3339(),
            }),
        };
        self.connections.push(connection.clone());
        tracing::info!(
            "Car connected: {} via {} (id={})",
            config.vehicle_name,
            config.protocol,
            id
        );
        Ok(connection)
    }

    /// Disconnect a vehicle by connection id
    pub fn disconnect(&mut self, id: &str) -> Result<(), String> {
        if let Some(conn) = self.connections.iter_mut().find(|c| c.id == id) {
            conn.connected = false;
            tracing::info!("Car disconnected: {}", conn.vehicle_name);
            Ok(())
        } else {
            Err(format!("No car connection with id {}", id))
        }
    }

    /// Get current vehicle data for a connection
    pub fn get_vehicle_data(&self, id: &str) -> Result<Value, String> {
        let conn = self
            .connections
            .iter()
            .find(|c| c.id == id && c.connected)
            .ok_or_else(|| format!("No active car connection with id {}", id))?;

        Ok(serde_json::json!({
            "id": conn.id,
            "vehicle_name": conn.vehicle_name,
            "protocol": conn.protocol,
            "connected": conn.connected,
            "data": conn.vehicle_data,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }))
    }

    /// Send a command to the vehicle (e.g. lock, unlock, climate)
    pub fn send_command(&self, id: &str, command: &str) -> Result<Value, String> {
        let conn = self
            .connections
            .iter()
            .find(|c| c.id == id && c.connected)
            .ok_or_else(|| format!("No active car connection with id {}", id))?;

        tracing::info!(
            "Sending command '{}' to {} via {}",
            command,
            conn.vehicle_name,
            conn.protocol
        );

        // Stub: in production this would communicate over OBD2/CAN/API
        Ok(serde_json::json!({
            "command": command,
            "vehicle": conn.vehicle_name,
            "status": "sent",
            "result": "ok",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }))
    }

    /// Get diagnostics report from the vehicle
    pub fn get_diagnostics(&self, id: &str) -> Result<DiagnosticsReport, String> {
        let conn = self
            .connections
            .iter()
            .find(|c| c.id == id && c.connected)
            .ok_or_else(|| format!("No active car connection with id {}", id))?;

        // Stub diagnostics — real impl reads from OBD2/CAN bus
        Ok(DiagnosticsReport {
            engine_rpm: Some(850.0),
            speed_kmh: Some(0.0),
            fuel_level_pct: Some(72.5),
            battery_voltage: Some(12.6),
            coolant_temp_c: Some(90.0),
            dtc_codes: vec![],
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Get current GPS location of the vehicle
    pub fn get_location(&self, id: &str) -> Result<VehicleLocation, String> {
        let _conn = self
            .connections
            .iter()
            .find(|c| c.id == id && c.connected)
            .ok_or_else(|| format!("No active car connection with id {}", id))?;

        // Stub location
        Ok(VehicleLocation {
            latitude: 40.7128,
            longitude: -74.0060,
            heading: Some(180.0),
            speed_kmh: Some(0.0),
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// List all connections (active and inactive)
    pub fn list_connections(&self) -> &[CarConnection] {
        &self.connections
    }
}
