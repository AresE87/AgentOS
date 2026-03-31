use super::manifest::{PluginManifest, PluginType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub path: PathBuf,
    pub enabled: bool,
    pub config: serde_json::Value,
}

pub struct PluginManager {
    plugins: HashMap<String, LoadedPlugin>,
    plugins_dir: PathBuf,
}

impl PluginManager {
    pub fn new(plugins_dir: PathBuf) -> Self {
        Self {
            plugins: HashMap::new(),
            plugins_dir,
        }
    }

    /// Discover all plugins in the plugins directory
    pub fn discover(&mut self) -> Result<Vec<PluginManifest>, String> {
        let mut manifests = vec![];

        if !self.plugins_dir.exists() {
            std::fs::create_dir_all(&self.plugins_dir).map_err(|e| e.to_string())?;
            return Ok(manifests);
        }

        let entries = std::fs::read_dir(&self.plugins_dir).map_err(|e| e.to_string())?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let manifest_path = path.join("plugin.json");
                if manifest_path.exists() {
                    match PluginManifest::load(&manifest_path) {
                        Ok(manifest) => {
                            if manifest.validate().is_ok() {
                                self.plugins.insert(
                                    manifest.name.clone(),
                                    LoadedPlugin {
                                        manifest: manifest.clone(),
                                        path: path.clone(),
                                        enabled: true,
                                        config: serde_json::Value::Object(Default::default()),
                                    },
                                );
                                manifests.push(manifest);
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to load plugin at {:?}: {}", path, e);
                        }
                    }
                }
            }
        }

        Ok(manifests)
    }

    /// Install a plugin from a directory
    pub fn install(&mut self, source_path: &Path) -> Result<PluginManifest, String> {
        let manifest_path = source_path.join("plugin.json");
        let manifest = PluginManifest::load(&manifest_path)?;
        manifest.validate()?;

        let dest = self.plugins_dir.join(&manifest.name);
        if dest.exists() {
            return Err(format!("Plugin '{}' is already installed", manifest.name));
        }

        // Copy plugin directory
        copy_dir_recursive(source_path, &dest)?;

        self.plugins.insert(
            manifest.name.clone(),
            LoadedPlugin {
                manifest: manifest.clone(),
                path: dest,
                enabled: true,
                config: serde_json::Value::Object(Default::default()),
            },
        );

        Ok(manifest)
    }

    /// Uninstall a plugin
    pub fn uninstall(&mut self, name: &str) -> Result<(), String> {
        if let Some(plugin) = self.plugins.remove(name) {
            if plugin.path.exists() {
                std::fs::remove_dir_all(&plugin.path).map_err(|e| e.to_string())?;
            }
            Ok(())
        } else {
            Err(format!("Plugin '{}' not found", name))
        }
    }

    /// Enable/disable a plugin
    pub fn set_enabled(&mut self, name: &str, enabled: bool) -> Result<(), String> {
        if let Some(plugin) = self.plugins.get_mut(name) {
            plugin.enabled = enabled;
            Ok(())
        } else {
            Err(format!("Plugin '{}' not found", name))
        }
    }

    /// Execute a plugin (runs the entry_point script via PowerShell/Python)
    pub async fn execute(&self, name: &str, input: &str) -> Result<String, String> {
        let plugin = self
            .plugins
            .get(name)
            .ok_or_else(|| format!("Plugin '{}' not found", name))?;

        if !plugin.enabled {
            return Err(format!("Plugin '{}' is disabled", name));
        }

        let entry = plugin.path.join(&plugin.manifest.entry_point);
        if !entry.exists() {
            return Err(format!("Entry point not found: {:?}", entry));
        }

        // Execute based on entry point extension
        let ext = entry.extension().and_then(|e| e.to_str()).unwrap_or("");
        let output = match ext {
            "ps1" => tokio::process::Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-File",
                    &entry.to_string_lossy(),
                    input,
                ])
                .current_dir(&plugin.path)
                .output()
                .await
                .map_err(|e| e.to_string())?,
            "py" => tokio::process::Command::new("python")
                .args([&entry.to_string_lossy().to_string(), &input.to_string()])
                .current_dir(&plugin.path)
                .output()
                .await
                .map_err(|e| e.to_string())?,
            _ => {
                return Err(format!("Unsupported entry point type: .{}", ext));
            }
        };

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    /// List all plugins
    pub fn list(&self) -> Vec<&LoadedPlugin> {
        self.plugins.values().collect()
    }

    /// Get a specific plugin
    #[allow(dead_code)]
    pub fn get(&self, name: &str) -> Option<&LoadedPlugin> {
        self.plugins.get(name)
    }

    /// Get all enabled plugins of a specific type
    #[allow(dead_code)]
    pub fn get_by_type(&self, plugin_type: &PluginType) -> Vec<&LoadedPlugin> {
        self.plugins
            .values()
            .filter(|p| p.enabled && p.manifest.plugin_type == *plugin_type)
            .collect()
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())?.flatten() {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
