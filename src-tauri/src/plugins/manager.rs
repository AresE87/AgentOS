use super::manifest::{PluginManifest, PluginType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const STATE_FILE: &str = ".agentos-plugin-state.json";
const BACKUP_DIR: &str = ".backups";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PluginLifecycleState {
    Installed,
    Enabled,
    Disabled,
    Updated,
    RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginLifecycle {
    pub state: PluginLifecycleState,
    pub installed_at: String,
    pub last_updated_at: String,
    pub rollback_version: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub path: PathBuf,
    pub enabled: bool,
    pub config: serde_json::Value,
    pub lifecycle: PluginLifecycle,
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

    pub fn discover(&mut self) -> Result<Vec<PluginManifest>, String> {
        let mut manifests = vec![];
        std::fs::create_dir_all(&self.plugins_dir).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(self.backups_root()).map_err(|e| e.to_string())?;

        let entries = std::fs::read_dir(&self.plugins_dir).map_err(|e| e.to_string())?;
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() || path.file_name().and_then(|n| n.to_str()) == Some(BACKUP_DIR) {
                continue;
            }
            let manifest_path = path.join("plugin.json");
            if !manifest_path.exists() {
                continue;
            }
            match Self::load_plugin_from_path(&path) {
                Ok(plugin) => {
                    manifests.push(plugin.manifest.clone());
                    self.plugins.insert(plugin.manifest.name.clone(), plugin);
                }
                Err(e) => {
                    tracing::warn!("Failed to discover plugin at {:?}: {}", path, e);
                }
            }
        }

        Ok(manifests)
    }

    pub fn install(&mut self, source_path: &Path) -> Result<PluginManifest, String> {
        let manifest = Self::validate_plugin_source(source_path)?;
        let dest = self.plugins_dir.join(&manifest.name);
        if dest.exists() {
            return Err(format!("Plugin '{}' is already installed", manifest.name));
        }

        copy_dir_recursive(source_path, &dest)?;
        let plugin = Self::plugin_from_install(
            manifest.clone(),
            dest,
            true,
            None,
            PluginLifecycleState::Enabled,
        );
        Self::persist_plugin_state(&plugin)?;
        self.plugins.insert(manifest.name.clone(), plugin);
        Ok(manifest)
    }

    pub fn update(&mut self, name: &str, source_path: &Path) -> Result<PluginManifest, String> {
        let current = self
            .plugins
            .get(name)
            .cloned()
            .ok_or_else(|| format!("Plugin '{}' not found", name))?;
        let manifest = Self::validate_plugin_source(source_path)?;
        if manifest.name != current.manifest.name {
            return Err(format!(
                "Plugin update name mismatch: expected '{}', got '{}'",
                current.manifest.name, manifest.name
            ));
        }
        Self::ensure_version_upgrade(&current.manifest.version, &manifest.version)?;

        let backup_path = self.backup_path(&current.manifest.name, &current.manifest.version);
        if backup_path.exists() {
            std::fs::remove_dir_all(&backup_path).map_err(|e| e.to_string())?;
        }
        copy_dir_recursive(&current.path, &backup_path)?;

        if current.path.exists() {
            std::fs::remove_dir_all(&current.path).map_err(|e| e.to_string())?;
        }
        if let Err(error) = copy_dir_recursive(source_path, &current.path) {
            let _ = std::fs::remove_dir_all(&current.path);
            let _ = copy_dir_recursive(&backup_path, &current.path);
            return Err(format!("Plugin update failed: {}", error));
        }

        let mut plugin = Self::plugin_from_install(
            manifest.clone(),
            current.path.clone(),
            current.enabled,
            Some(current.manifest.version.clone()),
            PluginLifecycleState::Updated,
        );
        plugin.config = current.config;
        plugin.lifecycle.installed_at = current.lifecycle.installed_at;
        Self::persist_plugin_state(&plugin)?;
        self.plugins.insert(name.to_string(), plugin);
        Ok(manifest)
    }

    pub fn rollback(&mut self, name: &str) -> Result<PluginManifest, String> {
        let current = self
            .plugins
            .get(name)
            .cloned()
            .ok_or_else(|| format!("Plugin '{}' not found", name))?;
        let rollback_path = self.latest_backup_path(name)?;
        let manifest = Self::validate_plugin_source(&rollback_path)?;

        if current.path.exists() {
            std::fs::remove_dir_all(&current.path).map_err(|e| e.to_string())?;
        }
        copy_dir_recursive(&rollback_path, &current.path)?;

        let mut plugin = Self::plugin_from_install(
            manifest.clone(),
            current.path.clone(),
            current.enabled,
            Some(current.manifest.version.clone()),
            PluginLifecycleState::RolledBack,
        );
        plugin.config = current.config;
        plugin.lifecycle.installed_at = current.lifecycle.installed_at;
        Self::persist_plugin_state(&plugin)?;
        self.plugins.insert(name.to_string(), plugin);
        Ok(manifest)
    }

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

    pub fn set_enabled(&mut self, name: &str, enabled: bool) -> Result<(), String> {
        if let Some(plugin) = self.plugins.get_mut(name) {
            plugin.enabled = enabled;
            plugin.lifecycle.state = if enabled {
                PluginLifecycleState::Enabled
            } else {
                PluginLifecycleState::Disabled
            };
            plugin.lifecycle.last_updated_at = chrono::Utc::now().to_rfc3339();
            Self::persist_plugin_state(plugin)?;
            Ok(())
        } else {
            Err(format!("Plugin '{}' not found", name))
        }
    }

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

        let ext = entry.extension().and_then(|e| e.to_str()).unwrap_or("");
        let output = match ext {
            "ps1" => {
                let mut cmd = tokio::process::Command::new("powershell");
                cmd.args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-File",
                    &entry.to_string_lossy(),
                    input,
                ])
                .current_dir(&plugin.path);
                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    cmd.creation_flags(0x08000000);
                }
                cmd.output().await.map_err(|e| e.to_string())?
            },
            "py" => tokio::process::Command::new("python")
                .args([&entry.to_string_lossy().to_string(), &input.to_string()])
                .current_dir(&plugin.path)
                .output()
                .await
                .map_err(|e| e.to_string())?,
            _ => return Err(format!("Unsupported entry point type: .{}", ext)),
        };

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    pub fn list(&self) -> Vec<&LoadedPlugin> {
        self.plugins.values().collect()
    }

    pub fn get(&self, name: &str) -> Option<&LoadedPlugin> {
        self.plugins.get(name)
    }

    pub fn get_by_type(&self, plugin_type: &PluginType) -> Vec<&LoadedPlugin> {
        self.plugins
            .values()
            .filter(|p| p.enabled && p.manifest.plugin_type == *plugin_type)
            .collect()
    }

    fn validate_plugin_source(source_path: &Path) -> Result<PluginManifest, String> {
        let manifest_path = source_path.join("plugin.json");
        let manifest = PluginManifest::load(&manifest_path)?;
        manifest.validate()?;
        Self::validate_version(&manifest.version)?;
        let entry = source_path.join(&manifest.entry_point);
        if !entry.exists() {
            return Err(format!("Plugin entry point not found: {}", entry.display()));
        }
        Ok(manifest)
    }

    fn validate_version(version: &str) -> Result<(), String> {
        let parts: Vec<_> = version.split('.').collect();
        if parts.len() != 3 || parts.iter().any(|part| part.parse::<u64>().is_err()) {
            return Err(format!(
                "Plugin version '{}' must use semantic version format x.y.z",
                version
            ));
        }
        Ok(())
    }

    fn ensure_version_upgrade(current: &str, next: &str) -> Result<(), String> {
        let current_parts: Vec<u64> = current
            .split('.')
            .map(|part| part.parse::<u64>().unwrap_or(0))
            .collect();
        let next_parts: Vec<u64> = next
            .split('.')
            .map(|part| part.parse::<u64>().unwrap_or(0))
            .collect();
        if next_parts <= current_parts {
            return Err(format!(
                "Plugin update must increase version (current {}, new {})",
                current, next
            ));
        }
        Ok(())
    }

    fn load_plugin_from_path(path: &Path) -> Result<LoadedPlugin, String> {
        let manifest = Self::validate_plugin_source(path)?;
        let lifecycle = Self::load_plugin_state(path, &manifest)?;
        Ok(LoadedPlugin {
            manifest,
            path: path.to_path_buf(),
            enabled: lifecycle.state != PluginLifecycleState::Disabled,
            config: serde_json::Value::Object(Default::default()),
            lifecycle,
        })
    }

    fn plugin_from_install(
        manifest: PluginManifest,
        path: PathBuf,
        enabled: bool,
        rollback_version: Option<String>,
        state: PluginLifecycleState,
    ) -> LoadedPlugin {
        let now = chrono::Utc::now().to_rfc3339();
        LoadedPlugin {
            manifest,
            path,
            enabled,
            config: serde_json::Value::Object(Default::default()),
            lifecycle: PluginLifecycle {
                state,
                installed_at: now.clone(),
                last_updated_at: now,
                rollback_version,
                last_error: None,
            },
        }
    }

    fn load_plugin_state(
        path: &Path,
        manifest: &PluginManifest,
    ) -> Result<PluginLifecycle, String> {
        let state_path = path.join(STATE_FILE);
        if !state_path.exists() {
            return Ok(Self::plugin_from_install(
                manifest.clone(),
                path.to_path_buf(),
                true,
                None,
                PluginLifecycleState::Enabled,
            )
            .lifecycle);
        }

        let content = std::fs::read_to_string(&state_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| format!("Invalid plugin state: {}", e))
    }

    fn persist_plugin_state(plugin: &LoadedPlugin) -> Result<(), String> {
        let state_path = plugin.path.join(STATE_FILE);
        let json = serde_json::to_string_pretty(&plugin.lifecycle).map_err(|e| e.to_string())?;
        std::fs::write(&state_path, json).map_err(|e| e.to_string())
    }

    fn backups_root(&self) -> PathBuf {
        self.plugins_dir.join(BACKUP_DIR)
    }

    fn backup_path(&self, name: &str, version: &str) -> PathBuf {
        self.backups_root().join(name).join(version)
    }

    fn latest_backup_path(&self, name: &str) -> Result<PathBuf, String> {
        let root = self.backups_root().join(name);
        let mut candidates = vec![];
        if root.exists() {
            for entry in std::fs::read_dir(&root)
                .map_err(|e| e.to_string())?
                .flatten()
            {
                let path = entry.path();
                if path.is_dir() {
                    let modified = entry
                        .metadata()
                        .and_then(|meta| meta.modified())
                        .map_err(|e| e.to_string())?;
                    candidates.push((modified, path));
                }
            }
        }

        candidates.sort_by(|a, b| b.0.cmp(&a.0));
        candidates
            .into_iter()
            .map(|(_, path)| path)
            .next()
            .ok_or_else(|| format!("No rollback backup found for plugin '{}'", name))
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_plugin_dir(root: &Path, name: &str, version: &str) -> PathBuf {
        let dir = root.join(format!("{}-{}", name, version.replace('.', "-")));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.py"), "print('ok')").unwrap();
        let manifest = serde_json::json!({
            "name": name,
            "version": version,
            "type": "action",
            "description": "Demo plugin",
            "author": "Codex",
            "permissions": ["read"],
            "entry_point": "main.py",
            "config_schema": null
        });
        std::fs::write(
            dir.join("plugin.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        dir
    }

    #[test]
    fn plugin_install_update_and_rollback_are_persisted() {
        let dir = tempdir().unwrap();
        let plugins_dir = dir.path().join("plugins");
        let mut manager = PluginManager::new(plugins_dir.clone());

        let v1 = create_plugin_dir(dir.path(), "demo", "1.0.0");
        let v2 = create_plugin_dir(dir.path(), "demo", "1.1.0");

        let install = manager.install(&v1).unwrap();
        assert_eq!(install.version, "1.0.0");
        assert_eq!(
            manager.get("demo").unwrap().lifecycle.state,
            PluginLifecycleState::Enabled
        );

        let update = manager.update("demo", &v2).unwrap();
        assert_eq!(update.version, "1.1.0");
        assert_eq!(
            manager.get("demo").unwrap().lifecycle.state,
            PluginLifecycleState::Updated
        );
        assert_eq!(
            manager
                .get("demo")
                .unwrap()
                .lifecycle
                .rollback_version
                .as_deref(),
            Some("1.0.0")
        );

        let rollback = manager.rollback("demo").unwrap();
        assert_eq!(rollback.version, "1.0.0");
        assert_eq!(
            manager.get("demo").unwrap().lifecycle.state,
            PluginLifecycleState::RolledBack
        );
    }

    #[test]
    fn plugin_disable_state_survives_rediscovery() {
        let dir = tempdir().unwrap();
        let plugins_dir = dir.path().join("plugins");
        let mut manager = PluginManager::new(plugins_dir.clone());
        let source = create_plugin_dir(dir.path(), "demo", "1.0.0");

        manager.install(&source).unwrap();
        manager.set_enabled("demo", false).unwrap();

        let mut reloaded = PluginManager::new(plugins_dir);
        reloaded.discover().unwrap();
        let plugin = reloaded.get("demo").unwrap();
        assert!(!plugin.enabled);
        assert_eq!(plugin.lifecycle.state, PluginLifecycleState::Disabled);
    }
}
