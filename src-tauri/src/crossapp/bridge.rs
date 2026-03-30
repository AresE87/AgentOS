use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a connection to an external application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConnection {
    pub id: String,
    pub app_name: String,
    /// Connection type: "com", "cli", or "api"
    pub connection_type: String,
    pub config: serde_json::Value,
    pub status: String,
}

/// Result from sending a command to an external app.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppActionResult {
    pub app_id: String,
    pub action: String,
    pub success: bool,
    pub output: String,
    pub duration_ms: u64,
}

/// Bridge that manages connections to external applications and cross-app data flow.
pub struct CrossAppBridge {
    apps: HashMap<String, AppConnection>,
}

impl CrossAppBridge {
    pub fn new() -> Self {
        let mut apps = HashMap::new();

        // Pre-register common Windows apps
        let excel = AppConnection {
            id: "app-excel".to_string(),
            app_name: "excel".to_string(),
            connection_type: "com".to_string(),
            config: serde_json::json!({
                "prog_id": "Excel.Application",
                "description": "Microsoft Excel via COM automation"
            }),
            status: "available".to_string(),
        };
        apps.insert(excel.id.clone(), excel);

        let notepad = AppConnection {
            id: "app-notepad".to_string(),
            app_name: "notepad".to_string(),
            connection_type: "cli".to_string(),
            config: serde_json::json!({
                "executable": "notepad.exe",
                "description": "Notepad via CLI"
            }),
            status: "available".to_string(),
        };
        apps.insert(notepad.id.clone(), notepad);

        let chrome = AppConnection {
            id: "app-chrome".to_string(),
            app_name: "chrome".to_string(),
            connection_type: "cli".to_string(),
            config: serde_json::json!({
                "executable": "chrome.exe",
                "description": "Google Chrome via CLI / DevTools Protocol"
            }),
            status: "available".to_string(),
        };
        apps.insert(chrome.id.clone(), chrome);

        Self { apps }
    }

    /// Register a new application connection.
    pub fn register_app(&mut self, conn: AppConnection) -> Result<AppConnection, String> {
        if self.apps.contains_key(&conn.id) {
            return Err(format!("App '{}' is already registered", conn.id));
        }
        let result = conn.clone();
        self.apps.insert(conn.id.clone(), conn);
        Ok(result)
    }

    /// List all registered applications.
    pub fn list_apps(&self) -> Vec<AppConnection> {
        self.apps.values().cloned().collect()
    }

    /// Send an action to a registered app (stub — real impl would use COM/CLI/API).
    pub fn send_to_app(
        &self,
        app_id: &str,
        action: &str,
        data: &serde_json::Value,
    ) -> Result<AppActionResult, String> {
        let app = self
            .apps
            .get(app_id)
            .ok_or_else(|| format!("App '{}' not found", app_id))?;

        // Stub: log the action and return a simulated result
        Ok(AppActionResult {
            app_id: app_id.to_string(),
            action: action.to_string(),
            success: true,
            output: format!(
                "Cross-app action stub: sent '{}' to {} ({}) with data_size={}",
                action,
                app.app_name,
                app.connection_type,
                data.to_string().len()
            ),
            duration_ms: 50,
        })
    }

    /// Get the status of a specific app connection.
    pub fn get_app_status(&self, app_id: &str) -> Result<AppConnection, String> {
        self.apps
            .get(app_id)
            .cloned()
            .ok_or_else(|| format!("App '{}' not found", app_id))
    }
}
