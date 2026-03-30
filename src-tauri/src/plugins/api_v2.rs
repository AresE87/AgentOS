use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// UI definition for a plugin — pages and widgets it provides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginUI {
    /// Full pages contributed by the plugin
    pub pages: Vec<PluginPage>,
    /// Widgets contributed by the plugin (dashboard cards, etc.)
    pub widgets: Vec<PluginWidget>,
}

/// A full page contributed by a plugin, shown in the sidebar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPage {
    /// Unique page identifier
    pub id: String,
    /// Display title
    pub title: String,
    /// Icon name (e.g. lucide icon key)
    pub icon: String,
    /// Position in the sidebar (lower = higher)
    pub sidebar_position: u32,
    /// HTML content for the page
    pub html_content: String,
}

/// A widget contributed by a plugin (e.g. dashboard card).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginWidget {
    /// Unique widget identifier
    pub id: String,
    /// Display title
    pub title: String,
    /// Width in grid units
    pub width: u32,
    /// Height in grid units
    pub height: u32,
    /// HTML content for the widget
    pub html_content: String,
}

/// Extension API v2 — provides plugin UI registration, method invocation, and scoped storage.
pub struct ExtensionAPIv2 {
    /// Plugin UI registrations: plugin_name -> PluginUI
    ui_registry: HashMap<String, PluginUI>,
    /// Plugin-scoped storage path (SQLite-backed)
    storage_path: PathBuf,
}

impl ExtensionAPIv2 {
    pub fn new(storage_path: PathBuf) -> Self {
        // Ensure the storage database exists
        if let Ok(conn) = rusqlite::Connection::open(&storage_path) {
            let _ = conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS plugin_storage (
                    plugin_name TEXT NOT NULL,
                    key TEXT NOT NULL,
                    value TEXT NOT NULL,
                    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                    PRIMARY KEY (plugin_name, key)
                );"
            );
        }

        Self {
            ui_registry: HashMap::new(),
            storage_path,
        }
    }

    /// Register a plugin UI (pages + widgets).
    pub fn register_ui(&mut self, name: &str, ui: PluginUI) {
        self.ui_registry.insert(name.to_string(), ui);
    }

    /// Unregister a plugin UI.
    pub fn unregister_ui(&mut self, name: &str) {
        self.ui_registry.remove(name);
    }

    /// Get the UI definition for a plugin.
    pub fn get_plugin_ui(&self, name: &str) -> Option<&PluginUI> {
        self.ui_registry.get(name)
    }

    /// List all plugins that have registered UI.
    pub fn list_plugin_uis(&self) -> Vec<String> {
        self.ui_registry.keys().cloned().collect()
    }

    /// Invoke a method on a plugin by name.
    /// In this stub, we look up the plugin in the registry and return a structured response.
    /// A real implementation would delegate to the plugin's script runtime.
    pub fn invoke_plugin_method(
        &self,
        name: &str,
        method: &str,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        if !self.ui_registry.contains_key(name) {
            return Err(format!("Plugin '{}' not found in API v2 registry", name));
        }

        // Stub: return the invocation details for now
        Ok(serde_json::json!({
            "plugin": name,
            "method": method,
            "args": args,
            "result": null,
            "status": "stub_ok"
        }))
    }

    /// Get a value from plugin-scoped storage.
    pub fn plugin_storage_get(&self, name: &str, key: &str) -> Result<Option<String>, String> {
        let conn = rusqlite::Connection::open(&self.storage_path)
            .map_err(|e| format!("Failed to open plugin storage: {}", e))?;

        let mut stmt = conn
            .prepare("SELECT value FROM plugin_storage WHERE plugin_name = ?1 AND key = ?2")
            .map_err(|e| e.to_string())?;

        let result = stmt
            .query_row(rusqlite::params![name, key], |row| row.get::<_, String>(0))
            .ok();

        Ok(result)
    }

    /// Set a value in plugin-scoped storage.
    pub fn plugin_storage_set(&self, name: &str, key: &str, value: &str) -> Result<(), String> {
        let conn = rusqlite::Connection::open(&self.storage_path)
            .map_err(|e| format!("Failed to open plugin storage: {}", e))?;

        conn.execute(
            "INSERT INTO plugin_storage (plugin_name, key, value, updated_at)
             VALUES (?1, ?2, ?3, datetime('now'))
             ON CONFLICT(plugin_name, key) DO UPDATE SET value = ?3, updated_at = datetime('now')",
            rusqlite::params![name, key, value],
        )
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Delete a key from plugin-scoped storage.
    pub fn plugin_storage_delete(&self, name: &str, key: &str) -> Result<bool, String> {
        let conn = rusqlite::Connection::open(&self.storage_path)
            .map_err(|e| format!("Failed to open plugin storage: {}", e))?;

        let rows = conn
            .execute(
                "DELETE FROM plugin_storage WHERE plugin_name = ?1 AND key = ?2",
                rusqlite::params![name, key],
            )
            .map_err(|e| e.to_string())?;

        Ok(rows > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_get_ui() {
        let dir = std::env::temp_dir().join("agentos_test_plugin_v2.db");
        let mut api = ExtensionAPIv2::new(dir);
        let ui = PluginUI {
            pages: vec![PluginPage {
                id: "page1".into(),
                title: "Test Page".into(),
                icon: "settings".into(),
                sidebar_position: 1,
                html_content: "<h1>Test</h1>".into(),
            }],
            widgets: vec![],
        };
        api.register_ui("test-plugin", ui);
        assert!(api.get_plugin_ui("test-plugin").is_some());
        assert!(api.get_plugin_ui("unknown").is_none());
    }

    #[test]
    fn test_invoke_method_stub() {
        let dir = std::env::temp_dir().join("agentos_test_plugin_v2b.db");
        let mut api = ExtensionAPIv2::new(dir);
        api.register_ui("my-plugin", PluginUI { pages: vec![], widgets: vec![] });
        let result = api.invoke_plugin_method("my-plugin", "hello", &serde_json::json!({}));
        assert!(result.is_ok());
    }
}
