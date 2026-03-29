use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    #[serde(rename = "type")]
    pub plugin_type: PluginType,
    pub description: String,
    pub author: String,
    pub permissions: Vec<String>,
    pub entry_point: String,
    pub config_schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PluginType {
    Executor,
    Channel,
    Provider,
    Action,
    Widget,
}

impl PluginManifest {
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| format!("Invalid plugin.json: {}", e))
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Plugin name is required".into());
        }
        if self.version.is_empty() {
            return Err("Plugin version is required".into());
        }
        if self.entry_point.is_empty() {
            return Err("Entry point is required".into());
        }
        // Check for dangerous permissions
        if self.permissions.contains(&"system".to_string()) {
            return Err("'system' permission is not allowed for plugins".into());
        }
        Ok(())
    }
}
