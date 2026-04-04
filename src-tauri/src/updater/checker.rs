use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub release_notes: Option<String>,
    pub download_url: Option<String>,
    pub asset_url: Option<String>,
    pub checked_at: String,
}

pub struct UpdateChecker;

impl UpdateChecker {
    /// Check GitHub releases for a newer version
    pub async fn check_for_update(current_version: &str, repo: &str) -> Result<UpdateInfo, String> {
        let client = reqwest::Client::new();
        let url = format!("https://api.github.com/repos/{}/releases/latest", repo);

        let response = client
            .get(&url)
            .header("User-Agent", "AgentOS-Updater")
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| format!("Update check failed: {}", e))?;

        if !response.status().is_success() {
            return Ok(UpdateInfo {
                current_version: current_version.to_string(),
                latest_version: None,
                update_available: false,
                release_notes: None,
                download_url: None,
                asset_url: None,
                checked_at: chrono::Utc::now().to_rfc3339(),
            });
        }

        let release: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        let tag = release
            .get("tag_name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim_start_matches('v');
        let notes = release
            .get("body")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let html_url = release
            .get("html_url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Find the installer asset URL from the release assets
        let asset_url = Self::extract_asset_url(&release);

        let update_available = version_is_newer(tag, current_version);

        Ok(UpdateInfo {
            current_version: current_version.to_string(),
            latest_version: Some(tag.to_string()),
            update_available,
            release_notes: notes,
            download_url: html_url,
            asset_url,
            checked_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Extract the correct installer asset URL from a GitHub release
    fn extract_asset_url(release: &serde_json::Value) -> Option<String> {
        let assets = release.get("assets").and_then(|a| a.as_array())?;
        // Prefer -setup.exe, then .msi, then any .exe
        let priorities = ["-setup.exe", ".msi", ".exe"];
        for suffix in &priorities {
            for asset in assets {
                let name = asset.get("name").and_then(|n| n.as_str()).unwrap_or("");
                if name.ends_with(suffix) {
                    if let Some(url) = asset.get("browser_download_url").and_then(|u| u.as_str()) {
                        return Some(url.to_string());
                    }
                }
            }
        }
        None
    }

    /// Find the correct asset URL from a GitHub release by repo
    pub async fn find_asset_url(repo: &str) -> Result<Option<String>, String> {
        let client = reqwest::Client::new();
        let url = format!("https://api.github.com/repos/{}/releases/latest", repo);
        let response = client
            .get(&url)
            .header("User-Agent", "AgentOS-Updater")
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await
            .map_err(|e| format!("Failed to fetch release: {}", e))?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let release: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        Ok(Self::extract_asset_url(&release))
    }

    /// Download the latest release asset to the given directory
    pub async fn download_update(download_url: &str, dest_path: &Path) -> Result<PathBuf, String> {
        info!(url = %download_url, "Downloading update installer...");
        let client = reqwest::Client::new();
        let response = client
            .get(download_url)
            .header("User-Agent", "AgentOS-Updater")
            .timeout(std::time::Duration::from_secs(300))
            .send()
            .await
            .map_err(|e| format!("Download failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Download returned HTTP {}",
                response.status().as_u16()
            ));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("Failed to read download body: {}", e))?;

        // Determine filename from URL or use default
        let filename = download_url
            .rsplit('/')
            .next()
            .unwrap_or("AgentOS-update.exe");
        let file_path = dest_path.join(filename);

        std::fs::create_dir_all(dest_path)
            .map_err(|e| format!("Failed to create download directory: {}", e))?;
        std::fs::write(&file_path, &bytes)
            .map_err(|e| format!("Failed to write installer: {}", e))?;

        info!(path = %file_path.display(), bytes = bytes.len(), "Update downloaded successfully");
        Ok(file_path)
    }

    /// Launch the downloaded installer and optionally exit the current process
    pub fn install_update(installer_path: &Path) -> Result<(), String> {
        if !installer_path.exists() {
            return Err(format!("Installer not found: {}", installer_path.display()));
        }

        info!(path = %installer_path.display(), "Launching update installer...");

        #[cfg(target_os = "windows")]
        {
            std::process::Command::new(installer_path)
                .arg("/SILENT")
                .spawn()
                .map_err(|e| format!("Failed to launch installer: {}", e))?;
        }

        #[cfg(not(target_os = "windows"))]
        {
            warn!("Auto-install is only supported on Windows. Please run the installer manually.");
            return Err(format!(
                "Please run the installer manually: {}",
                installer_path.display()
            ));
        }

        Ok(())
    }
}

/// Compare semver strings: returns true if `latest` > `current`
pub fn version_is_newer(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> { v.split('.').filter_map(|p| p.parse().ok()).collect() };
    let l = parse(latest);
    let c = parse(current);
    l > c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(version_is_newer("4.3.0", "4.2.0"));
        assert!(version_is_newer("5.0.0", "4.2.0"));
        assert!(version_is_newer("4.2.1", "4.2.0"));
        assert!(!version_is_newer("4.2.0", "4.2.0"));
        assert!(!version_is_newer("4.1.0", "4.2.0"));
        assert!(!version_is_newer("3.9.9", "4.2.0"));
    }
}
